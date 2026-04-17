use crate::core::watermark::{
    self, ImageWatermarkOptions, TextWatermarkOptions, WatermarkPosition, WatermarkResult,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextWatermarkDto {
    pub text: String,
    pub font_size: f32,
    pub opacity: f32,
    pub color_hex: String,
    pub position: String,
    pub padding: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageWatermarkDto {
    pub overlay_path: String,
    pub scale: f32,
    pub opacity: f32,
    pub position: String,
    pub padding: Option<u32>,
}

#[tauri::command]
pub async fn apply_text_watermark(
    input: String,
    output: Option<String>,
    options: TextWatermarkDto,
) -> Result<WatermarkResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let out = output.unwrap_or_else(|| watermark::derive_output_path(&input));
        watermark::apply_text_watermark(
            &input,
            &out,
            TextWatermarkOptions {
                text: options.text,
                font_size: options.font_size,
                opacity: options.opacity,
                color_hex: options.color_hex,
                position: WatermarkPosition::from_str(&options.position),
                padding: options.padding.unwrap_or(20),
            },
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn apply_image_watermark(
    input: String,
    output: Option<String>,
    options: ImageWatermarkDto,
) -> Result<WatermarkResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let out = output.unwrap_or_else(|| watermark::derive_output_path(&input));
        watermark::apply_image_watermark(
            &input,
            &out,
            ImageWatermarkOptions {
                overlay_path: options.overlay_path,
                scale: options.scale,
                opacity: options.opacity,
                position: WatermarkPosition::from_str(&options.position),
                padding: options.padding.unwrap_or(20),
            },
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
