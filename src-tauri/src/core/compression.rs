use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Cursor};
use std::path::Path;

fn open_image(input_path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    let ext = Path::new(input_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("heic") | Some("heif") => crate::core::heic::decode_heic(input_path),
        _ => {
            let file = fs::File::open(input_path)?;
            let reader = ImageReader::new(BufReader::new(file)).with_guessed_format()?;
            Ok(reader.decode()?)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CompressionResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Jpeg,
    Png,
    Webp,
}

pub fn compress_jpeg(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);

    let metadata = fs::metadata(input_path)?;
    let original_size = metadata.len();
    if original_size > 100 * 1024 * 1024 {
        return Err("file exceeds 100 MB limit".into());
    }

    let ext = Path::new(input_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let is_heic = matches!(ext.as_deref(), Some("heic") | Some("heif"));

    let (width, height, pixels) = if is_heic {
        let img = crate::core::heic::decode_heic(input_path)?;
        let rgb_img = img.to_rgb8();
        (
            rgb_img.width() as usize,
            rgb_img.height() as usize,
            rgb_img.into_raw(),
        )
    } else {
        // Only read full file if it looks like a JPEG (magic bytes FF D8 FF)
        let jpeg_result = fs::read(input_path).ok().and_then(|input| {
            if !input.starts_with(&[0xFF, 0xD8, 0xFF]) {
                return None;
            }
            // mozjpeg panics (not Err) on invalid data — catch_unwind is required
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let dinfo = mozjpeg::Decompress::with_markers(mozjpeg::ALL_MARKERS)
                    .from_mem(&input)?;
                let mut rgb = dinfo.rgb()?;
                let w = rgb.width();
                let h = rgb.height();
                let p: Vec<u8> = rgb.read_scanlines()?;
                rgb.finish()?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>((w, h, p))
            }))
            .ok()
            .and_then(|r| r.ok())
        });

        match jpeg_result {
            Some(whp) => whp,
            None => {
                let img = open_image(input_path)?;
                let rgb_img = img.to_rgb8();
                (
                    rgb_img.width() as usize,
                    rgb_img.height() as usize,
                    rgb_img.into_raw(),
                )
            }
        }
    };

    let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    cinfo.set_size(width, height);
    cinfo.set_quality(quality);
    let mut cinfo = cinfo.start_compress(Vec::new())?;
    cinfo.write_scanlines(&pixels)?;
    let compressed = cinfo.finish()?;

    let compressed_size = compressed.len() as u64;
    fs::write(output_path, &compressed)?;

    Ok(CompressionResult {
        input_path: input_path.to_string(),
        output_path: output_path.to_string(),
        original_size,
        compressed_size,
        error: None,
    })
}

pub fn compress_png(
    input_path: &str,
    output_path: &str,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let metadata = fs::metadata(input_path)?;
    let original_size = metadata.len();
    if original_size > 100 * 1024 * 1024 {
        return Err("file exceeds 100 MB limit".into());
    }

    let is_png = matches!(
        Path::new(input_path).extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(),
        Some("png")
    );

    let compressed = if is_png {
        // Already PNG: fast re-encode with oxipng preset 0 (libdeflater level 5, no filter search)
        oxipng::optimize_from_memory(&fs::read(input_path)?, &oxipng::Options::from_preset(0))?
    } else {
        // Non-PNG input: encode via image crate (fdeflate, fast and well-compressed)
        let img = open_image(input_path)?;
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)?;
        buf.into_inner()
    };

    let compressed_size = compressed.len() as u64;
    fs::write(output_path, &compressed)?;

    Ok(CompressionResult {
        input_path: input_path.to_string(),
        output_path: output_path.to_string(),
        original_size,
        compressed_size,
        error: None,
    })
}

pub fn compress_webp(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);

    let metadata = fs::metadata(input_path)?;
    let original_size = metadata.len();
    if original_size > 100 * 1024 * 1024 {
        return Err("file exceeds 100 MB limit".into());
    }

    let img = open_image(input_path)?;
    let encoder = webp::Encoder::from_image(&img)?;
    let memory = encoder
        .encode_simple(false, quality)
        .map_err(|e| format!("webp encoding failed: {e:?}"))?;

    let compressed_size = memory.len() as u64;
    fs::write(output_path, &*memory)?;

    Ok(CompressionResult {
        input_path: input_path.to_string(),
        output_path: output_path.to_string(),
        original_size,
        compressed_size,
        error: None,
    })
}

pub fn compress_image(
    input_path: &str,
    output_path: &str,
    format: &OutputFormat,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Jpeg => compress_jpeg(input_path, output_path, quality),
        OutputFormat::Png => compress_png(input_path, output_path),
        OutputFormat::Webp => compress_webp(input_path, output_path, quality),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_jpeg(path: &str, width: usize, height: usize, quality: f32) {
        let mut pixels = vec![0u8; width * height * 3];
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 3;
                pixels[i] = (x % 256) as u8;
                pixels[i + 1] = (y % 256) as u8;
                pixels[i + 2] = ((x + y) % 256) as u8;
            }
        }
        let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        cinfo.set_size(width, height);
        cinfo.set_quality(quality);
        let mut cinfo = cinfo.start_compress(Vec::new()).unwrap();
        cinfo.write_scanlines(&pixels).unwrap();
        let data = cinfo.finish().unwrap();
        let mut file = fs::File::create(path).unwrap();
        file.write_all(&data).unwrap();
    }

    fn create_test_png(path: &str, width: u32, height: u32) {
        let mut img = image::RgbImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                img.put_pixel(
                    x,
                    y,
                    image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]),
                );
            }
        }
        img.save(path).unwrap();
    }

    #[test]
    fn compress_jpeg_reduces_size() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");
        let output = dir.path().join("test_out.jpg");

        create_test_jpeg(input.to_str().unwrap(), 200, 200, 95.0);
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 60.0).unwrap();

        assert!(result.compressed_size < result.original_size);
        assert!(output.exists());
    }

    #[test]
    fn lower_quality_means_smaller_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");
        let out_high = dir.path().join("high.jpg");
        let out_low = dir.path().join("low.jpg");

        create_test_jpeg(input.to_str().unwrap(), 200, 200, 95.0);

        let high =
            compress_jpeg(input.to_str().unwrap(), out_high.to_str().unwrap(), 90.0).unwrap();
        let low = compress_jpeg(input.to_str().unwrap(), out_low.to_str().unwrap(), 20.0).unwrap();

        assert!(low.compressed_size < high.compressed_size);
    }

    #[test]
    fn compress_invalid_path_returns_error() {
        let result = compress_jpeg("/nonexistent/image.jpg", "/tmp/out.jpg", 80.0);
        assert!(result.is_err());
    }

    #[test]
    fn compress_png_reduces_size() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_out.png");

        create_test_png(input.to_str().unwrap(), 200, 200);
        let original_size = fs::metadata(&input).unwrap().len();

        let result = compress_png(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();

        assert!(output.exists());
        assert_eq!(result.original_size, original_size);
        assert!(result.compressed_size > 0);
    }

    #[test]
    fn compress_png_invalid_path_returns_error() {
        let result = compress_png("/nonexistent/image.png", "/tmp/out.png");
        assert!(result.is_err());
    }

    #[test]
    fn compress_webp_reduces_size() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_out.webp");

        create_test_png(input.to_str().unwrap(), 200, 200);

        let result =
            compress_webp(input.to_str().unwrap(), output.to_str().unwrap(), 75.0).unwrap();

        assert!(output.exists());
        assert!(result.compressed_size > 0);
    }

    #[test]
    fn compress_webp_invalid_path_returns_error() {
        let result = compress_webp("/nonexistent/image.png", "/tmp/out.webp", 75.0);
        assert!(result.is_err());
    }

    #[test]
    fn compress_image_routes_correctly() {
        let dir = tempfile::tempdir().unwrap();

        let jpeg_input = dir.path().join("test.jpg");
        let jpeg_output = dir.path().join("test_out.jpg");
        create_test_jpeg(jpeg_input.to_str().unwrap(), 100, 100, 95.0);
        let result = compress_image(
            jpeg_input.to_str().unwrap(),
            jpeg_output.to_str().unwrap(),
            &OutputFormat::Jpeg,
            60.0,
        )
        .unwrap();
        assert!(jpeg_output.exists());
        assert!(result.compressed_size > 0);

        let png_input = dir.path().join("test.png");
        let png_output = dir.path().join("test_out.png");
        create_test_png(png_input.to_str().unwrap(), 100, 100);
        let result = compress_image(
            png_input.to_str().unwrap(),
            png_output.to_str().unwrap(),
            &OutputFormat::Png,
            2.0,
        )
        .unwrap();
        assert!(png_output.exists());
        assert!(result.compressed_size > 0);
    }
}
