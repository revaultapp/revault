use crate::core::compression;

#[tauri::command]
pub async fn convert_images(
    paths: Vec<String>,
    format: compression::OutputFormat,
    quality: Option<f32>,
    output_dir: Option<String>,
) -> Result<Vec<compression::CompressionResult>, String> {
    let quality = quality.unwrap_or(90.0);
    tauri::async_runtime::spawn_blocking(move || {
        Ok(compression::compress_batch(
            &paths,
            quality,
            Some(format),
            output_dir.as_deref(),
            "_converted",
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
