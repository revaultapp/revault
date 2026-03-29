use crate::core::organize::{self, OrganizeMode};
use crate::core::rename::{RenameRequest, RenameResult};

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
