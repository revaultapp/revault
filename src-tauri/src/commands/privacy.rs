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

#[tauri::command]
pub async fn strip_files_selective(
    paths: Vec<String>,
    strip_gps: bool,
    strip_device: bool,
    strip_datetime: bool,
    strip_author: bool,
    output_dir: Option<String>,
) -> Result<Vec<privacy::StripResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let opts = privacy::StripOptions {
            gps: strip_gps,
            device: strip_device,
            datetime: strip_datetime,
            author: strip_author,
        };
        Ok(privacy::strip_selective_batch(
            &paths,
            opts,
            output_dir.as_deref(),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
