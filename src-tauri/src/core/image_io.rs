use base64::Engine;
use image::ImageReader;
use std::fs;
use std::io::BufReader;
use std::path::Path;

pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

pub fn ext_lowercase(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

fn decode_jxl(path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    let image = jxl_oxide::JxlImage::builder()
        .read(std::io::Cursor::new(&data))
        .map_err(|e| e.to_string())?;
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
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(16384);
            limits.max_image_height = Some(16384);
            limits.max_alloc = Some(512 * 1024 * 1024);
            reader.limits(limits);
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
    let img = open_image(path)?;
    let thumb = img.thumbnail(max_size, max_size).to_rgb8();
    let (w, h) = (thumb.width() as usize, thumb.height() as usize);
    let jpeg_bytes = crate::core::compression::encode_jpeg_bytes(w, h, thumb.as_raw(), 60.0)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
    Ok(format!("data:image/jpeg;base64,{b64}"))
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
}
