use crate::core::compression;

#[tauri::command]
pub async fn compress_images(
    paths: Vec<String>,
    quality_preset: Option<compression::QualityPreset>,
    format: Option<compression::OutputFormat>,
    output_dir: Option<String>,
    strip_gps: Option<bool>,
) -> Result<Vec<compression::CompressionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(compression::compress_batch(
            &paths,
            quality_preset.unwrap_or(compression::QualityPreset::Balanced),
            format,
            output_dir.as_deref(),
            "_compressed",
            strip_gps.unwrap_or(false),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
