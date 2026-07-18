use crate::core::cancel::CancelSlot;
use crate::core::video;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

static ACTIVE_CANCEL: CancelSlot = CancelSlot::new();
// Separate slot from ACTIVE_CANCEL on purpose: that one doubles as the
// "compression already running" re-entrancy guard, and an audio extraction
// must not block a video compression (or vice versa).
static AUDIO_CANCEL: CancelSlot = CancelSlot::new();

#[tauri::command]
pub async fn compress_video(
    app: AppHandle,
    input: String,
    preset: video::VideoPreset,
    output_dir: Option<String>,
    privacy: video::PrivacyMode,
) -> Result<video::VideoCompressionResult, String> {
    let cancel_flag = ACTIVE_CANCEL.start("video compression already running")?;
    let cancel_for_worker = cancel_flag.clone();

    let join_result = tauri::async_runtime::spawn_blocking(move || {
        video::compress_video(
            &input,
            preset,
            output_dir.as_deref(),
            privacy,
            cancel_for_worker,
            move |progress| {
                let _ = app.emit("video-compress-progress", &progress);
            },
        )
    })
    .await;

    ACTIVE_CANCEL.finish(&cancel_flag)?;
    join_result.map_err(|e| e.to_string())?
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
pub async fn trim_video(
    input: String,
    start_sec: f64,
    end_sec: Option<f64>,
    output_dir: Option<String>,
) -> Result<video::VideoTrimResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        video::trim_video(&input, start_sec, end_sec, output_dir.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn extract_audio(
    app: AppHandle,
    input: String,
    output_dir: Option<String>,
    format: String,
    bitrate_kbps: u32,
) -> Result<video::AudioExtractResult, String> {
    let format = match format.as_str() {
        "auto" => video::AudioExtractFormat::Auto,
        "mp3" => video::AudioExtractFormat::Mp3,
        other => return Err(format!("unknown audio format: {other}")),
    };
    let cancel_flag = AUDIO_CANCEL.start("audio extraction already running")?;
    let cancel_for_worker = cancel_flag.clone();

    let join_result = tauri::async_runtime::spawn_blocking(move || {
        video::extract_audio(
            &input,
            output_dir.as_deref(),
            video::AudioExtractOptions {
                format,
                bitrate_kbps,
            },
            cancel_for_worker,
            move |progress| {
                let _ = app.emit("audio-extract-progress", &progress);
            },
        )
    })
    .await;

    AUDIO_CANCEL.finish(&cancel_flag)?;
    join_result.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cancel_audio_extract() -> Result<(), String> {
    AUDIO_CANCEL.cancel()
}

#[tauri::command]
pub async fn cancel_video_compress() -> Result<(), String> {
    ACTIVE_CANCEL.cancel()
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
