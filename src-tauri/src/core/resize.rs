use fast_image_resize::images::Image;
use fast_image_resize::{ResizeAlg, ResizeOptions, Resizer};
use image::DynamicImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tempfile::NamedTempFile;

use crate::core::compression::{detect_format, encode_jpeg_bytes, OutputFormat};
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
    quality: Option<f32>,
    strip_gps: bool,
) -> Result<ResizeResult, Box<dyn std::error::Error>> {
    let original_size = checked_size(input)?;
    let img = open_image(input)?;
    let (ow, oh) = (img.width(), img.height());

    let (dst_w, dst_h) = match mode {
        ResizeMode::Fit => fit_dimensions(img.width(), img.height(), width, height),
        ResizeMode::Exact => (width, height),
    };

    let resized = fast_resize(&img, dst_w, dst_h)?;

    let fmt = detect_format(input);
    let quality = quality.unwrap_or(90.0).clamp(0.0, 100.0);

    let bytes = match fmt {
        OutputFormat::Jpeg => encode_jpeg_mozjpeg(&resized, quality)?,
        OutputFormat::Png => {
            let mut buf = Cursor::new(Vec::new());
            resized.write_to(&mut buf, image::ImageFormat::Png)?;
            buf.into_inner()
        }
        OutputFormat::Webp => {
            let encoder = webp::Encoder::from_image(&resized)?;
            let mut config = webp::WebPConfig::new().map_err(|_| "failed to create WebP config")?;
            config.quality = quality;
            config.method = 0;
            let memory = encoder
                .encode_advanced(&config)
                .map_err(|e| format!("webp encoding failed: {e:?}"))?;
            memory.to_vec()
        }
        OutputFormat::Avif => {
            let rgba = resized.to_rgba8();
            let (w, h) = (rgba.width() as usize, rgba.height() as usize);
            let pixels: Vec<ravif::RGBA8> = rgba
                .as_raw()
                .chunks_exact(4)
                .map(|c| ravif::RGBA8::new(c[0], c[1], c[2], c[3]))
                .collect();
            let encoded = ravif::Encoder::new()
                .with_quality(quality)
                .with_speed(6)
                .with_alpha_quality(quality)
                .encode_rgba(ravif::Img::new(&pixels, w, h))
                .map_err(|e| format!("avif encoding failed: {e}"))?;
            encoded.avif_file
        }
    };

    let resized_size = bytes.len() as u64;

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

pub fn resize_batch(
    paths: &[String],
    width: u32,
    height: u32,
    mode: ResizeMode,
    quality: Option<f32>,
    output_dir: Option<&str>,
    strip_gps: bool,
) -> Vec<ResizeResult> {
    paths
        .par_iter()
        .map(|path| {
            let input = Path::new(path.as_str());
            let stem = match input.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => return ResizeResult::err(path, format!("invalid filename: {path}")),
            };
            let ext = ext_lowercase(path).unwrap_or_else(|| "jpg".to_string());
            let parent = match input.parent() {
                Some(p) => p,
                None => return ResizeResult::err(path, format!("invalid path: {path}")),
            };

            let out_base = output_dir.map(Path::new).unwrap_or(parent);
            let output = out_base.join(format!("{stem}_resized.{ext}"));

            match resize_image(
                path,
                &output.to_string_lossy(),
                width,
                height,
                &mode,
                quality,
                strip_gps,
            ) {
                Ok(r) => r,
                Err(e) => ResizeResult::err(path, e.to_string()),
            }
        })
        .collect()
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
        let results = resize_batch(&paths, 200, 150, ResizeMode::Fit, None, None, false);

        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[0].new_width > 0);
        assert!(results[1].error.is_some());
    }
}
