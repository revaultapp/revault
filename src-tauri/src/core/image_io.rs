use image::ImageReader;
use std::fs;
use std::io::BufReader;
use std::path::Path;

use crate::core::compression::OutputFormat;

pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

pub fn ext_lowercase(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

pub fn open_image(path: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    match ext_lowercase(path).as_deref() {
        Some("heic") | Some("heif") => crate::core::heic::decode_heic(path),
        _ => {
            let file = fs::File::open(path)?;
            Ok(ImageReader::new(BufReader::new(file))
                .with_guessed_format()?
                .decode()?)
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

pub fn detect_format(path: &str) -> OutputFormat {
    match ext_lowercase(path).as_deref() {
        Some("png") => OutputFormat::Png,
        Some("webp") => OutputFormat::Webp,
        _ => OutputFormat::Jpeg,
    }
}
