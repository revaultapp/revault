use fast_image_resize::images::Image;
use fast_image_resize::{ResizeAlg, ResizeOptions, Resizer};
use image::DynamicImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

use crate::core::compression::{
    detect_format, encode_avif_bytes, encode_jpeg_bytes, encode_webp_bytes, OutputFormat,
    QualityPreset,
};
use crate::core::image_io::{checked_size, ext_lowercase, open_image, write_preserving_timestamps};
use crate::core::privacy;

fn encode_jpeg_mozjpeg(
    img: &image::DynamicImage,
    quality: f32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);
    encode_jpeg_bytes(w, h, rgb.as_raw(), quality)
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ResizeMode {
    Fit,
    Exact,
}

/// Per-format encode parameters for an optional quality preset. `None`
/// preserves the historical defaults exactly (quality 90.0, AVIF speed 5)
/// so pre-preset outputs are byte-identical.
fn resolve_encode_params(fmt: OutputFormat, preset: Option<QualityPreset>) -> (f32, u8) {
    let Some(p) = preset else { return (90.0, 5) };
    let quality = match fmt {
        OutputFormat::Jpeg => p.jpeg_quality(),
        OutputFormat::Webp => p.webp_quality(),
        OutputFormat::Avif => p.avif_quality(),
        OutputFormat::Png => 90.0, // lossless — value is ignored downstream
    };
    let speed = match fmt {
        OutputFormat::Avif => p.avif_speed(),
        _ => 5,
    };
    (quality, speed)
}

#[derive(Serialize)]
pub struct ResizeResult {
    pub input_path: String,
    pub output_path: String,
    pub original_width: u32,
    pub original_height: u32,
    pub new_width: u32,
    pub new_height: u32,
    pub original_size: u64,
    pub resized_size: u64,
    pub error: Option<String>,
}

impl ResizeResult {
    fn err(input: &str, msg: String) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: String::new(),
            original_width: 0,
            original_height: 0,
            new_width: 0,
            new_height: 0,
            original_size: fs::metadata(input).map(|m| m.len()).unwrap_or(0),
            resized_size: 0,
            error: Some(msg),
        }
    }
}

fn fit_dimensions(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let ratio = (max_w as f64 / src_w as f64).min(max_h as f64 / src_h as f64);
    let w = ((src_w as f64 * ratio).round() as u32).max(1);
    let h = ((src_h as f64 * ratio).round() as u32).max(1);
    (w, h)
}

fn fast_resize(
    img: &DynamicImage,
    width: u32,
    height: u32,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let src = img.to_rgba8();
    let src_view = fast_image_resize::images::ImageRef::new(
        img.width(),
        img.height(),
        src.as_raw(),
        fast_image_resize::PixelType::U8x4,
    )?;
    let mut dst = Image::new(width, height, fast_image_resize::PixelType::U8x4);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(
        fast_image_resize::FilterType::Lanczos3,
    ));
    resizer.resize(&src_view, &mut dst, &options)?;

    let buf = image::RgbaImage::from_raw(width, height, dst.into_vec())
        .ok_or("failed to reconstruct image from resized buffer")?;
    Ok(DynamicImage::ImageRgba8(buf))
}

pub fn resize_image(
    input: &str,
    output: &str,
    width: u32,
    height: u32,
    mode: &ResizeMode,
    preset: Option<QualityPreset>,
    strip_gps: bool,
) -> Result<ResizeResult, Box<dyn std::error::Error>> {
    if width == 0 || height == 0 {
        return Err("target width and height must be greater than zero".into());
    }
    crate::core::paths::validate_input_path(input, false)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let original_size = checked_size(input)?;
    let img = open_image(input)?;
    let (ow, oh) = (img.width(), img.height());

    let (dst_w, dst_h) = match mode {
        ResizeMode::Fit => fit_dimensions(img.width(), img.height(), width, height),
        ResizeMode::Exact => (width, height),
    };

    let resized = fast_resize(&img, dst_w, dst_h)?;

    let fmt = detect_format(input);
    let (quality, avif_speed) = resolve_encode_params(fmt, preset);

    let bytes = match fmt {
        OutputFormat::Jpeg => encode_jpeg_mozjpeg(&resized, quality)?,
        OutputFormat::Png => {
            let mut buf = Cursor::new(Vec::new());
            resized.write_to(&mut buf, image::ImageFormat::Png)?;
            buf.into_inner()
        }
        OutputFormat::Webp => encode_webp_bytes(&resized, quality)?,
        OutputFormat::Avif => encode_avif_bytes(&resized, quality, avif_speed)?,
    };

    if strip_gps {
        // Write to temp file in same directory as output (same filesystem for atomic rename)
        let output_path = Path::new(output);
        let parent = output_path
            .parent()
            .ok_or("output has no parent directory")?;
        let tmp = NamedTempFile::new_in(parent)?;
        write_preserving_timestamps(input, &tmp.path().to_string_lossy(), &bytes)?;
        if let Err(e) = privacy::strip_gps_in_place(&tmp.path().to_string_lossy()) {
            return Err(format!("resize succeeded but GPS strip failed: {e}").into());
        }
        tmp.persist(output_path)?;
    } else {
        write_preserving_timestamps(input, output, &bytes)?;
    }

    let resized_size = fs::metadata(output)?.len();

    Ok(ResizeResult {
        input_path: input.to_string(),
        output_path: output.to_string(),
        original_width: ow,
        original_height: oh,
        new_width: resized.width(),
        new_height: resized.height(),
        original_size,
        resized_size,
        error: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn resize_batch(
    paths: &[String],
    width: u32,
    height: u32,
    mode: ResizeMode,
    preset: Option<QualityPreset>,
    output_dir: Option<&str>,
    strip_gps: bool,
    suffix: Option<&str>,
) -> Vec<ResizeResult> {
    let suffix = suffix.unwrap_or("_resized");
    if let Err(e) = crate::core::paths::validate_output_suffix(suffix) {
        return paths
            .iter()
            .map(|p| ResizeResult::err(p, e.clone()))
            .collect();
    }
    // Canonicalize once outside the parallel loop — fails fast for the whole
    // batch instead of per-file, and prevents path traversal via "../..".
    let canonical_output_dir = match output_dir.map(crate::core::paths::validate_output_dir) {
        Some(Ok(canon)) => Some(canon),
        Some(Err(e)) => {
            return paths
                .iter()
                .map(|p| ResizeResult::err(p, e.clone()))
                .collect()
        }
        None => None,
    };

    let outputs = build_output_paths(paths, canonical_output_dir.as_deref(), suffix);
    paths
        .par_iter()
        .zip(outputs.into_par_iter())
        .map(|(path, output)| {
            let output = match output {
                Ok(output) => output,
                Err(e) => return ResizeResult::err(path, e),
            };
            match resize_image(path, &output, width, height, &mode, preset, strip_gps) {
                Ok(r) => r,
                Err(e) => ResizeResult::err(path, e.to_string()),
            }
        })
        .collect()
}

fn build_output_paths(
    paths: &[String],
    output_dir: Option<&Path>,
    suffix: &str,
) -> Vec<Result<String, String>> {
    let mut reserved = HashSet::new();
    paths
        .iter()
        .map(|path| build_output_path(path, output_dir, suffix, &mut reserved))
        .collect()
}

fn build_output_path(
    path: &str,
    output_dir: Option<&Path>,
    suffix: &str,
    reserved: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let input = Path::new(path);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("invalid filename: {path}"))?;
    let ext = ext_lowercase(path).unwrap_or_else(|| "jpg".to_string());
    let parent = input
        .parent()
        .ok_or_else(|| format!("invalid path: {path}"))?;
    let out_base = output_dir.unwrap_or(parent);
    crate::core::paths::first_available_path(
        &out_base.join(format!("{stem}{suffix}.{ext}")),
        reserved,
    )
    .to_str()
    .map(|s| s.to_string())
    .ok_or_else(|| "Invalid output path".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_png(path: &str, width: u32, height: u32) {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        img.save(path).unwrap();
    }

    fn create_test_jpeg(path: &str, width: u32, height: u32) {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        img.save(path).unwrap();
    }

    #[test]
    fn encode_params_default_to_90_and_speed_5_without_preset() {
        for fmt in [
            OutputFormat::Jpeg,
            OutputFormat::Png,
            OutputFormat::Webp,
            OutputFormat::Avif,
        ] {
            assert_eq!(resolve_encode_params(fmt, None), (90.0, 5));
        }
    }

    #[test]
    fn encode_params_map_preset_per_format() {
        // Must match compression.rs exactly so both tools stay tuned identically.
        let cases = [
            (QualityPreset::Smallest, 45.0, 40.0, 50.0, 2),
            (QualityPreset::Balanced, 75.0, 72.0, 72.0, 4),
            (QualityPreset::HighQuality, 88.0, 85.0, 88.0, 3),
        ];
        for (preset, jpeg, webp, avif, speed) in cases {
            assert_eq!(
                resolve_encode_params(OutputFormat::Jpeg, Some(preset)),
                (jpeg, 5)
            );
            assert_eq!(
                resolve_encode_params(OutputFormat::Webp, Some(preset)),
                (webp, 5)
            );
            assert_eq!(
                resolve_encode_params(OutputFormat::Avif, Some(preset)),
                (avif, speed)
            );
            // PNG is lossless — quality placeholder, never preset-dependent speed.
            assert_eq!(
                resolve_encode_params(OutputFormat::Png, Some(preset)),
                (90.0, 5)
            );
        }
    }

    #[test]
    fn resize_applies_preset_quality_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        let out_small = dir.path().join("photo_small.jpg");
        let out_high = dir.path().join("photo_high.jpg");
        create_test_jpeg(input.to_str().unwrap(), 400, 300);

        let small = resize_image(
            input.to_str().unwrap(),
            out_small.to_str().unwrap(),
            200,
            150,
            &ResizeMode::Exact,
            Some(QualityPreset::Smallest),
            false,
        )
        .unwrap();
        let high = resize_image(
            input.to_str().unwrap(),
            out_high.to_str().unwrap(),
            200,
            150,
            &ResizeMode::Exact,
            Some(QualityPreset::HighQuality),
            false,
        )
        .unwrap();

        // The preset must actually reach the encoder: lower quality → fewer bytes.
        assert!(small.resized_size < high.resized_size);
    }

    #[test]
    fn resize_reduces_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        let output = dir.path().join("test_resized.png");

        create_test_png(input.to_str().unwrap(), 400, 300);
        let result = resize_image(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            200,
            150,
            &ResizeMode::Exact,
            None,
            false,
        )
        .unwrap();

        assert_eq!(result.new_width, 200);
        assert_eq!(result.new_height, 150);
        assert_eq!(result.original_width, 400);
        assert_eq!(result.original_height, 300);
        assert!(output.exists());
    }

    #[test]
    fn resize_invalid_path_returns_error() {
        let result = resize_image(
            "/nonexistent/img.png",
            "/tmp/out.png",
            100,
            100,
            &ResizeMode::Fit,
            None,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn resize_batch_handles_mixed_results() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        create_test_png(input.to_str().unwrap(), 400, 300);

        let paths = vec![
            input.to_string_lossy().to_string(),
            "/nonexistent/fake.png".to_string(),
        ];
        let results = resize_batch(&paths, 200, 150, ResizeMode::Fit, None, None, false, None);

        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[0].new_width > 0);
        assert!(results[1].error.is_some());
    }

    #[test]
    fn resize_batch_does_not_clobber_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.png");
        let existing = dir.path().join("photo_resized.png");
        let expected = dir.path().join("photo_resized_2.png");
        create_test_png(input.to_str().unwrap(), 400, 300);
        fs::write(&existing, b"existing").unwrap();

        let paths = vec![input.to_string_lossy().to_string()];
        let results = resize_batch(&paths, 200, 150, ResizeMode::Fit, None, None, false, None);

        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
        assert_eq!(Path::new(&results[0].output_path), expected);
        assert_eq!(fs::read(&existing).unwrap(), b"existing");
        assert!(expected.exists());
    }

    #[test]
    fn resize_batch_avoids_output_collisions_within_batch() {
        let dir = tempfile::tempdir().unwrap();
        let input_a = dir.path().join("a").join("photo.png");
        let input_b = dir.path().join("b").join("photo.png");
        std::fs::create_dir_all(input_a.parent().unwrap()).unwrap();
        std::fs::create_dir_all(input_b.parent().unwrap()).unwrap();
        create_test_png(input_a.to_str().unwrap(), 400, 300);
        create_test_png(input_b.to_str().unwrap(), 400, 300);

        let paths = vec![
            input_a.to_string_lossy().to_string(),
            input_b.to_string_lossy().to_string(),
        ];
        let results = resize_batch(
            &paths,
            200,
            150,
            ResizeMode::Fit,
            None,
            Some(dir.path().to_str().unwrap()),
            false,
            None,
        );

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.error.is_none()));
        assert_ne!(results[0].output_path, results[1].output_path);
        assert!(results[0].output_path.ends_with("photo_resized.png"));
        assert!(results[1].output_path.ends_with("photo_resized_2.png"));
        assert!(Path::new(&results[0].output_path).exists());
        assert!(Path::new(&results[1].output_path).exists());
    }

    #[test]
    fn resize_batch_rejects_unsafe_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.png");
        create_test_png(input.to_str().unwrap(), 400, 300);

        let paths = vec![input.to_string_lossy().to_string()];
        let results = resize_batch(
            &paths,
            200,
            150,
            ResizeMode::Fit,
            None,
            None,
            false,
            Some("/../../test"),
        );

        assert_eq!(results.len(), 1);
        assert!(results[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("output suffix"));
        assert!(!dir.path().join("test.jpg").exists());
    }
}
