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
