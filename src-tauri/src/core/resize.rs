use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::core::compression::{detect_format, OutputFormat};
use crate::core::image_io::{checked_size, ext_lowercase, open_image, write_preserving_timestamps};

fn encode_jpeg_mozjpeg(
    img: &image::DynamicImage,
    quality: f32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);
    let pixels = rgb.into_raw();

    let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    cinfo.set_size(w, h);
    cinfo.set_quality(quality);
    let mut cinfo = cinfo.start_compress(Vec::new())?;
    cinfo.write_scanlines(&pixels)?;
    Ok(cinfo.finish()?)
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

pub fn resize_image(
    input: &str,
    output: &str,
    width: u32,
    height: u32,
    mode: &ResizeMode,
    quality: Option<f32>,
) -> Result<ResizeResult, Box<dyn std::error::Error>> {
    let original_size = checked_size(input)?;
    let img = open_image(input)?;
    let (ow, oh) = (img.width(), img.height());

    let resized = match mode {
        ResizeMode::Fit => img.resize(width, height, FilterType::Lanczos3),
        ResizeMode::Exact => img.resize_exact(width, height, FilterType::Lanczos3),
    };

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
    write_preserving_timestamps(input, output, &bytes)?;

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
) -> Vec<ResizeResult> {
    paths
        .iter()
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
        let results = resize_batch(&paths, 200, 150, ResizeMode::Fit, None, None);

        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[0].new_width > 0);
        assert!(results[1].error.is_some());
    }
}
