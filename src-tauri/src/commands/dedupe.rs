use crate::core::dedupe;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
pub struct ScanProgress {
    pub current: usize,
    pub total: usize,
    pub phase: String,
}

#[tauri::command]
pub async fn find_duplicates(
    app: AppHandle,
    paths: Vec<String>,
    recursive: bool,
) -> Result<dedupe::FindDuplicatesResult, String> {
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        dedupe::find_duplicates_with_progress(&paths, recursive, move |current, total, phase| {
            let _ = app_clone.emit(
                "dedupe-progress",
                ScanProgress {
                    current,
                    total,
                    phase: phase.to_string(),
                },
            );
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
