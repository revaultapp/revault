use serde::Serialize;
use std::fs;

#[derive(Debug, Serialize)]
pub struct CompressionResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
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

    let input = fs::read(input_path)?;

    let dinfo = mozjpeg::Decompress::with_markers(mozjpeg::ALL_MARKERS).from_mem(&input)?;
    let mut rgb = dinfo.rgb()?;
    let width = rgb.width();
    let height = rgb.height();
    let pixels: Vec<u8> = rgb.read_scanlines()?;
    rgb.finish()?;

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
    })
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
}
