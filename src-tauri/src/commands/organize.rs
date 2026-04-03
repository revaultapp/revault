use crate::core::organize::{self, OrganizeMode};
use crate::core::rename::{RenameRequest, RenameResult};
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn rename_batch(requests: Vec<RenameRequest>) -> Result<Vec<RenameResult>, String> {
    tauri::async_runtime::spawn_blocking(move || Ok(crate::core::rename::rename_batch(&requests)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn organize_by_date(
    source: String,
    dest: String,
    copy: bool,
) -> Result<organize::OrganizeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mode = OrganizeMode { copy };
        Ok(organize::organize_by_date(&source, &dest, &mode))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn organize_by_date_stream(
    app: AppHandle,
    source: String,
    dest: String,
    copy: bool,
) -> Result<(), String> {
    // Collect images first to know the total
    let source_path = std::path::Path::new(&source);
    let mut images = Vec::new();
    organize::collect_images(source_path, true, &mut images).map_err(|e| e.to_string())?;
    let _total = images.len();

    let app_progress = app.clone();
    let app_complete = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mode = OrganizeMode { copy };
        organize::organize_by_date_stream(&source, &dest, &mode, move |progress| {
            let _ = app_progress.emit("organize-progress", progress);
        });
        let _ = app_complete.emit("organize-complete", ());
    });

    Ok(())
}
