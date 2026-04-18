use crate::core::video;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

static ACTIVE_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

#[tauri::command]
pub async fn compress_video(
    app: AppHandle,
    input: String,
    preset: video::VideoPreset,
    output_dir: Option<String>,
    privacy: video::PrivacyMode,
) -> Result<video::VideoCompressionResult, String> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    *ACTIVE_CANCEL.lock().map_err(|e| e.to_string())? = Some(cancel_flag.clone());

    let result = tauri::async_runtime::spawn_blocking(move || {
        video::compress_video(
            &input,
            preset,
            output_dir.as_deref(),
            privacy,
            cancel_flag,
            move |progress| {
                let _ = app.emit("video-compress-progress", &progress);
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    *ACTIVE_CANCEL.lock().map_err(|e| e.to_string())? = None;
    result
}

#[tauri::command]
pub async fn preview_video_compression(
    input: String,
    preset: video::VideoPreset,
    // Accepted from the frontend for IPC symmetry with compress_video. The
    // estimator is format-level (bitrate × factor) so privacy mode does not
    // change the output size materially.
    _privacy: video::PrivacyMode,
) -> Result<video::VideoCompressionPreview, String> {
    tauri::async_runtime::spawn_blocking(move || video::preview_video_compression(&input, preset))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cancel_video_compress() -> Result<(), String> {
    if let Some(flag) = ACTIVE_CANCEL.lock().map_err(|e| e.to_string())?.as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
pub async fn check_ffmpeg() -> bool {
    tauri::async_runtime::spawn_blocking(video::ffmpeg_is_available)
        .await
        .unwrap_or(false)
}

#[derive(Serialize, Clone)]
struct FfmpegDownloadProgress {
    downloaded: u64,
    total: u64,
    percent: f32,
}

#[tauri::command]
pub async fn reveal_video_output(path: String) -> Result<(), String> {
    let path = path.clone();
    tauri::async_runtime::spawn_blocking(move || video::reveal_in_file_manager(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn download_ffmpeg(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        video::download_ffmpeg(move |downloaded, total| {
            let percent = if total > 0 {
                (downloaded as f32 / total as f32 * 100.0).min(100.0)
            } else {
                0.0
            };
            let _ = app.emit(
                "ffmpeg-download-progress",
                FfmpegDownloadProgress {
                    downloaded,
                    total,
                    percent,
                },
            );
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map(|_| ())
}
