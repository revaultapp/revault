#[tauri::command]
pub async fn generate_thumbnail(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::image_io::generate_thumbnail(&path, 80).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
