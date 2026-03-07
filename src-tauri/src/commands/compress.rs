use crate::core::compression;

#[tauri::command]
pub async fn compress_images(paths: Vec<String>, quality: u8) -> Result<Vec<compression::CompressionResult>, String> {
    compression::compress_batch(&paths, quality).map_err(|e| e.to_string())
}
