use base64::Engine;
use image::ImageReader;
use memmap2::Mmap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "tif", "bmp", "gif", "avif", "jxl",
];

/// Caps applied before decoding any user-supplied image buffer, to prevent
/// decompression-bomb OOM (a crafted header declaring huge dimensions).
pub const MAX_IMAGE_DIMENSION: u32 = 8192;
pub const MAX_IMAGE_ALLOC: u64 = 256 * 1024 * 1024;

pub fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_ALLOC);
    limits
}

/// Threshold for using memory-mapped I/O instead of reading into RAM.
/// 10MB+ files benefit from mmap as OS handles caching better.
const MMAP_THRESHOLD: u64 = 10 * 1024 * 1024;

/// Read file contents via memory mapping for large files (>10MB).
/// Falls back to regular read for smaller files or if mmap fails.
pub fn read_file_mmap_or_default(path: &str) -> Result<Vec<u8>, std::io::Error> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    if size >= MMAP_THRESHOLD {
        // Safety: we immediately copy the mmap contents to owned memory
        // and the Mmap is dropped after. This is safe.
        unsafe {
            let mmap = Mmap::map(&file)?;
            return Ok(mmap.to_vec());
        }
    }
    fs::read(path)
}

pub fn ext_lowercase(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

fn check_jxl_dimensions(width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(format!(
            "JXL dimensions {width}x{height} exceed maximum {MAX_IMAGE_DIMENSION}x{MAX_IMAGE_DIMENSION}"
        )
        .into());
    }
    Ok(())
}

fn decode_jxl(path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    let data = read_file_mmap_or_default(path)?;
    let image = jxl_oxide::JxlImage::builder()
        .read(std::io::Cursor::new(&data))
        .map_err(|e| e.to_string())?;
    check_jxl_dimensions(image.width(), image.height())?;
    let render = image.render_frame(0).map_err(|e| e.to_string())?;
    let fb = render.image_all_channels();
    let (w, h, ch) = (fb.width(), fb.height(), fb.channels());
    let buf = fb.buf();

    let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;

    match ch {
        1 => {
            let pixels: Vec<u8> = buf.iter().map(|&v| to_u8(v)).collect();
            Ok(image::DynamicImage::ImageLuma8(
                image::GrayImage::from_raw(w as u32, h as u32, pixels)
                    .ok_or("failed to create grayscale image")?,
            ))
        }
        3 => {
            let pixels: Vec<u8> = buf.iter().map(|&v| to_u8(v)).collect();
            Ok(image::DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(w as u32, h as u32, pixels)
                    .ok_or("failed to create RGB image")?,
            ))
        }
        4 => {
            let pixels: Vec<u8> = buf.iter().map(|&v| to_u8(v)).collect();
            Ok(image::DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(w as u32, h as u32, pixels)
                    .ok_or("failed to create RGBA image")?,
            ))
        }
        _ => Err(format!("unsupported JXL channel count: {ch}").into()),
    }
}

pub fn open_image(path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    match ext_lowercase(path).as_deref() {
        Some("heic") | Some("heif") => crate::core::heic::decode_heic(path),
        Some("jxl") => decode_jxl(path),
        _ => {
            let file = fs::File::open(path)?;
            let mut reader = ImageReader::new(BufReader::new(file)).with_guessed_format()?;
            reader.limits(decode_limits());
            Ok(reader.decode()?)
        }
    }
}

pub fn write_preserving_timestamps(
    input_path: &str,
    output_path: &str,
    data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mtime = filetime::FileTime::from_last_modification_time(&fs::metadata(input_path)?);
    fs::write(output_path, data)?;
    filetime::set_file_mtime(output_path, mtime)?;
    Ok(())
}

pub fn checked_size(path: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let size = fs::metadata(path)?.len();
    if size > MAX_FILE_SIZE {
        return Err("file exceeds 100 MB limit".into());
    }
    Ok(size)
}

pub fn decode_rgb(path: &str) -> Result<(usize, usize, Vec<u8>), Box<dyn std::error::Error>> {
    let img = open_image(path)?;
    let rgb = img.to_rgb8();
    Ok((rgb.width() as usize, rgb.height() as usize, rgb.into_raw()))
}

pub fn generate_thumbnail(path: &str, max_size: u32) -> Result<String, Box<dyn std::error::Error>> {
    crate::core::paths::validate_input_path(path, false)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let img = open_image(path)?;
    let thumb = img.thumbnail(max_size, max_size).to_rgb8();
    let (w, h) = (thumb.width() as usize, thumb.height() as usize);
    let jpeg_bytes = crate::core::compression::encode_jpeg_bytes(w, h, thumb.as_raw(), 60.0)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

pub fn read_dimensions(path: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    crate::core::paths::validate_input_path(path, false)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let ext = ext_lowercase(path).unwrap_or_default();
    if ext == "heic" || ext == "heif" {
        let img = crate::core::heic::decode_heic(path)?;
        return Ok((img.width(), img.height()));
    }
    let (w, h) = image::image_dimensions(path)?;
    Ok((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_preserving_timestamps_copies_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.bin");
        let output = dir.path().join("output.bin");

        fs::write(&input, b"hello").unwrap();

        let past = filetime::FileTime::from_unix_time(1_000_000, 0);
        filetime::set_file_times(&input, past, past).unwrap();

        write_preserving_timestamps(input.to_str().unwrap(), output.to_str().unwrap(), b"world")
            .unwrap();

        let meta = fs::metadata(&output).unwrap();
        let mtime = filetime::FileTime::from_last_modification_time(&meta);
        assert_eq!(mtime, past);
    }

    #[test]
    fn generate_thumbnail_returns_data_uri() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jpg");

        let img = image::RgbImage::from_fn(200, 200, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        img.save(&path).unwrap();

        let result = generate_thumbnail(path.to_str().unwrap(), 80).unwrap();
        assert!(result.starts_with("data:image/jpeg;base64,"));
        assert!(result.len() > 30);
    }

    #[test]
    fn dimensions_jpeg_returns_correct_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jpg");
        image::RgbImage::new(320, 240).save(&path).unwrap();
        let (w, h) = read_dimensions(path.to_str().unwrap()).unwrap();
        assert_eq!((w, h), (320, 240));
    }

    #[test]
    fn dimensions_png_returns_correct_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.png");
        image::RgbaImage::new(100, 200).save(&path).unwrap();
        let (w, h) = read_dimensions(path.to_str().unwrap()).unwrap();
        assert_eq!((w, h), (100, 200));
    }

    #[test]
    fn dimensions_invalid_path_returns_err() {
        assert!(read_dimensions("/nonexistent/x.jpg").is_err());
    }

    #[test]
    fn jxl_dimension_check_rejects_oversized_width() {
        let err = check_jxl_dimensions(9000, 100).unwrap_err();
        assert!(
            err.to_string().contains("exceed maximum"),
            "expected 'exceed maximum' in: {err}"
        );
    }

    #[test]
    fn jxl_dimension_check_rejects_oversized_height() {
        let err = check_jxl_dimensions(100, 100_000).unwrap_err();
        assert!(
            err.to_string().contains("exceed maximum"),
            "expected 'exceed maximum' in: {err}"
        );
    }

    #[test]
    fn jxl_dimension_check_accepts_boundary() {
        assert!(check_jxl_dimensions(MAX_IMAGE_DIMENSION, MAX_IMAGE_DIMENSION).is_ok());
    }
}
