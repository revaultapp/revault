use crate::core::gif;
use serde_json::json;
use tauri::{Emitter, Manager};

#[tauri::command]
pub async fn export_gif(
    app: tauri::AppHandle,
    input_path: String,
    output_path: String,
    options: gif::GifOptions,
) -> Result<gif::GifResult, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        gif::export_gif(&app_data, &input_path, &output_path, options)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn estimate_gif_size(options: gif::GifOptions) -> Result<u64, String> {
    options.validate()?;
    Ok(gif::estimate_gif_size(&options))
}

#[tauri::command]
pub async fn check_gifski(app: tauri::AppHandle) -> Result<bool, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || gif::check_gifski(&app_data))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn download_gifski(app: tauri::AppHandle) -> Result<(), String> {
    eprintln!("[gifski] download_gifski command invoked");
    let app_data = app.path().app_data_dir().map_err(|e| {
        eprintln!("[gifski] app_data_dir failed: {}", e);
        e.to_string()
    })?;
    let app_for_emit = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        gif::download_and_install(&app_data, move |done, total| {
            let _ = app_for_emit.emit(
                "gifski-download-progress",
                json!({
                    "bytes_done": done,
                    "bytes_total": total,
                }),
            );
        })
    })
    .await
    .map_err(|e| {
        eprintln!("[gifski] spawn_blocking join error: {}", e);
        e.to_string()
    })?;
    match &result {
        Ok(path) => eprintln!(
            "[gifski] download_gifski command OK, installed at {}",
            path.display()
        ),
        Err(e) => eprintln!("[gifski] download_gifski command ERR: {}", e),
    }
    result.map(|_| ())
}
