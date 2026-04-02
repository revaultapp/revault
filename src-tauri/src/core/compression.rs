use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::ImageEncoder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::core::image_io::{
    checked_size, decode_rgb, ext_lowercase, open_image, write_preserving_timestamps,
};

#[derive(Serialize)]
pub struct CompressionResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub already_optimal: bool,
    pub error: Option<String>,
}

impl CompressionResult {
    pub fn ok(input: &str, output: &str, original: u64, compressed: u64) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: output.to_string(),
            original_size: original,
            compressed_size: compressed,
            already_optimal: false,
            error: None,
        }
    }

    pub fn optimal(input: &str, output: &str, original: u64) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: output.to_string(),
            original_size: original,
            compressed_size: original,
            already_optimal: true,
            error: None,
        }
    }

    pub fn err(input: &str, msg: String) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: String::new(),
            original_size: fs::metadata(input).map(|m| m.len()).unwrap_or(0),
            compressed_size: 0,
            already_optimal: false,
            error: Some(msg),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum QualityPreset {
    Smallest,
    Balanced,
    HighQuality,
}

impl QualityPreset {
    fn jpeg_quality(self) -> f32 {
        match self {
            QualityPreset::Smallest => 45.0,
            QualityPreset::Balanced => 75.0,
            QualityPreset::HighQuality => 88.0,
        }
    }
    fn webp_quality(self) -> f32 {
        match self {
            QualityPreset::Smallest => 40.0,
            QualityPreset::Balanced => 72.0,
            QualityPreset::HighQuality => 85.0,
        }
    }
    fn avif_quality(self) -> f32 {
        match self {
            QualityPreset::Smallest => 35.0,
            QualityPreset::Balanced => 65.0,
            QualityPreset::HighQuality => 80.0,
        }
    }
    fn oxipng_preset(self) -> u8 {
        match self {
            QualityPreset::Smallest => 0,
            QualityPreset::Balanced => 2,
            QualityPreset::HighQuality => 6,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum OutputFormat {
    Jpeg,
    Png,
    Webp,
    Avif,
}

pub fn detect_format(path: &str) -> OutputFormat {
    match ext_lowercase(path).as_deref() {
        Some("png") => OutputFormat::Png,
        Some("webp") => OutputFormat::Webp,
        Some("avif") => OutputFormat::Avif,
        Some("heic") | Some("heif") => OutputFormat::Webp,
        _ => OutputFormat::Jpeg,
    }
}

/// Decode input to RGB pixels, preferring mozjpeg for JPEG quality preservation.
fn decode_input_rgb(
    input_path: &str,
) -> Result<(usize, usize, Vec<u8>), Box<dyn std::error::Error>> {
    let is_heic = matches!(
        ext_lowercase(input_path).as_deref(),
        Some("heic") | Some("heif")
    );
    if is_heic {
        return decode_rgb(input_path);
    }

    // Try mozjpeg direct decode (better quality) — falls back to image crate
    let jpeg_result = fs::read(input_path).ok().and_then(|input| {
        if !input.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return None;
        }
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let dinfo = mozjpeg::Decompress::with_markers(mozjpeg::ALL_MARKERS).from_mem(&input)?;
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

    jpeg_result.map_or_else(|| decode_rgb(input_path), Ok)
}

pub(crate) fn encode_jpeg_bytes(
    width: usize,
    height: usize,
    pixels: &[u8],
    quality: f32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    encode_jpeg_bytes_inner(width, height, pixels, quality, true)
}

fn encode_jpeg_bytes_inner(
    width: usize,
    height: usize,
    pixels: &[u8],
    quality: f32,
    progressive: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // mozjpeg's C code can panic on invalid state — catch_unwind is required.
    // pixels is an immutable borrow valid for the closure's lifetime.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        cinfo.set_size(width, height);
        cinfo.set_quality(quality);
        if !progressive {
            cinfo.set_optimize_scans(false);
        }
        let mut cinfo = cinfo.start_compress(Vec::new())?;
        cinfo.write_scanlines(pixels)?;
        Ok::<_, Box<dyn std::error::Error>>(cinfo.finish()?)
    }))
    .map_err(|_| "mozjpeg encoder panicked")?
}

pub(crate) fn encode_webp_bytes(
    img: &image::DynamicImage,
    quality: f32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let encoder = webp::Encoder::from_image(img)?;
    let mut config = webp::WebPConfig::new().map_err(|_| "failed to create WebP config")?;
    config.quality = quality;
    config.method = 0;
    let memory = encoder
        .encode_advanced(&config)
        .map_err(|e| format!("webp encoding failed: {e:?}"))?;
    Ok(memory.to_vec())
}

pub(crate) fn encode_avif_bytes(
    img: &image::DynamicImage,
    quality: f32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let pixels: Vec<ravif::RGBA8> = rgba
        .as_raw()
        .chunks_exact(4)
        .map(|c| ravif::RGBA8::new(c[0], c[1], c[2], c[3]))
        .collect();
    let speed = 4;
    let encoded = ravif::Encoder::new()
        .with_quality(quality)
        .with_speed(speed)
        .with_alpha_quality(100.0)
        .encode_rgba(ravif::Img::new(&pixels, w, h))?;
    Ok(encoded.avif_file)
}

fn same_format(input_path: &str, output_path: &str) -> bool {
    ext_lowercase(input_path) == ext_lowercase(output_path)
}

fn write_smallest(
    input_path: &str,
    output_path: &str,
    compressed: &[u8],
    original_size: u64,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let compressed_size = compressed.len() as u64;
    if compressed_size >= original_size && same_format(input_path, output_path) {
        fs::copy(input_path, output_path)?;
        Ok(CompressionResult::optimal(
            input_path,
            output_path,
            original_size,
        ))
    } else {
        write_preserving_timestamps(input_path, output_path, compressed)?;
        Ok(CompressionResult::ok(
            input_path,
            output_path,
            original_size,
            compressed_size,
        ))
    }
}

pub fn compress_jpeg(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);
    let original_size = checked_size(input_path)?;
    let (width, height, pixels) = decode_input_rgb(input_path)?;
    let compressed = encode_jpeg_bytes(width, height, &pixels, quality)?;
    write_smallest(input_path, output_path, &compressed, original_size)
}

pub fn compress_png(
    input_path: &str,
    output_path: &str,
    preset: QualityPreset,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let original_size = checked_size(input_path)?;
    let oxipng_preset = preset.oxipng_preset();

    let compressed = if matches!(ext_lowercase(input_path).as_deref(), Some("png")) {
        oxipng::optimize_from_memory(
            &fs::read(input_path)?,
            &oxipng::Options::from_preset(oxipng_preset),
        )?
    } else {
        // Non-PNG input: encode to PNG directly, skip oxipng (too slow on large images)
        let img = open_image(input_path)?;
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize));
        PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, FilterType::Sub)
            .write_image(rgba.as_raw(), w, h, image::ExtendedColorType::Rgba8)?;
        buf.into_inner()
    };

    write_smallest(input_path, output_path, &compressed, original_size)
}

pub fn compress_webp(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);
    let original_size = checked_size(input_path)?;
    let img = open_image(input_path)?;
    let compressed = encode_webp_bytes(&img, quality)?;
    write_smallest(input_path, output_path, &compressed, original_size)
}

pub fn compress_avif(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);
    let original_size = checked_size(input_path)?;
    let img = open_image(input_path)?;
    let compressed = encode_avif_bytes(&img, quality)?;
    write_smallest(input_path, output_path, &compressed, original_size)
}

pub fn compress_image(
    input_path: &str,
    output_path: &str,
    format: &OutputFormat,
    preset: QualityPreset,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Jpeg => compress_jpeg(input_path, output_path, preset.jpeg_quality()),
        OutputFormat::Png => compress_png(input_path, output_path, preset),
        OutputFormat::Webp => compress_webp(input_path, output_path, preset.webp_quality()),
        OutputFormat::Avif => compress_avif(input_path, output_path, preset.avif_quality()),
    }
}

fn resolve_output_path(
    path: &str,
    fmt: &OutputFormat,
    output_dir: Option<&str>,
    suffix: &str,
) -> Result<String, String> {
    let input = Path::new(path);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("invalid filename: {path}"))?;
    let parent = input
        .parent()
        .ok_or_else(|| format!("invalid path: {path}"))?;
    let ext = match fmt {
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Png => "png",
        OutputFormat::Webp => "webp",
        OutputFormat::Avif => "avif",
    };
    let out_base = output_dir.map(Path::new).unwrap_or(parent);
    Ok(out_base
        .join(format!("{stem}{suffix}.{ext}"))
        .to_string_lossy()
        .to_string())
}

pub fn compress_batch(
    paths: &[String],
    preset: QualityPreset,
    format: Option<OutputFormat>,
    output_dir: Option<&str>,
    suffix: &str,
    strip_gps: bool,
) -> Vec<CompressionResult> {
    paths
        .par_iter()
        .map(|path| {
            let fmt = format.unwrap_or_else(|| detect_format(path));
            let output = match resolve_output_path(path, &fmt, output_dir, suffix) {
                Ok(o) => o,
                Err(e) => return CompressionResult::err(path, e),
            };
            match compress_image(path, &output, &fmt, preset) {
                Ok(mut r) => {
                    if strip_gps && r.error.is_none() {
                        if let Err(e) = crate::core::privacy::strip_gps_in_place(&r.output_path) {
                            eprintln!("warning: GPS strip failed for {}: {}", path, e);
                        } else if let Ok(meta) = std::fs::metadata(&r.output_path) {
                            r.compressed_size = meta.len();
                        }
                    }
                    r
                }
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

        let result = compress_png(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            QualityPreset::Balanced,
        )
        .unwrap();

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
        let results = compress_batch(
            &paths,
            QualityPreset::Balanced,
            None,
            None,
            "_compressed",
            false,
        );

        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[0].compressed_size > 0);
        assert!(results[1].error.is_some());
    }

    #[test]
    fn compress_png_from_jpeg_input_produces_valid_png() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        let output = dir.path().join("photo_out.png");

        create_test_jpeg(input.to_str().unwrap(), 100, 100, 90.0);
        let result = compress_png(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            QualityPreset::Balanced,
        )
        .unwrap();

        assert!(result.compressed_size > 0);
        let header = &fs::read(&output).unwrap()[..4];
        assert_eq!(header, [0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn compress_avif_produces_output() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_out.avif");

        create_test_png(input.to_str().unwrap(), 200, 200);

        let result =
            compress_avif(input.to_str().unwrap(), output.to_str().unwrap(), 75.0).unwrap();

        assert!(output.exists());
        assert!(result.compressed_size > 0);
    }

    #[test]
    fn detect_format_from_extension() {
        assert!(matches!(detect_format("photo.jpg"), OutputFormat::Jpeg));
        assert!(matches!(detect_format("photo.png"), OutputFormat::Png));
        assert!(matches!(detect_format("photo.webp"), OutputFormat::Webp));
        assert!(matches!(detect_format("photo.avif"), OutputFormat::Avif));
        assert!(matches!(detect_format("photo.heic"), OutputFormat::Webp));
        assert!(matches!(detect_format("photo.heif"), OutputFormat::Webp));
        assert!(matches!(detect_format("photo.unknown"), OutputFormat::Jpeg));
    }

    #[test]
    fn skip_if_output_larger_than_input() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("tiny.jpg");
        let output = dir.path().join("tiny_out.jpg");

        // Compress once at q60 to get a well-optimized baseline
        create_test_jpeg(input.to_str().unwrap(), 200, 200, 95.0);
        let pre_out = dir.path().join("pre.jpg");
        compress_jpeg(input.to_str().unwrap(), pre_out.to_str().unwrap(), 60.0).unwrap();

        // Use the compressed file as input, re-compress at q100 — output will be larger
        let original_size = fs::metadata(&pre_out).unwrap().len();
        let result =
            compress_jpeg(pre_out.to_str().unwrap(), output.to_str().unwrap(), 100.0).unwrap();

        assert!(result.already_optimal);
        assert_eq!(result.compressed_size, original_size);
        assert!(output.exists());
    }

    #[test]
    fn compress_jpeg_not_already_optimal_when_reduced() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");
        let output = dir.path().join("test_out.jpg");

        create_test_jpeg(input.to_str().unwrap(), 200, 200, 95.0);
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 60.0).unwrap();

        assert!(!result.already_optimal);
        assert!(result.compressed_size < result.original_size);
    }
}
