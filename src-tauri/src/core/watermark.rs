use ab_glyph::{FontRef, PxScale};
use image::imageops::{self, FilterType};
use image::{ImageBuffer, ImageReader, Rgba, RgbaImage};
use std::io::BufReader;
use std::path::Path;

const BUNDLED_FONT: &[u8] = include_bytes!("../../assets/fonts/Roboto-Regular.ttf");

fn hex_to_rgba(hex: &str) -> Result<Rgba<u8>, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return Err(format!("invalid hex color: {}", hex));
    }
    let r =
        u8::from_str_radix(&hex[0..2], 16).map_err(|_| format!("invalid hex: {}", &hex[0..2]))?;
    let g =
        u8::from_str_radix(&hex[2..4], 16).map_err(|_| format!("invalid hex: {}", &hex[2..4]))?;
    let b =
        u8::from_str_radix(&hex[4..6], 16).map_err(|_| format!("invalid hex: {}", &hex[4..6]))?;
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
    } else {
        255
    };
    Ok(Rgba([r, g, b, a]))
}

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl WatermarkPosition {
    pub fn from_str(s: &str) -> Self {
        match s {
            "top_left" => Self::TopLeft,
            "top_center" => Self::TopCenter,
            "top_right" => Self::TopRight,
            "center_left" => Self::CenterLeft,
            "center" => Self::Center,
            "center_right" => Self::CenterRight,
            "bottom_left" => Self::BottomLeft,
            "bottom_center" => Self::BottomCenter,
            _ => Self::BottomRight,
        }
    }
}

pub struct TextWatermarkOptions {
    pub text: String,
    pub font_size: f32,
    pub opacity: f32,
    pub color_hex: String,
    pub position: WatermarkPosition,
    pub padding: u32,
}

pub struct ImageWatermarkOptions {
    pub overlay_path: String,
    pub scale: f32,
    pub opacity: f32,
    pub position: WatermarkPosition,
    pub padding: u32,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkResult {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
}

fn checked_size(path: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let size = std::fs::metadata(path)?.len();
    if size > MAX_FILE_SIZE {
        return Err("file exceeds 100 MB limit".into());
    }
    Ok(size)
}

fn open_image(path: &str) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    checked_size(path)?;
    let file = std::fs::File::open(path)?;
    let mut reader = ImageReader::new(BufReader::new(file)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader.decode()?.to_rgba8())
}

fn compute_position(
    img_w: u32,
    img_h: u32,
    item_w: u32,
    item_h: u32,
    pos: WatermarkPosition,
    padding: u32,
) -> (i64, i64) {
    let x = match pos {
        WatermarkPosition::TopLeft
        | WatermarkPosition::CenterLeft
        | WatermarkPosition::BottomLeft => padding as i64,
        WatermarkPosition::TopCenter
        | WatermarkPosition::Center
        | WatermarkPosition::BottomCenter => (img_w.saturating_sub(item_w) / 2) as i64,
        WatermarkPosition::TopRight
        | WatermarkPosition::CenterRight
        | WatermarkPosition::BottomRight => img_w.saturating_sub(item_w + padding) as i64,
    };
    let y = match pos {
        WatermarkPosition::TopLeft | WatermarkPosition::TopCenter | WatermarkPosition::TopRight => {
            padding as i64
        }
        WatermarkPosition::CenterLeft
        | WatermarkPosition::Center
        | WatermarkPosition::CenterRight => (img_h.saturating_sub(item_h) / 2) as i64,
        WatermarkPosition::BottomLeft
        | WatermarkPosition::BottomCenter
        | WatermarkPosition::BottomRight => img_h.saturating_sub(item_h + padding) as i64,
    };
    (x, y)
}

pub fn derive_output_path(input: &str) -> String {
    let path = Path::new(input);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let parent = path.parent().unwrap_or(Path::new("."));
    parent
        .join(format!("{stem}_watermarked.{ext}"))
        .to_string_lossy()
        .into_owned()
}

fn render_text_rgba(text: &str, font_size: f32, color: Rgba<u8>, opacity: f32) -> RgbaImage {
    use ab_glyph::{Font, ScaleFont};

    let font = FontRef::try_from_slice(BUNDLED_FONT).expect("bundled font is valid");
    let scale = PxScale::from(font_size);
    let scaled = font.as_scaled(scale);

    let height = scaled.height();
    let ascent = scaled.ascent();

    // Measure total width in a single pass
    let mut total_w: f32 = 0.0;
    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        total_w += scaled.h_advance(glyph_id);
    }

    let img_w = total_w.ceil() as u32 + 2;
    let img_h = height.ceil() as u32 + 2;

    let alpha_byte = (opacity.clamp(0.0, 1.0) * color.0[3] as f32) as u8;
    let mut buffer: RgbaImage = ImageBuffer::new(img_w, img_h);

    let mut cursor_x: f32 = 0.0;
    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        let scaled_glyph = glyph_id.with_scale(scale);

        if let Some(outline) = font.outline_glyph(scaled_glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|px, py, coverage| {
                let x = (cursor_x + bounds.min.x + px as f32) as i32;
                let y = (ascent + bounds.min.y + py as f32) as i32;
                if x >= 0 && x < img_w as i32 && y >= 0 && y < img_h as i32 {
                    let a = (coverage * alpha_byte as f32) as u8;
                    if a > 0 {
                        let pixel = buffer.get_pixel_mut(x as u32, y as u32);
                        let src_a = a as f32 / 255.0;
                        let dst_a = pixel.0[3] as f32 / 255.0;
                        let out_a = src_a + dst_a * (1.0 - src_a);
                        if out_a > 0.0 {
                            let blend = |s: u8, d: u8| -> u8 {
                                ((s as f32 * src_a + d as f32 * dst_a * (1.0 - src_a)) / out_a)
                                    as u8
                            };
                            pixel.0[0] = blend(color.0[0], pixel.0[0]);
                            pixel.0[1] = blend(color.0[1], pixel.0[1]);
                            pixel.0[2] = blend(color.0[2], pixel.0[2]);
                            pixel.0[3] = (out_a * 255.0) as u8;
                        }
                    }
                }
            });
        }
        cursor_x += scaled.h_advance(glyph_id);
    }

    buffer
}

fn overlay_with_opacity(base: &mut RgbaImage, overlay: &RgbaImage, x: i64, y: i64) {
    let (bw, bh) = (base.width() as i64, base.height() as i64);
    for oy in 0..overlay.height() {
        for ox in 0..overlay.width() {
            let bx = x + ox as i64;
            let by = y + oy as i64;
            if bx < 0 || by < 0 || bx >= bw || by >= bh {
                continue;
            }
            let src = overlay.get_pixel(ox, oy);
            if src.0[3] == 0 {
                continue;
            }
            let dst = base.get_pixel(bx as u32, by as u32);
            let sa = src.0[3] as f32 / 255.0;
            let da = dst.0[3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a > 0.0 {
                let blend = |s: u8, d: u8| -> u8 {
                    ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a) as u8
                };
                base.put_pixel(
                    bx as u32,
                    by as u32,
                    Rgba([
                        blend(src.0[0], dst.0[0]),
                        blend(src.0[1], dst.0[1]),
                        blend(src.0[2], dst.0[2]),
                        (out_a * 255.0) as u8,
                    ]),
                );
            }
        }
    }
}

pub fn apply_text_watermark(
    input: &str,
    output: &str,
    options: TextWatermarkOptions,
) -> Result<WatermarkResult, Box<dyn std::error::Error>> {
    let mut base = open_image(input)?;
    let color =
        hex_to_rgba(&options.color_hex).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let text_img = render_text_rgba(&options.text, options.font_size, color, options.opacity);
    let (x, y) = compute_position(
        base.width(),
        base.height(),
        text_img.width(),
        text_img.height(),
        options.position,
        options.padding,
    );
    overlay_with_opacity(&mut base, &text_img, x, y);
    let (w, h) = (base.width(), base.height());
    base.save(output)?;
    Ok(WatermarkResult {
        output_path: output.to_string(),
        width: w,
        height: h,
    })
}

pub fn apply_image_watermark(
    input: &str,
    output: &str,
    options: ImageWatermarkOptions,
) -> Result<WatermarkResult, Box<dyn std::error::Error>> {
    let mut base = open_image(input)?;
    let overlay_full = open_image(&options.overlay_path)?;

    let target_w = ((base.width() as f32) * options.scale.clamp(0.05, 1.0)) as u32;
    let aspect = overlay_full.height() as f32 / overlay_full.width() as f32;
    let target_h = (target_w as f32 * aspect) as u32;
    let mut overlay = imageops::resize(
        &overlay_full,
        target_w.max(1),
        target_h.max(1),
        FilterType::Lanczos3,
    );

    // Apply opacity to overlay alpha channel
    let opacity = options.opacity.clamp(0.0, 1.0);
    if opacity < 1.0 {
        for pixel in overlay.pixels_mut() {
            pixel.0[3] = (pixel.0[3] as f32 * opacity) as u8;
        }
    }

    let (x, y) = compute_position(
        base.width(),
        base.height(),
        overlay.width(),
        overlay.height(),
        options.position,
        options.padding,
    );
    overlay_with_opacity(&mut base, &overlay, x, y);
    let (w, h) = (base.width(), base.height());
    base.save(output)?;
    Ok(WatermarkResult {
        output_path: output.to_string(),
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_image(dir: &Path, name: &str, w: u32, h: u32) -> String {
        let path = dir.join(name);
        let img: RgbaImage = ImageBuffer::from_fn(w, h, |_, _| Rgba([100, 150, 200, 255]));
        img.save(&path).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn position_from_str_all() {
        assert_eq!(
            WatermarkPosition::from_str("top_left"),
            WatermarkPosition::TopLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("top_center"),
            WatermarkPosition::TopCenter
        );
        assert_eq!(
            WatermarkPosition::from_str("top_right"),
            WatermarkPosition::TopRight
        );
        assert_eq!(
            WatermarkPosition::from_str("center_left"),
            WatermarkPosition::CenterLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("center"),
            WatermarkPosition::Center
        );
        assert_eq!(
            WatermarkPosition::from_str("center_right"),
            WatermarkPosition::CenterRight
        );
        assert_eq!(
            WatermarkPosition::from_str("bottom_left"),
            WatermarkPosition::BottomLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("bottom_center"),
            WatermarkPosition::BottomCenter
        );
        assert_eq!(
            WatermarkPosition::from_str("bottom_right"),
            WatermarkPosition::BottomRight
        );
    }

    #[test]
    fn position_from_str_unknown_defaults() {
        assert_eq!(
            WatermarkPosition::from_str("unknown"),
            WatermarkPosition::BottomRight
        );
    }

    #[test]
    fn compute_position_top_left() {
        let (x, y) = compute_position(800, 600, 100, 50, WatermarkPosition::TopLeft, 20);
        assert_eq!(x, 20);
        assert_eq!(y, 20);
    }

    #[test]
    fn compute_position_center() {
        let (x, y) = compute_position(800, 600, 100, 50, WatermarkPosition::Center, 20);
        assert_eq!(x, 350);
        assert_eq!(y, 275);
    }

    #[test]
    fn compute_position_bottom_right() {
        let (x, y) = compute_position(800, 600, 100, 50, WatermarkPosition::BottomRight, 20);
        assert_eq!(x, 680);
        assert_eq!(y, 530);
    }

    #[test]
    fn derive_output_path_basic() {
        assert_eq!(
            derive_output_path("/tmp/photo.jpg"),
            "/tmp/photo_watermarked.jpg"
        );
    }

    #[test]
    fn derive_output_path_png() {
        assert_eq!(
            derive_output_path("/tmp/img.png"),
            "/tmp/img_watermarked.png"
        );
    }

    #[test]
    fn text_watermark_creates_output() {
        let dir = tempdir().unwrap();
        let input = make_test_image(dir.path(), "base.png", 400, 300);
        let output = dir.path().join("out.png").to_string_lossy().into_owned();

        let result = apply_text_watermark(
            &input,
            &output,
            TextWatermarkOptions {
                text: "Hello".to_string(),
                font_size: 32.0,
                opacity: 0.8,
                color_hex: "#ffffff".to_string(),
                position: WatermarkPosition::BottomRight,
                padding: 20,
            },
        )
        .unwrap();

        assert_eq!(result.width, 400);
        assert_eq!(result.height, 300);
        assert!(Path::new(&result.output_path).exists());
    }

    #[test]
    fn image_watermark_creates_output() {
        let dir = tempdir().unwrap();
        let input = make_test_image(dir.path(), "base.png", 400, 300);
        let overlay = make_test_image(dir.path(), "logo.png", 100, 100);
        let output = dir.path().join("out.png").to_string_lossy().into_owned();

        let result = apply_image_watermark(
            &input,
            &output,
            ImageWatermarkOptions {
                overlay_path: overlay,
                scale: 0.25,
                opacity: 0.5,
                position: WatermarkPosition::Center,
                padding: 10,
            },
        )
        .unwrap();

        assert_eq!(result.width, 400);
        assert_eq!(result.height, 300);
        assert!(Path::new(&result.output_path).exists());
    }

    #[test]
    fn text_watermark_invalid_path() {
        let result = apply_text_watermark(
            "/nonexistent/file.png",
            "/tmp/out.png",
            TextWatermarkOptions {
                text: "Test".to_string(),
                font_size: 24.0,
                opacity: 1.0,
                color_hex: "#000000".to_string(),
                position: WatermarkPosition::Center,
                padding: 0,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn text_watermark_zero_opacity() {
        let dir = tempdir().unwrap();
        let input = make_test_image(dir.path(), "base.png", 200, 200);
        let output = dir.path().join("out.png").to_string_lossy().into_owned();

        let result = apply_text_watermark(
            &input,
            &output,
            TextWatermarkOptions {
                text: "Ghost".to_string(),
                font_size: 24.0,
                opacity: 0.0,
                color_hex: "#ff0000".to_string(),
                position: WatermarkPosition::Center,
                padding: 0,
            },
        )
        .unwrap();

        // Should still succeed, just invisible
        assert!(Path::new(&result.output_path).exists());
    }
}
