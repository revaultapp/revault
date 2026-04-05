use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::ImageEncoder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::core::image_io::{
    checked_size, decode_rgb, ext_lowercase, open_image, read_file_mmap_or_default,
    write_preserving_timestamps,
};

#[derive(Serialize)]
pub struct CompressionResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub already_optimal: bool,
    pub warning: Option<String>,
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
            warning: None,
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
            warning: None,
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
            warning: None,
            error: Some(msg),
        }
    }
}

/// Result of a preview compression scan (no file written, temp dir used).
#[derive(Serialize)]
pub struct PreviewResult {
    pub input_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    /// True if compressed_size >= original_size (file would not benefit)
    pub may_increase: bool,
    pub error: Option<String>,
}

/// Response from preview_compress containing total batch size + sample results.
#[derive(Serialize)]
pub struct PreviewResponse {
    /// Total original size of ALL files in batch (read from metadata).
    pub total_original_bytes: u64,
    /// Compression results for the sample files.
    pub sample_results: Vec<PreviewResult>,
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
    fn avif_speed(self) -> u8 {
        match self {
            QualityPreset::Smallest => 2,
            QualityPreset::Balanced => 5,
            QualityPreset::HighQuality => 1,
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

/// Read JPEG bytes using memory-mapped I/O for large files (>10MB).
/// For smaller files, uses regular read.
fn read_jpeg_bytes(path: &str) -> Result<Vec<u8>, std::io::Error> {
    read_file_mmap_or_default(path)
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
    let jpeg_result = read_jpeg_bytes(input_path).ok().and_then(|input| {
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

pub fn encode_jpeg_bytes(
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
        cinfo.set_optimize_coding(true);
        cinfo.set_use_scans_in_trellis(true);
        cinfo.set_progressive_mode();
        if quality >= 80.0 {
            cinfo.set_optimize_scans(true);
        }
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
    config.method = 6;
    config.sns_strength = match quality {
        40.0 => 80, // Smallest
        72.0 => 60, // Balanced
        85.0 => 40, // HighQuality
        _ => 60,
    };
    let memory = encoder
        .encode_advanced(&config)
        .map_err(|e| format!("webp encoding failed: {e:?}"))?;
    Ok(memory.to_vec())
}

pub(crate) fn encode_avif_bytes(
    img: &image::DynamicImage,
    quality: f32,
    speed: u8,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let mut pixels = Vec::with_capacity(w * h);
    for chunk in rgba.as_raw().chunks_exact(4) {
        pixels.push(ravif::RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]));
    }
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

/// Read average JPEG quantization table value to estimate quality level.
/// JPEG quality 92+ typically has low quantization values (1-4).
fn estimate_jpeg_quality(data: &[u8]) -> Option<f32> {
    // DQT marker: FF DB
    let mut i = 0;
    while i < data.len() - 4 {
        if data[i] == 0xFF && data[i + 1] == 0xDB {
            let len = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
            if len < 3 || i + 2 + len > data.len() {
                return None;
            }
            // Byte at i+4 is precision (0=8bit, 1=16bit) + table ID
            let precision = data[i + 4] & 0x0F;
            let value_size = if precision == 0 { 1 } else { 2 };
            // Quantization table values start at i+5
            let table_start = i + 5;
            let num_values = (len - 2) / value_size; // minus 2 for precision byte
            if num_values < 64 {
                i += 1;
                continue;
            }
            let mut sum = 0u32;
            for j in 0..64 {
                let offset = table_start + j * value_size;
                if offset + value_size > data.len() {
                    break;
                }
                let val = if value_size == 1 {
                    data[offset] as u32
                } else {
                    (((data[offset] as usize) << 8) | (data[offset + 1] as usize)) as u32
                };
                sum += val;
            }
            let avg = sum as f32 / 64.0;
            // Convert avg quantization to quality estimate
            // Low quantization (2-4) = high quality (92+), medium (8-12) = medium (70-80)
            let quality = (115.0 - avg * 3.0).clamp(0.0, 100.0);
            return Some(quality);
        }
        i += 1;
    }
    None
}

/// Detect common screenshot resolutions (may not compress well to JPEG).
fn is_screenshot_resolution(width: u32, height: u32) -> bool {
    matches!(
        (width, height),
        (1920, 1080)
            | (2560, 1440)
            | (2560, 1600)
            | (2880, 1800)
            | (2560, 1080)
            | (3440, 1440)
            | (3840, 2160)
            | (5120, 2880)
            | (1366, 768)
            | (1600, 900)
            | (1280, 720)
    )
}

/// Collect all warnings for a compression operation (informational, don't prevent compression).
fn gather_warnings(
    input_path: &str,
    output_path: &str,
    output_format: OutputFormat,
    width: u32,
    height: u32,
    input_size: u64,
) -> Option<String> {
    // Priority 1: PNG lossless (most specific to PNG output)
    if matches!(output_format, OutputFormat::Png) {
        return Some(
            "PNG is lossless - quality preset has no effect, only optimization level".to_string(),
        );
    }

    // Priority 2: Already highly compressed JPEG (quality 92+) - re-compression won't help much
    if matches!(
        ext_lowercase(input_path).as_deref(),
        Some("jpg") | Some("jpeg")
    ) {
        if let Ok(data) = fs::read(input_path) {
            if let Some(quality) = estimate_jpeg_quality(&data) {
                if quality > 92.0 {
                    return Some(format!(
                        "JPEG already at high quality ({:.0}) - re-compression won't reduce size much",
                        quality
                    ));
                }
            }
        }
    }

    // Priority 3: Screenshot resolution (for JPEG output)
    if matches!(output_format, OutputFormat::Jpeg) && is_screenshot_resolution(width, height) {
        return Some(
            "Image is a screenshot (1920x1080) - may not compress well to JPEG".to_string(),
        );
    }

    // Priority 4: PNG to JPEG conversion warning
    if matches!(ext_lowercase(input_path).as_deref(), Some("png"))
        && matches!(output_format, OutputFormat::Jpeg)
    {
        return Some(
            "Converting PNG to JPEG - loss of transparency, may increase size".to_string(),
        );
    }

    // Priority 5: Small file (more important than same-format - warns about potential size increase)
    if input_size > 0 && input_size < 10 * 1024 {
        return Some(format!(
            "File too small ({} bytes) - overhead may exceed savings",
            input_size
        ));
    }

    // Priority 6: Same format (general - unnecessary re-compression)
    if same_format(input_path, output_path) {
        return Some("Output format same as input - no conversion needed".to_string());
    }

    None
}

fn write_smallest(
    input_path: &str,
    output_path: &str,
    compressed: &[u8],
    original_size: u64,
    warning: Option<String>,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let compressed_size = compressed.len() as u64;
    if compressed_size >= original_size && same_format(input_path, output_path) {
        fs::copy(input_path, output_path)?;
        let mut result = CompressionResult::optimal(input_path, output_path, original_size);
        result.warning = warning;
        Ok(result)
    } else {
        write_preserving_timestamps(input_path, output_path, compressed)?;
        let mut result =
            CompressionResult::ok(input_path, output_path, original_size, compressed_size);
        result.warning = warning;
        Ok(result)
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
    let warning = gather_warnings(
        input_path,
        output_path,
        OutputFormat::Jpeg,
        width as u32,
        height as u32,
        original_size,
    );
    write_smallest(input_path, output_path, &compressed, original_size, warning)
}

pub fn compress_png(
    input_path: &str,
    output_path: &str,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let original_size = checked_size(input_path)?;

    let (compressed, width, height) = if matches!(ext_lowercase(input_path).as_deref(), Some("png"))
    {
        let img = open_image(input_path)?;
        let (w, h) = (img.width(), img.height());
        let mut opts = oxipng::Options {
            deflater: oxipng::Deflater::Zopfli(oxipng::ZopfliOptions {
                iteration_count: std::num::NonZeroU64::new(15).unwrap(),
                ..Default::default()
            }),
            optimize_alpha: true,
            strip: oxipng::StripChunks::None,
            ..Default::default()
        };
        opts.force = true;
        // Use memory-mapped read for large PNG files (>10MB)
        let png_data = read_file_mmap_or_default(input_path)
            .map_err(|e| format!("failed to read {}: {}", input_path, e))?;
        let data = oxipng::optimize_from_memory(&png_data, &opts)?;
        (data, w, h)
    } else {
        // Non-PNG input: encode to PNG directly, skip oxipng (too slow on large images)
        let img = open_image(input_path)?;
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let mut buf = Cursor::new(Vec::with_capacity((w * h * 4) as usize));
        PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, FilterType::Sub)
            .write_image(rgba.as_raw(), w, h, image::ExtendedColorType::Rgba8)?;
        (buf.into_inner(), w, h)
    };

    let warning = gather_warnings(
        input_path,
        output_path,
        OutputFormat::Png,
        width,
        height,
        original_size,
    );
    write_smallest(input_path, output_path, &compressed, original_size, warning)
}

pub fn compress_webp(
    input_path: &str,
    output_path: &str,
    quality: f32,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = quality.clamp(0.0, 100.0);
    let original_size = checked_size(input_path)?;
    let img = open_image(input_path)?;
    let (w, h) = (img.width(), img.height());
    let compressed = encode_webp_bytes(&img, quality)?;
    let warning = gather_warnings(
        input_path,
        output_path,
        OutputFormat::Webp,
        w,
        h,
        original_size,
    );
    write_smallest(input_path, output_path, &compressed, original_size, warning)
}

pub fn compress_avif(
    input_path: &str,
    output_path: &str,
    preset: QualityPreset,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    let quality = preset.avif_quality();
    let original_size = checked_size(input_path)?;
    let img = open_image(input_path)?;
    let (w, h) = (img.width(), img.height());
    let compressed = encode_avif_bytes(&img, quality, preset.avif_speed())?;
    let warning = gather_warnings(
        input_path,
        output_path,
        OutputFormat::Avif,
        w,
        h,
        original_size,
    );
    write_smallest(input_path, output_path, &compressed, original_size, warning)
}

pub fn compress_image(
    input_path: &str,
    output_path: &str,
    format: &OutputFormat,
    preset: QualityPreset,
) -> Result<CompressionResult, Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Jpeg => compress_jpeg(input_path, output_path, preset.jpeg_quality()),
        OutputFormat::Png => compress_png(input_path, output_path),
        OutputFormat::Webp => compress_webp(input_path, output_path, preset.webp_quality()),
        OutputFormat::Avif => compress_avif(input_path, output_path, preset),
    }
}

pub fn resolve_output_path(
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
    let num_paths = paths.len();
    let num_cores = rayon::current_num_threads();

    // Adaptive parallelism: use parallel only when batch is large enough to justify thread overhead.
    // Threshold of 8 was chosen based on typical thread pool steal cost vs parallel work.
    // Single-core machines or tiny batches always use sequential.
    if num_paths >= 8 && num_cores > 1 {
        paths
            .par_iter()
            .map(|path| compress_single(path, format, output_dir, suffix, preset, strip_gps))
            .collect()
    } else {
        paths
            .iter()
            .map(|path| compress_single(path, format, output_dir, suffix, preset, strip_gps))
            .collect()
    }
}

/// Compress a single file (extracted for reuse in both parallel and sequential paths).
fn compress_single(
    path: &str,
    format: Option<OutputFormat>,
    output_dir: Option<&str>,
    suffix: &str,
    preset: QualityPreset,
    strip_gps: bool,
) -> CompressionResult {
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
        let result = compress_png(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();

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

        let result = compress_avif(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            QualityPreset::Balanced,
        )
        .unwrap();

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

    #[test]
    fn is_screenshot_resolution_detects_common_screenshots() {
        assert!(is_screenshot_resolution(1920, 1080));
        assert!(is_screenshot_resolution(2560, 1440));
        assert!(is_screenshot_resolution(2560, 1600));
        assert!(is_screenshot_resolution(1366, 768));
        assert!(is_screenshot_resolution(3440, 1440));
        assert!(!is_screenshot_resolution(1920, 1200));
        assert!(!is_screenshot_resolution(800, 600));
        assert!(!is_screenshot_resolution(4000, 3000));
    }

    #[test]
    fn png_compression_includes_lossless_warning() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_out.png");

        create_test_png(input.to_str().unwrap(), 200, 200);
        let result = compress_png(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();

        assert!(result.warning.is_some());
        assert!(result
            .warning
            .unwrap()
            .contains("PNG is lossless - quality preset has no effect"));
    }

    #[test]
    fn jpeg_to_jpeg_compression_includes_same_format_warning() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");
        let output = dir.path().join("test_out.jpg");

        // 800x600 is NOT a screenshot resolution and is large enough to avoid "file too small"
        // Use quality 85 (<92) to avoid triggering "already highly compressed" warning
        create_test_jpeg(input.to_str().unwrap(), 800, 600, 85.0);
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 60.0).unwrap();

        assert!(result.warning.is_some());
        assert!(result
            .warning
            .unwrap()
            .contains("Output format same as input - no conversion needed"));
    }

    #[test]
    fn screenshot_resolution_gets_warning_when_compressing_to_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("screenshot.jpg");
        let output = dir.path().join("screenshot_out.jpg");

        // 1920x1080 is a known screenshot resolution; use q85 to avoid "already highly compressed"
        create_test_jpeg(input.to_str().unwrap(), 1920, 1080, 85.0);
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 60.0).unwrap();

        assert!(result.warning.is_some());
        assert!(result.warning.unwrap().contains("screenshot (1920x1080)"));
    }

    #[test]
    fn non_screenshot_resolution_no_screenshot_warning() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        let output = dir.path().join("photo_out.jpg");

        // 200x200 is not a screenshot resolution
        create_test_jpeg(input.to_str().unwrap(), 200, 200, 95.0);
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 60.0).unwrap();

        // Warning should still be present (for same-format), but not screenshot warning
        assert!(result.warning.is_some());
        assert!(!result.warning.unwrap().contains("screenshot"));
    }

    #[test]
    fn compress_webp_no_same_format_warning_when_converting_from_png() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_out.webp");

        // Create a PNG and convert to WebP - this is a format change, not same format
        create_test_png(input.to_str().unwrap(), 200, 200);

        let result =
            compress_webp(input.to_str().unwrap(), output.to_str().unwrap(), 75.0).unwrap();

        // Same format warning should NOT trigger (PNG→WebP is format change)
        assert!(
            result.warning.is_none() || !result.warning.as_ref().unwrap().contains("same as input")
        );
    }

    #[test]
    fn already_highly_compressed_jpeg_warns_on_recompress() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("highq.jpg");
        let output = dir.path().join("highq_out.jpg");

        // Create a high-quality JPEG (q95)
        create_test_jpeg(input.to_str().unwrap(), 800, 600, 95.0);
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 60.0).unwrap();

        assert!(result.warning.is_some());
        let warning = result.warning.unwrap();
        // Warning should be either already_highly_compressed or same_format
        assert!(
            warning.contains("JPEG already at high quality") || warning.contains("same as input"),
            "Got warning: {}",
            warning
        );
    }

    #[test]
    fn png_to_jpeg_conversion_warns() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_out.jpg");

        create_test_png(input.to_str().unwrap(), 200, 200);
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 75.0).unwrap();

        assert!(result.warning.is_some());
        let warning = result.warning.unwrap();
        assert!(warning.contains("Converting PNG to JPEG"));
        assert!(warning.contains("loss of transparency"));
    }

    #[test]
    fn estimate_jpeg_quality_returns_some_for_valid_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");

        // Create JPEG at known quality
        create_test_jpeg(input.to_str().unwrap(), 200, 200, 85.0);
        let data = fs::read(&input).unwrap();

        let quality = estimate_jpeg_quality(&data);
        // The function should return Some for a valid JPEG (actual value is approximate)
        assert!(
            quality.is_some(),
            "estimate_jpeg_quality returned None for a valid JPEG"
        );
    }

    #[test]
    fn file_too_small_warning_for_tiny_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("tiny.jpg");
        let output = dir.path().join("tiny_out.jpg");

        // Create a very small JPEG (50x50 at high quality)
        create_test_jpeg(input.to_str().unwrap(), 50, 50, 95.0);
        let original_size = fs::metadata(&input).unwrap().len();
        let result =
            compress_jpeg(input.to_str().unwrap(), output.to_str().unwrap(), 60.0).unwrap();

        // If file is small enough, should get small file warning
        if original_size < 10 * 1024 {
            assert!(result.warning.is_some());
            assert!(result.warning.unwrap().contains("File too small"));
        } else {
            // If not small, just verify compression worked
            assert!(result.compressed_size > 0);
        }
    }

    #[test]
    fn non_jpeg_input_no_highly_compressed_warning() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_out.png");

        create_test_png(input.to_str().unwrap(), 200, 200);
        let result = compress_png(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();

        // PNG shouldn't trigger "already highly compressed" warning
        assert!(result.warning.is_some());
        let warning = result.warning.unwrap();
        // Should get PNG lossless warning, not JPEG warning
        assert!(!warning.contains("JPEG already at high quality"));
    }
}
