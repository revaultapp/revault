use crate::core::video;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

static CANCEL_FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

fn get_cancel_flag() -> Arc<AtomicBool> {
    CANCEL_FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

#[tauri::command]
pub async fn compress_video(
    app: AppHandle,
    input: String,
    preset: video::VideoPreset,
    output_dir: Option<String>,
) -> Result<video::VideoCompressionResult, String> {
    let flag = get_cancel_flag();
    flag.store(false, Ordering::SeqCst);

    tauri::async_runtime::spawn_blocking(move || {
        video::compress_video(
            &input,
            preset,
            output_dir.as_deref(),
            flag,
            move |progress| {
                let _ = app.emit("video-compress-progress", &progress);
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cancel_video_compress() -> Result<(), String> {
    get_cancel_flag().store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn check_ffmpeg() -> bool {
    video::ffmpeg_is_available()
}

#[derive(Serialize, Clone)]
struct FfmpegDownloadProgress {
    downloaded: u64,
    total: u64,
    percent: f32,
}

#[tauri::command]
pub async fn reveal_video_output(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("File not found: {}", path));
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(p.parent().unwrap_or(p).to_str().unwrap_or(&path))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
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
