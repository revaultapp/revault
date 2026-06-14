use crate::core::dedupe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

static ACTIVE_DEDUPE_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

#[derive(Clone, serde::Serialize)]
pub struct ScanProgress {
    pub request_id: u64,
    pub current: usize,
    pub total: usize,
    pub phase: String,
}

#[tauri::command]
pub async fn find_duplicates(
    app: AppHandle,
    paths: Vec<String>,
    recursive: bool,
    mode: dedupe::ScanMode,
    request_id: Option<u64>,
) -> Result<dedupe::FindDuplicatesResult, String> {
    let request_id = request_id.unwrap_or(0);
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut active = ACTIVE_DEDUPE_CANCEL.lock().map_err(|e| e.to_string())?;
        if active.is_some() {
            return Err("dedupe scan already running".to_string());
        }
        *active = Some(cancel_flag.clone());
    }
    let cancel_for_worker = cancel_flag.clone();
    let app_clone = app.clone();
    let join_result = tauri::async_runtime::spawn_blocking(move || {
        dedupe::find_duplicates_with_progress_and_cancel(
            &paths,
            recursive,
            mode,
            move |current, total, phase| {
                let _ = app_clone.emit(
                    "dedupe-progress",
                    ScanProgress {
                        request_id,
                        current,
                        total,
                        phase: phase.to_string(),
                    },
                );
            },
            &cancel_for_worker,
        )
        .map_err(|e| e.to_string())
    })
    .await;

    let mut active = ACTIVE_DEDUPE_CANCEL.lock().map_err(|e| e.to_string())?;
    if active
        .as_ref()
        .map(|flag| Arc::ptr_eq(flag, &cancel_flag))
        .unwrap_or(false)
    {
        *active = None;
    }
    join_result.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cancel_dedupe_scan() -> Result<(), String> {
    if let Some(flag) = ACTIVE_DEDUPE_CANCEL
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
    {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}
