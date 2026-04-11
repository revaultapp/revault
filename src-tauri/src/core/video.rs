use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum VideoPreset {
    Email,
    Web,
    Archive,
    HighQuality,
}

impl VideoPreset {
    pub fn codec(self) -> &'static str {
        match self {
            VideoPreset::Archive => "libx265",
            _ => "libx264",
        }
    }
    pub fn crf(self) -> u32 {
        match self {
            VideoPreset::Email => 28,
            VideoPreset::Web => 23,
            VideoPreset::Archive => 20,
            VideoPreset::HighQuality => 20,
        }
    }
    pub fn speed_preset(self) -> &'static str {
        match self {
            VideoPreset::Email => "fast",
            VideoPreset::Web => "medium",
            VideoPreset::Archive => "slow",
            VideoPreset::HighQuality => "medium",
        }
    }
    pub fn audio_bitrate(self) -> &'static str {
        match self {
            VideoPreset::Email => "96k",
            VideoPreset::Web => "128k",
            VideoPreset::Archive => "192k",
            VideoPreset::HighQuality => "192k",
        }
    }
    pub fn max_height(self) -> Option<u32> {
        match self {
            VideoPreset::Email => Some(720),
            VideoPreset::Web => Some(1080),
            VideoPreset::Archive => None,
            VideoPreset::HighQuality => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoProgress {
    pub input_path: String,
    pub percent: f32,
    pub fps: f32,
    pub size_kb: u32,
    pub speed: f32,
}

#[derive(Debug, Serialize)]
pub struct VideoCompressionResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub error: Option<String>,
}

pub fn get_ffmpeg_path() -> PathBuf {
    ffmpeg_sidecar::paths::ffmpeg_path()
}

pub fn probe_duration(path: &str) -> Result<f64, String> {
    let output = std::process::Command::new(get_ffmpeg_path())
        .args(["-i", path, "-hide_banner"])
        .output()
        .map_err(|e| e.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_duration_from_stderr(&stderr)
        .ok_or_else(|| format!("Could not parse duration from: {}", path))
}

fn parse_duration_from_stderr(stderr: &str) -> Option<f64> {
    let line = stderr.lines().find(|l| l.contains("Duration:"))?;
    let start = line.find("Duration:")? + "Duration:".len();
    let substr = line[start..].trim();
    let end = substr.find(',')?;
    let time_str = substr[..end].trim();
    parse_time_to_secs(time_str)
}

fn parse_time_to_secs(time: &str) -> Option<f64> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

pub fn build_scale_filter(max_height: Option<u32>) -> Option<String> {
    max_height.map(|h| format!("scale=-2:{}", h))
}

pub fn resolve_video_output_path(input_path: &str) -> Result<String, String> {
    let path = Path::new(input_path);
    let parent = path.parent().ok_or("No parent directory")?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;
    let output = parent.join(format!("{}_compressed.mp4", stem));
    output
        .to_str()
        .map(|s| s.to_string())
        .ok_or("Invalid path".to_string())
}

pub fn compress_video(
    input_path: &str,
    preset: VideoPreset,
    cancelled: Arc<AtomicBool>,
    progress_cb: impl Fn(VideoProgress) + Send,
) -> Result<VideoCompressionResult, String> {
    let output_path = resolve_video_output_path(input_path)?;
    let original_size = std::fs::metadata(input_path)
        .map_err(|e| e.to_string())?
        .len();
    let total_duration = probe_duration(input_path).unwrap_or(0.0);

    if cancelled.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let mut cmd = FfmpegCommand::new();
    cmd.input(input_path)
        .overwrite()
        .codec_video(preset.codec())
        .arg("-crf")
        .arg(preset.crf().to_string())
        .arg("-preset")
        .arg(preset.speed_preset())
        .arg("-pix_fmt")
        .arg("yuv420p");

    if let Some(filter) = build_scale_filter(preset.max_height()) {
        cmd.arg("-vf").arg(filter);
    }

    cmd.arg("-map_metadata")
        .arg("-1")
        .codec_audio("aac")
        .arg("-b:a")
        .arg(preset.audio_bitrate())
        .arg("-movflags")
        .arg("+faststart")
        .output(&output_path);

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    for event in child.iter().map_err(|e| e.to_string())? {
        if let FfmpegEvent::Progress(p) = event {
            let current_secs = parse_time_to_secs(&p.time).unwrap_or(0.0);
            let percent = if total_duration > 0.0 {
                ((current_secs / total_duration) * 100.0).min(100.0) as f32
            } else {
                0.0
            };
            progress_cb(VideoProgress {
                input_path: input_path.to_string(),
                percent,
                fps: p.fps,
                size_kb: p.size_kb,
                speed: p.speed,
            });
        }
        if cancelled.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = std::fs::remove_file(&output_path);
            return Err("cancelled".to_string());
        }
    }

    let compressed_size = std::fs::metadata(&output_path)
        .map_err(|e| e.to_string())?
        .len();

    Ok(VideoCompressionResult {
        input_path: input_path.to_string(),
        output_path,
        original_size,
        compressed_size,
        error: None,
    })
}

pub fn ffmpeg_is_available() -> bool {
    ffmpeg_sidecar::command::ffmpeg_is_installed()
}

pub fn download_ffmpeg(progress_cb: impl Fn(u64, u64) + Send + 'static) -> Result<PathBuf, String> {
    use ffmpeg_sidecar::download::{auto_download_with_progress, FfmpegDownloadProgressEvent};

    std::env::set_var("KEEP_ONLY_FFMPEG", "1");

    auto_download_with_progress(move |event| {
        if let FfmpegDownloadProgressEvent::Downloading {
            total_bytes,
            downloaded_bytes,
        } = event
        {
            progress_cb(downloaded_bytes, total_bytes);
        }
    })
    .map_err(|e| format!("FFmpeg download failed: {}", e))?;

    let path = get_ffmpeg_path();
    if !path.exists() {
        return Err("FFmpeg binary not found after download".into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_to_secs() {
        assert_eq!(parse_time_to_secs("01:30:15.50"), Some(5415.5));
        assert_eq!(parse_time_to_secs("00:00:00.00"), Some(0.0));
        assert_eq!(parse_time_to_secs("00:01:00.00"), Some(60.0));
        assert_eq!(parse_time_to_secs("invalid"), None);
        assert_eq!(parse_time_to_secs("00:00"), None);
    }

    #[test]
    fn test_parse_duration_from_stderr() {
        let stderr = "  Duration: 00:02:30.00, start: 0.000000, bitrate: 1234 kb/s";
        assert_eq!(parse_duration_from_stderr(stderr), Some(150.0));

        assert_eq!(parse_duration_from_stderr("no duration here"), None);
    }

    #[test]
    fn test_build_scale_filter() {
        assert_eq!(
            build_scale_filter(Some(720)),
            Some("scale=-2:720".to_string())
        );
        assert_eq!(build_scale_filter(None), None);
    }

    #[test]
    fn test_resolve_video_output_path() {
        let result = resolve_video_output_path("/tmp/video.mp4").unwrap();
        assert_eq!(result, "/tmp/video_compressed.mp4");

        let result = resolve_video_output_path("/home/user/clip.mov").unwrap();
        assert_eq!(result, "/home/user/clip_compressed.mp4");
    }

    #[test]
    fn test_preset_values() {
        assert_eq!(VideoPreset::Email.codec(), "libx264");
        assert_eq!(VideoPreset::Archive.codec(), "libx265");
        assert_eq!(VideoPreset::Email.crf(), 28);
        assert_eq!(VideoPreset::Web.crf(), 23);
        assert_eq!(VideoPreset::Email.max_height(), Some(720));
        assert_eq!(VideoPreset::Web.max_height(), Some(1080));
        assert_eq!(VideoPreset::Archive.max_height(), None);
        assert_eq!(VideoPreset::HighQuality.max_height(), None);
        assert_eq!(VideoPreset::Email.audio_bitrate(), "96k");
        assert_eq!(VideoPreset::Archive.speed_preset(), "slow");
    }
}
