use crate::core::privacy;

#[tauri::command]
pub async fn read_metadata(path: String) -> Result<privacy::MetadataResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        privacy::read_metadata(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn strip_files(
    paths: Vec<String>,
    output_dir: Option<String>,
) -> Result<Vec<privacy::StripResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(privacy::strip_batch(&paths, output_dir.as_deref()))
    })
    .await
    .map_err(|e| e.to_string())?
}
