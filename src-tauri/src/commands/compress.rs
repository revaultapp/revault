use crate::core::compression;

#[tauri::command]
pub async fn compress_images(
    paths: Vec<String>,
    quality: f32,
    format: Option<compression::OutputFormat>,
    output_dir: Option<String>,
) -> Result<Vec<compression::CompressionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(compression::compress_batch(
            &paths,
            quality,
            format,
            output_dir.as_deref(),
            "_compressed",
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn compress_to_target(
    paths: Vec<String>,
    target_bytes: u64,
    format: Option<compression::OutputFormat>,
    output_dir: Option<String>,
) -> Result<Vec<compression::CompressionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(compression::compress_to_target_batch(
            &paths,
            target_bytes,
            format,
            output_dir.as_deref(),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
