use crate::core::scanner;

#[tauri::command]
pub async fn scan_folder(path: String, recursive: bool) -> Result<scanner::ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        scanner::scan_folder(&path, recursive).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
