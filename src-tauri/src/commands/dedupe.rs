use crate::core::dedupe;

#[tauri::command]
pub async fn find_duplicates(paths: Vec<String>) -> Result<dedupe::FindDuplicatesResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        dedupe::find_duplicates(&paths).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
