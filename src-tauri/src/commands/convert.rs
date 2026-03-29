use crate::core::compression;

#[tauri::command]
pub async fn convert_images(
    paths: Vec<String>,
    format: compression::OutputFormat,
    quality_preset: Option<compression::QualityPreset>,
    output_dir: Option<String>,
    strip_gps: Option<bool>,
) -> Result<Vec<compression::CompressionResult>, String> {
    let strip_gps = strip_gps.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        Ok(compression::compress_batch(
            &paths,
            quality_preset.unwrap_or(compression::QualityPreset::Balanced),
            Some(format),
            output_dir.as_deref(),
            "_converted",
            strip_gps,
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
