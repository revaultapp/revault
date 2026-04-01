#[tauri::command]
pub async fn generate_thumbnail(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::image_io::generate_thumbnail(&path, 80).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_file_sizes(paths: Vec<String>) -> Vec<u64> {
    let len = paths.len();
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
            .collect()
    })
    .await
    .unwrap_or_else(|_| vec![0; len])
}
