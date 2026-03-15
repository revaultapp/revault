use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::core::image_io::{checked_size, decode_rgb, ext_lowercase, open_image};

#[derive(Serialize)]
pub struct CompressionResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub error: Option<String>,
}

impl CompressionResult {
    pub fn ok(input: &str, output: &str, original: u64, compressed: u64) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: output.to_string(),
            original_size: original,
            compressed_size: compressed,
            error: None,
        }
    }

    pub fn err(input: &str, msg: String) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: String::new(),
            original_size: fs::metadata(input).map(|m| m.len()).unwrap_or(0),
            compressed_size: 0,
            error: Some(msg),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Jpeg,
    Png,
    Webp,
}

pub fn detect_format(path: &str) -> OutputFormat {
    match ext_lowercase(path).as_deref() {
        Some("png") => OutputFormat::Png,
        Some("webp") => OutputFormat::Webp,
        _ => OutputFormat::Jpeg,
    }
}

pub fn compress_jpeg(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);
    let original_size = checked_size(input_path)?;

    // HEIC: skip file read + magic byte check, go straight to native decoder
    let is_heic = matches!(
        ext_lowercase(input_path).as_deref(),
        Some("heic") | Some("heif")
    );

    let (width, height, pixels) = if is_heic {
        decode_rgb(input_path)?
    } else {
        // Try mozjpeg direct decode (better quality) — falls back to image crate
        let jpeg_result = fs::read(input_path).ok().and_then(|input| {
            if !input.starts_with(&[0xFF, 0xD8, 0xFF]) {
                return None;
            }
            // mozjpeg panics (not Err) on invalid data — catch_unwind is required
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let dinfo =
                    mozjpeg::Decompress::with_markers(mozjpeg::ALL_MARKERS).from_mem(&input)?;
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

        jpeg_result.map_or_else(|| decode_rgb(input_path), Ok)?
    };

    let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    cinfo.set_size(width, height);
    cinfo.set_quality(quality);
    let mut cinfo = cinfo.start_compress(Vec::new())?;
    cinfo.write_scanlines(&pixels)?;
    let compressed = cinfo.finish()?;

    let compressed_size = compressed.len() as u64;
    fs::write(output_path, &compressed)?;

    Ok(CompressionResult::ok(
        input_path,
        output_path,
        original_size,
        compressed_size,
    ))
}

pub fn compress_png(
    input_path: &str,
    output_path: &str,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let original_size = checked_size(input_path)?;

    let compressed = if matches!(ext_lowercase(input_path).as_deref(), Some("png")) {
        // Already PNG: fast re-encode with oxipng preset 0
        oxipng::optimize_from_memory(&fs::read(input_path)?, &oxipng::Options::from_preset(0))?
    } else {
        let img = open_image(input_path)?;
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)?;
        oxipng::optimize_from_memory(&buf.into_inner(), &oxipng::Options::from_preset(0))?
    };

    let compressed_size = compressed.len() as u64;
    fs::write(output_path, &compressed)?;

    Ok(CompressionResult::ok(
        input_path,
        output_path,
        original_size,
        compressed_size,
    ))
}

pub fn compress_webp(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);
    let original_size = checked_size(input_path)?;

    let img = open_image(input_path)?;
    let encoder = webp::Encoder::from_image(&img)?;

    let mut config = webp::WebPConfig::new().map_err(|_| "failed to create WebP config")?;
    config.quality = quality;
    config.method = 0; // fastest encoding (default 4 is ~3x slower)

    let memory = encoder
        .encode_advanced(&config)
        .map_err(|e| format!("webp encoding failed: {e:?}"))?;

    let compressed_size = memory.len() as u64;
    fs::write(output_path, &*memory)?;

    Ok(CompressionResult::ok(
        input_path,
        output_path,
        original_size,
        compressed_size,
    ))
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

pub fn compress_batch(
    paths: &[String],
    quality: f32,
    format: Option<OutputFormat>,
    output_dir: Option<&str>,
    suffix: &str,
) -> Vec<CompressionResult> {
    paths
        .iter()
        .map(|path| {
            let input = Path::new(path.as_str());
            let stem = match input.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => return CompressionResult::err(path, format!("invalid filename: {path}")),
            };
            let parent = match input.parent() {
                Some(p) => p,
                None => return CompressionResult::err(path, format!("invalid path: {path}")),
            };

            let fmt = format.clone().unwrap_or_else(|| detect_format(path));
            let ext = match fmt {
                OutputFormat::Jpeg => "jpg",
                OutputFormat::Png => "png",
                OutputFormat::Webp => "webp",
            };
            let out_base = output_dir.map(Path::new).unwrap_or(parent);
            let output = out_base.join(format!("{stem}{suffix}.{ext}"));

            match compress_image(path, &output.to_string_lossy(), &fmt, quality) {
                Ok(r) => r,
                Err(e) => CompressionResult::err(path, e.to_string()),
            }
        })
        .collect()
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
    fn compress_batch_handles_mixed_results() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");
        create_test_jpeg(input.to_str().unwrap(), 100, 100, 95.0);

        let paths = vec![
            input.to_string_lossy().to_string(),
            "/nonexistent/fake.jpg".to_string(),
        ];
        let results = compress_batch(&paths, 60.0, None, None, "_compressed");

        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[0].compressed_size > 0);
        assert!(results[1].error.is_some());
    }

    #[test]
    fn compress_png_from_jpeg_input_applies_oxipng() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        let output = dir.path().join("photo_out.png");

        create_test_jpeg(input.to_str().unwrap(), 100, 100, 90.0);
        let result = compress_png(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();

        assert!(result.compressed_size > 0);
        let header = &fs::read(&output).unwrap()[..4];
        assert_eq!(header, [0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn detect_format_from_extension() {
        assert!(matches!(detect_format("photo.jpg"), OutputFormat::Jpeg));
        assert!(matches!(detect_format("photo.png"), OutputFormat::Png));
        assert!(matches!(detect_format("photo.webp"), OutputFormat::Webp));
        assert!(matches!(detect_format("photo.heic"), OutputFormat::Jpeg));
        assert!(matches!(detect_format("photo.unknown"), OutputFormat::Jpeg));
    }
}
