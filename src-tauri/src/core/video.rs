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
    Smallest,
    Balanced,
    HighQuality,
}

impl VideoPreset {
    pub fn crf(self) -> u32 {
        match self {
            VideoPreset::Smallest => 35,
            VideoPreset::Balanced => 28,
            VideoPreset::HighQuality => 22,
        }
    }
    pub fn audio_bitrate(self) -> &'static str {
        match self {
            VideoPreset::Smallest => "96k",
            VideoPreset::Balanced => "128k",
            VideoPreset::HighQuality => "192k",
        }
    }
    pub fn max_height(self) -> Option<u32> {
        match self {
            VideoPreset::Smallest => Some(720),
            VideoPreset::Balanced => Some(1080),
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

fn app_support_ffmpeg_path() -> Option<PathBuf> {
    let data_dir = dirs::data_dir()?;
    let ffmpeg_dir = data_dir.join("com.revault.desktop");
    let binary = ffmpeg_dir.join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });
    Some(binary)
}

pub fn get_ffmpeg_path() -> PathBuf {
    if let Some(path) = app_support_ffmpeg_path() {
        if path.exists() {
            return path;
        }
    }
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
    if time_str == "N/A" {
        return None;
    }
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
    // Note: lanczos is applied via -sws_flags in the caller, not here.
    // Embedding :flags= inside the filter chain fails on some FFmpeg builds.
    // The comma inside min() must be escaped as \, so FFmpeg's filter chain
    // parser doesn't treat it as a filter separator.
    max_height.map(|h| format!("scale=-2:min({}\\,ih)", h))
}

pub fn resolve_video_output_path(
    input_path: &str,
    output_dir: Option<&str>,
) -> Result<String, String> {
    let path = Path::new(input_path);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let dir = match output_dir {
        Some(d) => {
            let p = Path::new(d);
            if !p.is_dir() {
                return Err(format!("Output directory does not exist: {}", d));
            }
            p.to_path_buf()
        }
        None => path.parent().ok_or("No parent directory")?.to_path_buf(),
    };
    let output = dir.join(format!("{}_compressed.{}", stem, ext));
    output
        .to_str()
        .map(|s| s.to_string())
        .ok_or("Invalid path".to_string())
}

pub fn compress_video(
    input_path: &str,
    preset: VideoPreset,
    output_dir: Option<&str>,
    cancelled: Arc<AtomicBool>,
    progress_cb: impl Fn(VideoProgress) + Send,
) -> Result<VideoCompressionResult, String> {
    let output_path = resolve_video_output_path(input_path, output_dir)?;
    let original_size = std::fs::metadata(input_path)
        .map_err(|e| format!("Cannot read input file '{}': {}", input_path, e))?
        .len();
    let total_duration = probe_duration(input_path).unwrap_or(0.0);

    if cancelled.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let ffmpeg_bin = get_ffmpeg_path();
    let mut cmd = FfmpegCommand::new_with_path(ffmpeg_bin);
    cmd.input(input_path)
        .overwrite()
        .codec_video("libx265")
        .arg("-crf")
        .arg(preset.crf().to_string())
        .arg("-preset")
        .arg("slow")
        .arg("-pix_fmt")
        .arg("yuv420p");

    // QuickTime requires hvc1 tag for H.265 — without it the video stream
    // is undecodable and only audio plays back.
    cmd.arg("-tag:v").arg("hvc1");

    if let Some(filter) = build_scale_filter(preset.max_height()) {
        // -sws_flags must come before -vf to apply to the scale filter
        cmd.arg("-sws_flags").arg("lanczos");
        cmd.arg("-vf").arg(filter);
    }

    cmd.arg("-map_metadata")
        .arg("-1")
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("0:a?") // audio is optional — videos without audio tracks won't fail
        .codec_audio("aac")
        .arg("-b:a")
        .arg(preset.audio_bitrate())
        .arg("-movflags")
        .arg("+faststart")
        .output(&output_path);

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    let encode_result: Result<(), String> = (|| {
        let mut ffmpeg_errors: Vec<String> = Vec::new();
        for event in child.iter().map_err(|e| e.to_string())? {
            match event {
                FfmpegEvent::Progress(p) => {
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
                FfmpegEvent::Log(level, msg) => {
                    use ffmpeg_sidecar::event::LogLevel;
                    if matches!(level, LogLevel::Fatal | LogLevel::Error) {
                        ffmpeg_errors.push(msg);
                    }
                }
                _ => {}
            }
            if cancelled.load(Ordering::SeqCst) {
                let _ = child.kill();
                let _ = std::fs::remove_file(&output_path);
                return Err("cancelled".to_string());
            }
        }
        if !ffmpeg_errors.is_empty() {
            return Err(ffmpeg_errors.join("; "));
        }
        Ok(())
    })();

    if let Err(e) = encode_result {
        let _ = std::fs::remove_file(&output_path);
        return Err(e);
    }

    let compressed_size = std::fs::metadata(&output_path)
        .map_err(|e| format!("FFmpeg produced no output — encoding likely failed: {}", e))?
        .len();

    if compressed_size >= original_size {
        std::fs::remove_file(&output_path).map_err(|e| e.to_string())?;
        std::fs::copy(input_path, &output_path).map_err(|e| e.to_string())?;
        return Ok(VideoCompressionResult {
            input_path: input_path.to_string(),
            output_path,
            original_size,
            compressed_size: original_size,
            error: Some("Output was larger — original copied".to_string()),
        });
    }

    Ok(VideoCompressionResult {
        input_path: input_path.to_string(),
        output_path,
        original_size,
        compressed_size,
        error: None,
    })
}

pub fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    let canonical =
        std::fs::canonicalize(path).map_err(|e| format!("Invalid path '{}': {}", path, e))?;
    let parent = Path::new(path)
        .parent()
        .ok_or("No parent directory")?
        .to_path_buf();
    let parent_canonical = std::fs::canonicalize(&parent)
        .map_err(|e| format!("Invalid parent '{}': {}", parent.display(), e))?;
    if !canonical.starts_with(&parent_canonical) {
        return Err("Path resolves outside allowed directory".into());
    }
    let canonical_str = canonical
        .to_str()
        .ok_or_else(|| "Path contains invalid UTF-8".to_string())?;

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", canonical_str])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", canonical_str])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let dir = canonical
            .parent()
            .unwrap_or(&canonical)
            .to_str()
            .unwrap_or(canonical_str);
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn ffmpeg_is_available() -> bool {
    let path = get_ffmpeg_path();
    std::process::Command::new(path)
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn download_ffmpeg(progress_cb: impl Fn(u64, u64) + Send + 'static) -> Result<PathBuf, String> {
    use ffmpeg_sidecar::download::{
        download_ffmpeg_package_with_progress, ffmpeg_download_url, unpack_ffmpeg_without_extras,
        FfmpegDownloadProgressEvent,
    };

    if ffmpeg_is_available() {
        return Ok(get_ffmpeg_path());
    }

    let dest = app_support_ffmpeg_path()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .ok_or("Could not determine application data directory")?;

    std::fs::create_dir_all(&dest).map_err(|e| format!("Failed to create ffmpeg dir: {}", e))?;

    let url = ffmpeg_download_url().map_err(|e| format!("Unsupported platform: {}", e))?;

    let archive_path = download_ffmpeg_package_with_progress(url, &dest, move |event| {
        if let FfmpegDownloadProgressEvent::Downloading {
            total_bytes,
            downloaded_bytes,
        } = event
        {
            progress_cb(downloaded_bytes, total_bytes);
        }
    })
    .map_err(|e| format!("FFmpeg download failed: {}", e))?;

    unpack_ffmpeg_without_extras(&archive_path, &dest)
        .map_err(|e| format!("FFmpeg unpack failed: {}", e))?;

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

        let na_stderr = "  Duration: N/A, start: 0.000000, bitrate: N/A";
        assert_eq!(parse_duration_from_stderr(na_stderr), None);
    }

    #[test]
    fn test_build_scale_filter() {
        assert_eq!(
            build_scale_filter(Some(720)),
            Some("scale=-2:min(720\\,ih)".to_string())
        );
        assert_eq!(
            build_scale_filter(Some(1080)),
            Some("scale=-2:min(1080\\,ih)".to_string())
        );
        assert_eq!(build_scale_filter(None), None);
    }

    #[test]
    fn test_resolve_video_output_path() {
        let result = resolve_video_output_path("/tmp/video.mp4", None).unwrap();
        assert_eq!(result, "/tmp/video_compressed.mp4");

        let result = resolve_video_output_path("/home/user/clip.mov", None).unwrap();
        assert_eq!(result, "/home/user/clip_compressed.mov");
    }

    #[test]
    fn test_resolve_video_output_path_with_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let dir_str = dir.path().to_str().unwrap();

        let result = resolve_video_output_path("/tmp/video.mp4", Some(dir_str)).unwrap();
        assert_eq!(result, format!("{}/video_compressed.mp4", dir_str));

        let result = resolve_video_output_path("/tmp/clip.mov", Some(dir_str)).unwrap();
        assert_eq!(result, format!("{}/clip_compressed.mov", dir_str));
    }

    #[test]
    fn test_resolve_video_output_path_missing_dir() {
        let result = resolve_video_output_path("/tmp/video.mp4", Some("/nonexistent/dir"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_preset_values() {
        assert_eq!(VideoPreset::Smallest.crf(), 35);
        assert_eq!(VideoPreset::Balanced.crf(), 28);
        assert_eq!(VideoPreset::HighQuality.crf(), 22);

        assert_eq!(VideoPreset::Smallest.audio_bitrate(), "96k");
        assert_eq!(VideoPreset::Balanced.audio_bitrate(), "128k");
        assert_eq!(VideoPreset::HighQuality.audio_bitrate(), "192k");

        assert_eq!(VideoPreset::Smallest.max_height(), Some(720));
        assert_eq!(VideoPreset::Balanced.max_height(), Some(1080));
        assert_eq!(VideoPreset::HighQuality.max_height(), None);
    }
}
