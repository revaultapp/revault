#[cfg(target_os = "macos")]
mod platform {
    use core_foundation::base::TCFType;
    use core_foundation::url::CFURL;
    use core_graphics::image::CGImage;
    use foreign_types::ForeignType;
    use std::ffi::c_void;
    use std::ptr;

    #[link(name = "ImageIO", kind = "framework")]
    extern "C" {
        fn CGImageSourceCreateWithURL(url: *const c_void, options: *const c_void) -> *const c_void;
        fn CGImageSourceCreateImageAtIndex(
            source: *const c_void,
            index: usize,
            options: *const c_void,
        ) -> *mut c_void;
    }

    pub fn decode(input_path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
        let path = std::path::Path::new(input_path);
        if !path.exists() {
            return Err(format!("file not found: {input_path}").into());
        }

        let url = CFURL::from_path(path, false)
            .ok_or_else(|| format!("failed to create CFURL for: {input_path}"))?;

        // SAFETY: CGImageSourceCreateWithURL and CGImageSourceCreateImageAtIndex are
        // ImageIO C APIs. `url` is borrowed via as_concrete_TypeRef() and stays alive on
        // the stack for the entire block. Null returns are checked before use. CFRelease
        // is called once on `source`. CGImage::from_ptr takes ownership of `image_ref`
        // and will call CGImageRelease on drop.
        let cg_image = unsafe {
            let source = CGImageSourceCreateWithURL(url.as_concrete_TypeRef() as _, ptr::null());
            if source.is_null() {
                return Err(format!("failed to create image source for: {input_path}").into());
            }
            let image_ref = CGImageSourceCreateImageAtIndex(source, 0, ptr::null());
            core_foundation::base::CFRelease(source);
            if image_ref.is_null() {
                return Err(format!("failed to decode image: {input_path}").into());
            }
            CGImage::from_ptr(image_ref as _)
        };

        let width = cg_image.width() as u32;
        let height = cg_image.height() as u32;
        let bytes_per_row = cg_image.bytes_per_row();
        let bits_per_pixel = cg_image.bits_per_pixel();
        let bytes_per_pixel = bits_per_pixel / 8;

        crate::core::heic::check_dimensions(width, height)?;

        if width == 0 || height == 0 {
            return Err("image has zero width or height".into());
        }
        if bytes_per_pixel == 0 {
            return Err(
                format!("unsupported pixel format: {bits_per_pixel} bits per pixel").into(),
            );
        }

        let data = cg_image.data();
        let raw = data.bytes();

        let expected_len = (height as usize - 1) * bytes_per_row + width as usize * bytes_per_pixel;
        if raw.len() < expected_len {
            return Err("image data buffer too small for declared dimensions".into());
        }

        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height as usize {
            let row_start = y * bytes_per_row;
            let row = &raw[row_start..row_start + width as usize * bytes_per_pixel];
            if bytes_per_pixel == 4 {
                for pixel in row.chunks_exact(4) {
                    rgb.extend_from_slice(&[pixel[0], pixel[1], pixel[2]]);
                }
            } else if bytes_per_pixel == 3 {
                rgb.extend_from_slice(row);
            } else {
                return Err(format!("unsupported pixel format: {bits_per_pixel} bpp").into());
            }
        }

        image::RgbImage::from_raw(width, height, rgb)
            .map(image::DynamicImage::ImageRgb8)
            .ok_or_else(|| "failed to construct image from decoded pixels".into())
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CO_E_ALREADYINITIALIZED, GENERIC_READ};
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::*;

    struct ComGuard(bool);
    impl Drop for ComGuard {
        fn drop(&mut self) {
            // SAFETY: CoUninitialize balances CoInitializeEx only when we own the init
            // (self.0 == true). Skipped for CO_E_ALREADYINITIALIZED to avoid unbalanced
            // uninit. Dropped on the same thread that called CoInitializeEx.
            if self.0 {
                unsafe { CoUninitialize() }
            }
        }
    }

    pub fn decode(input_path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
        let path = std::path::Path::new(input_path);
        if !path.exists() {
            return Err(format!("file not found: {input_path}").into());
        }

        // SAFETY: All COM pointers (factory, decoder, frame, converter) are managed
        // by the `windows` crate's ComInterface wrappers which handle Release on drop.
        // ComGuard ensures CoUninitialize is called only when we own the init.
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_err() && hr != CO_E_ALREADYINITIALIZED {
                return Err(format!("COM initialization failed: {hr:?}").into());
            }
            let _com = ComGuard(hr.is_ok());

            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

            let wide_path: Vec<u16> = input_path
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let decoder = factory
                .CreateDecoderFromFilename(
                    PCWSTR(wide_path.as_ptr()),
                    None,
                    GENERIC_READ,
                    WICDecodeMetadataCacheOnDemand,
                )
                .map_err(|e| {
                    if e.code().0 as u32 == 0xC00D5212 {
                        "HEVC codec not installed. Install 'HEVC Video Extensions' from Microsoft Store: ms-windows-store://pdp/?ProductId=9nmzlz57r3t7".to_string()
                    } else {
                        format!("failed to decode HEIC: {e}")
                    }
                })?;

            let frame = decoder.GetFrame(0)?;

            let converted: IWICFormatConverter = factory.CreateFormatConverter()?;
            converted.Initialize(
                &frame,
                &GUID_WICPixelFormat24bppRGB,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )?;

            let (mut width, mut height) = (0u32, 0u32);
            converted.GetSize(&mut width, &mut height)?;

            crate::core::heic::check_dimensions(width, height)?;

            let stride = width * 3;
            let buf_size = (stride * height) as usize;
            let mut pixels = vec![0u8; buf_size];
            converted.CopyPixels(std::ptr::null(), stride, &mut pixels)?;

            image::RgbImage::from_raw(width, height, pixels)
                .map(image::DynamicImage::ImageRgb8)
                .ok_or_else(|| "failed to construct image from decoded pixels".into())
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    pub fn decode(_input_path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
        Err("HEIC decoding is not supported on this platform".into())
    }
}

pub fn decode_heic(input_path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    platform::decode(input_path)
}

pub(crate) const HEIC_MAX_DIMENSION: u32 = 8192;

pub(crate) fn check_dimensions(width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
    if width > HEIC_MAX_DIMENSION || height > HEIC_MAX_DIMENSION {
        return Err(format!(
            "HEIC dimensions {}x{} exceed maximum {}x{}",
            width, height, HEIC_MAX_DIMENSION, HEIC_MAX_DIMENSION
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_check_rejects_oversized_width() {
        let err = check_dimensions(9000, 100).unwrap_err();
        assert!(
            err.to_string().contains("exceed maximum"),
            "expected 'exceed maximum' in: {err}"
        );
    }

    #[test]
    fn dimension_check_rejects_oversized_height() {
        let err = check_dimensions(100, 100_000).unwrap_err();
        assert!(
            err.to_string().contains("exceed maximum"),
            "expected 'exceed maximum' in: {err}"
        );
    }

    #[test]
    fn dimension_check_accepts_boundary() {
        assert!(check_dimensions(8192, 8192).is_ok());
        assert!(check_dimensions(1, 1).is_ok());
    }
}
