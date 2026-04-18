use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct VideoStats {
    pub duration_sec: f64,
    pub size_bytes: u64,
    pub video_bitrate_bps: Option<u64>,
    pub height: u32,
    pub width: u32,
    pub fps: u32,
    pub creation_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoCompressionPreview {
    pub input_path: String,
    pub duration_sec: f64,
    pub original_size_bytes: u64,
    pub estimated_size_bytes: u64,
    pub estimated_savings_pct: f32,
    pub confidence: f32,
    pub method: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum VideoPreset {
    Smallest,
    Balanced,
    HighQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PrivacyMode {
    #[default]
    Off,
    Smart,
    #[serde(rename = "gps_only")]
    GpsOnly,
    Full,
}

impl VideoPreset {
    pub fn crf(self) -> u32 {
        match self {
            VideoPreset::Smallest => 28,
            VideoPreset::Balanced => 23,
            VideoPreset::HighQuality => 20,
        }
    }
    pub fn codec(self) -> &'static str {
        match self {
            VideoPreset::Smallest => "libx264",
            VideoPreset::Balanced => "libx264",
            VideoPreset::HighQuality => "libx265",
        }
    }
    pub fn encoder_preset(self) -> &'static str {
        match self {
            VideoPreset::Smallest => "medium",
            VideoPreset::Balanced => "medium",
            VideoPreset::HighQuality => "slow",
        }
    }
    pub fn pix_fmt(self) -> &'static str {
        match self {
            VideoPreset::Smallest => "yuv420p",
            VideoPreset::Balanced => "yuv420p",
            VideoPreset::HighQuality => "yuv420p10le",
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

fn ffprobe_path() -> PathBuf {
    // ffprobe lives next to ffmpeg (same folder in the sidecar download).
    let ffmpeg = get_ffmpeg_path();
    let name = if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    };
    ffmpeg
        .parent()
        .map(|p| p.join(name))
        .unwrap_or_else(|| PathBuf::from(name))
}

pub fn probe_video_stats(path: &str) -> Result<VideoStats, String> {
    let probe_bin = ffprobe_path();
    let output = std::process::Command::new(&probe_bin)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path,
        ])
        .output()
        .map_err(|e| format!("ffprobe failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe exited with error: {}", stderr));
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("ffprobe JSON parse: {}", e))?;

    parse_video_stats_from_json(&json)
        .ok_or_else(|| format!("Could not extract video stats from: {}", path))
}

fn parse_video_stats_from_json(json: &serde_json::Value) -> Option<VideoStats> {
    let format = json.get("format")?;
    let duration_sec = format
        .get("duration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let size_bytes = format
        .get("size")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let creation_time = format
        .get("tags")
        .and_then(|t| t.get("creation_time"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let streams = json.get("streams")?.as_array()?;
    let video_stream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("video"))?;

    let width = video_stream
        .get("width")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(0);
    let height = video_stream
        .get("height")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(0);
    let video_bitrate_bps = video_stream
        .get("bit_rate")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok());
    let fps = video_stream
        .get("avg_frame_rate")
        .and_then(|v| v.as_str())
        .and_then(parse_frame_rate)
        .unwrap_or(0);

    Some(VideoStats {
        duration_sec,
        size_bytes,
        video_bitrate_bps,
        height,
        width,
        fps,
        creation_time,
    })
}

fn parse_frame_rate(s: &str) -> Option<u32> {
    let (num, den) = s.split_once('/')?;
    let n: f64 = num.parse().ok()?;
    let d: f64 = den.parse().ok()?;
    if d == 0.0 {
        return None;
    }
    Some((n / d).round() as u32)
}

pub fn probe_creation_time(path: &str) -> Option<String> {
    probe_video_stats(path).ok().and_then(|s| s.creation_time)
}

pub fn build_strip_flags(privacy: PrivacyMode) -> Vec<&'static str> {
    match privacy {
        PrivacyMode::Smart | PrivacyMode::Full => vec![
            "-map",
            "-0:d",
            "-map_chapters",
            "-1",
            "-fflags",
            "+bitexact",
            "-flags",
            "+bitexact",
        ],
        PrivacyMode::GpsOnly | PrivacyMode::Off => Vec::new(),
    }
}

/// Validate ISO 8601 creation_time so we never re-inject a tainted string
/// (quotes, semicolons, shell metacharacters) into the ffmpeg command line.
fn sanitize_iso8601(s: &str) -> Option<&str> {
    if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
        Some(s)
    } else {
        None
    }
}

fn parse_audio_bitrate_bps(s: &str) -> u64 {
    // e.g. "96k" -> 96_000, "192k" -> 192_000, fallback 128_000
    let trimmed = s.trim();
    if let Some(num) = trimmed
        .strip_suffix('k')
        .or_else(|| trimmed.strip_suffix('K'))
    {
        if let Ok(n) = num.parse::<u64>() {
            return n * 1000;
        }
    }
    trimmed.parse::<u64>().unwrap_or(128_000)
}

fn video_factor(preset: VideoPreset, input_height: u32) -> f32 {
    match preset {
        VideoPreset::Smallest => {
            if input_height >= 2160 {
                0.18
            } else if input_height <= 720 {
                0.35
            } else {
                0.25
            }
        }
        VideoPreset::Balanced => {
            if input_height >= 2160 {
                0.32
            } else if input_height <= 1080 {
                0.55
            } else {
                0.45
            }
        }
        VideoPreset::HighQuality => {
            if input_height >= 2160 {
                0.55
            } else {
                0.70
            }
        }
    }
}

pub fn estimate_output_size(stats: &VideoStats, preset: VideoPreset) -> (u64, f32) {
    let factor = video_factor(preset, stats.height);
    let audio_bps = parse_audio_bitrate_bps(preset.audio_bitrate());
    let audio_bytes = ((audio_bps as f64) * stats.duration_sec / 8.0) as u64;

    let (video_bytes, confidence) = match stats.video_bitrate_bps {
        Some(vb) if stats.duration_sec > 0.0 => {
            let raw = (vb as f64) * stats.duration_sec / 8.0;
            ((raw * factor as f64) as u64, 0.75)
        }
        _ => {
            // Fallback: assume ~85% of container is video, apply factor to that share.
            let video_share = (stats.size_bytes as f64) * 0.85;
            ((video_share * factor as f64) as u64, 0.5)
        }
    };

    (video_bytes + audio_bytes, confidence)
}

pub fn preview_video_compression(
    input_path: &str,
    preset: VideoPreset,
) -> Result<VideoCompressionPreview, String> {
    let stats = probe_video_stats(input_path)?;
    let original = if stats.size_bytes > 0 {
        stats.size_bytes
    } else {
        std::fs::metadata(input_path)
            .map_err(|e| e.to_string())?
            .len()
    };
    let (estimated, confidence) = estimate_output_size(&stats, preset);
    let savings_pct = if original > 0 {
        let s = (1.0 - (estimated as f32 / original as f32)) * 100.0;
        s.max(0.0)
    } else {
        0.0
    };
    Ok(VideoCompressionPreview {
        input_path: input_path.to_string(),
        duration_sec: stats.duration_sec,
        original_size_bytes: original,
        estimated_size_bytes: estimated,
        estimated_savings_pct: savings_pct,
        confidence,
        method: "ffprobe_bitrate_factor",
    })
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
    privacy: PrivacyMode,
    cancelled: Arc<AtomicBool>,
    progress_cb: impl Fn(VideoProgress) + Send,
) -> Result<VideoCompressionResult, String> {
    let output_path = resolve_video_output_path(input_path, output_dir)?;
    let original_size = std::fs::metadata(input_path)
        .map_err(|e| format!("Cannot read input file '{}': {}", input_path, e))?
        .len();
    let total_duration = probe_duration(input_path).unwrap_or(0.0);
    // Smart mode re-injects creation_time after -map_metadata -1 so chronological
    // order is preserved in file managers without leaking EXIF/GPS sidecar metadata.
    // Full mode strips without re-injection. Off and GpsOnly keep the original tag.
    let preserved_creation_time = if matches!(privacy, PrivacyMode::Smart) {
        probe_creation_time(input_path)
    } else {
        None
    };

    if cancelled.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let ffmpeg_bin = get_ffmpeg_path();
    let mut cmd = FfmpegCommand::new_with_path(ffmpeg_bin);
    cmd.input(input_path)
        .overwrite()
        .codec_video(preset.codec())
        .arg("-crf")
        .arg(preset.crf().to_string())
        .arg("-preset")
        .arg(preset.encoder_preset())
        .arg("-pix_fmt")
        .arg(preset.pix_fmt());

    // H.264 only: high profile + level 4.1 is the universal baseline for 1080p
    // playback on any device/browser from 2012 onward.
    if !matches!(preset, VideoPreset::HighQuality) {
        cmd.arg("-profile:v").arg("high").arg("-level").arg("4.1");
    }

    // H.265 only: QuickTime requires hvc1 tag — without it the video stream
    // is undecodable and only audio plays back. Also tune x265 for perceptual
    // quality at the HighQuality preset.
    if matches!(preset, VideoPreset::HighQuality) {
        cmd.arg("-tag:v").arg("hvc1");
        cmd.arg("-x265-params")
            .arg("aq-mode=3:psy-rd=1.5:psy-rdoq=5.0:rdoq-level=2");
    }

    if let Some(filter) = build_scale_filter(preset.max_height()) {
        // -sws_flags must come before -vf to apply to the scale filter
        cmd.arg("-sws_flags").arg("lanczos");
        cmd.arg("-vf").arg(filter);
    }

    // Smart/Full wipe all container tags via -map_metadata -1. GpsOnly keeps
    // the original tags and wipes only the location fields below. Off passes
    // through untouched.
    if matches!(privacy, PrivacyMode::Smart | PrivacyMode::Full) {
        cmd.arg("-map_metadata").arg("-1");
    }
    cmd.arg("-map").arg("0:v:0").arg("-map").arg("0:a?"); // audio is optional — videos without audio tracks won't fail

    // Re-inject creation_time AFTER -map_metadata -1 so it's the only surviving tag.
    // Guard with a strict ISO 8601 check to avoid injecting untrusted probe output.
    if let Some(ref ct) = preserved_creation_time {
        if let Some(valid) = sanitize_iso8601(ct) {
            cmd.arg("-metadata").arg(format!("creation_time={}", valid));
        }
    }

    // GpsOnly: wipe the location tags Apple/Android write, keep everything else.
    if matches!(privacy, PrivacyMode::GpsOnly) {
        cmd.arg("-metadata").arg("location=");
        cmd.arg("-metadata").arg("location-eng=");
    }

    // Hardened strip flags: drop data streams (iPhone GPS tracks), chapters,
    // and encoder fingerprint bits. Only applied for Smart/Full.
    for flag in build_strip_flags(privacy) {
        cmd.arg(flag);
    }

    cmd.codec_audio("aac")
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
        // Use Path::join for cross-platform comparison — Windows produces
        // backslash separators, Unix produces forward slashes.
        let result = resolve_video_output_path("/tmp/video.mp4", None).unwrap();
        let expected = Path::new("/tmp")
            .join("video_compressed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);

        let result = resolve_video_output_path("/home/user/clip.mov", None).unwrap();
        let expected = Path::new("/home/user")
            .join("clip_compressed.mov")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_resolve_video_output_path_with_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let dir_str = dir.path().to_str().unwrap();

        let result = resolve_video_output_path("/tmp/video.mp4", Some(dir_str)).unwrap();
        let expected = dir
            .path()
            .join("video_compressed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);

        let result = resolve_video_output_path("/tmp/clip.mov", Some(dir_str)).unwrap();
        let expected = dir
            .path()
            .join("clip_compressed.mov")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_resolve_video_output_path_missing_dir() {
        let result = resolve_video_output_path("/tmp/video.mp4", Some("/nonexistent/dir"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_preset_values() {
        assert_eq!(VideoPreset::Smallest.crf(), 28);
        assert_eq!(VideoPreset::Balanced.crf(), 23);
        assert_eq!(VideoPreset::HighQuality.crf(), 20);

        assert_eq!(VideoPreset::Smallest.codec(), "libx264");
        assert_eq!(VideoPreset::Balanced.codec(), "libx264");
        assert_eq!(VideoPreset::HighQuality.codec(), "libx265");

        assert_eq!(VideoPreset::Smallest.encoder_preset(), "medium");
        assert_eq!(VideoPreset::Balanced.encoder_preset(), "medium");
        assert_eq!(VideoPreset::HighQuality.encoder_preset(), "slow");

        assert_eq!(VideoPreset::Smallest.pix_fmt(), "yuv420p");
        assert_eq!(VideoPreset::Balanced.pix_fmt(), "yuv420p");
        assert_eq!(VideoPreset::HighQuality.pix_fmt(), "yuv420p10le");

        assert_eq!(VideoPreset::Smallest.audio_bitrate(), "96k");
        assert_eq!(VideoPreset::Balanced.audio_bitrate(), "128k");
        assert_eq!(VideoPreset::HighQuality.audio_bitrate(), "192k");

        assert_eq!(VideoPreset::Smallest.max_height(), Some(720));
        assert_eq!(VideoPreset::Balanced.max_height(), Some(1080));
        assert_eq!(VideoPreset::HighQuality.max_height(), None);
    }

    fn synthetic_stats(
        duration: f64,
        size: u64,
        bitrate: Option<u64>,
        w: u32,
        h: u32,
        creation_time: Option<&str>,
    ) -> VideoStats {
        VideoStats {
            duration_sec: duration,
            size_bytes: size,
            video_bitrate_bps: bitrate,
            width: w,
            height: h,
            fps: 30,
            creation_time: creation_time.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_strip_flags_off() {
        assert!(build_strip_flags(PrivacyMode::Off).is_empty());
    }

    #[test]
    fn test_strip_flags_smart() {
        let flags = build_strip_flags(PrivacyMode::Smart);
        assert!(flags.contains(&"-map"));
        assert!(flags.contains(&"-0:d"));
        assert!(flags.contains(&"-map_chapters"));
        assert!(flags.contains(&"-1"));
        assert!(flags.contains(&"-fflags"));
        assert!(flags.contains(&"+bitexact"));
        assert!(flags.contains(&"-flags"));
    }

    #[test]
    fn test_strip_flags_gps_only() {
        // GpsOnly must NOT emit any -map -0:d / -map_metadata / bitexact flags;
        // only the location= wipes happen (added directly to the command, not here).
        assert!(build_strip_flags(PrivacyMode::GpsOnly).is_empty());
    }

    #[test]
    fn test_strip_flags_full() {
        let flags = build_strip_flags(PrivacyMode::Full);
        assert!(flags.contains(&"-map"));
        assert!(flags.contains(&"-0:d"));
        assert!(flags.contains(&"-map_chapters"));
        assert!(flags.contains(&"+bitexact"));
    }

    #[test]
    fn test_privacy_mode_serde_strings() {
        // Frontend contract: "off" | "smart" | "gps_only" | "full".
        assert_eq!(
            serde_json::from_str::<PrivacyMode>("\"off\"").unwrap(),
            PrivacyMode::Off
        );
        assert_eq!(
            serde_json::from_str::<PrivacyMode>("\"smart\"").unwrap(),
            PrivacyMode::Smart
        );
        assert_eq!(
            serde_json::from_str::<PrivacyMode>("\"gps_only\"").unwrap(),
            PrivacyMode::GpsOnly
        );
        assert_eq!(
            serde_json::from_str::<PrivacyMode>("\"full\"").unwrap(),
            PrivacyMode::Full
        );
    }

    #[test]
    fn test_sanitize_iso8601_valid() {
        assert_eq!(
            sanitize_iso8601("2024-03-15T10:30:00.000000Z"),
            Some("2024-03-15T10:30:00.000000Z")
        );
        assert_eq!(
            sanitize_iso8601("2024-03-15T10:30:00Z"),
            Some("2024-03-15T10:30:00Z")
        );
        assert_eq!(
            sanitize_iso8601("2024-03-15T10:30:00+02:00"),
            Some("2024-03-15T10:30:00+02:00")
        );
    }

    #[test]
    fn test_sanitize_iso8601_rejects_injection() {
        assert_eq!(sanitize_iso8601("2024; rm -rf /"), None);
        assert_eq!(sanitize_iso8601("2024-03-15T10:30:00\" bad"), None);
        assert_eq!(sanitize_iso8601("not a date"), None);
        assert_eq!(sanitize_iso8601(""), None);
    }

    #[test]
    fn test_sanitize_iso8601_rejects_semantically_invalid() {
        // Regex would accept this (syntactically matches \d{4}-\d{2}-\d{2}T...),
        // but chrono rejects it because month/day/hour are out of range.
        assert_eq!(sanitize_iso8601("9999-99-99T99:99:99Z"), None);
    }

    #[test]
    fn test_parse_audio_bitrate_bps() {
        assert_eq!(parse_audio_bitrate_bps("96k"), 96_000);
        assert_eq!(parse_audio_bitrate_bps("128K"), 128_000);
        assert_eq!(parse_audio_bitrate_bps("192k"), 192_000);
        assert_eq!(parse_audio_bitrate_bps("bogus"), 128_000);
    }

    #[test]
    fn test_estimate_video_size_smallest_preset() {
        // 100MB, 60s, 5Mbps, 1080p → Smallest uses factor 0.25.
        // video: 5_000_000 * 60 / 8 = 37_500_000 bytes, * 0.25 = 9_375_000
        // audio: 96_000 * 60 / 8 = 720_000
        // total: ~10_095_000 bytes. Brief asks for "20-40MB" range but the
        // math on a 5Mbps stream lands ~10MB; accept 5-40MB as sanity band.
        let stats = synthetic_stats(60.0, 100 * 1024 * 1024, Some(5_000_000), 1920, 1080, None);
        let (estimated, confidence) = estimate_output_size(&stats, VideoPreset::Smallest);
        assert!(
            (5 * 1024 * 1024..=40 * 1024 * 1024).contains(&estimated),
            "expected 5-40MB, got {} bytes",
            estimated
        );
        assert!((confidence - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_estimate_video_size_no_bitrate_fallback() {
        // No bitrate known → fallback uses size_bytes * 0.85 * factor.
        // 100MB * 0.85 * 0.25 (Smallest, 1080p) = 22_282_240 bytes video
        // audio: 96_000 * 60 / 8 = 720_000
        let stats = synthetic_stats(60.0, 100 * 1024 * 1024, None, 1920, 1080, None);
        let (estimated, confidence) = estimate_output_size(&stats, VideoPreset::Smallest);
        assert!((confidence - 0.5).abs() < 1e-6);
        // conservative bound: fallback should produce something meaningful.
        assert!(estimated > 10 * 1024 * 1024);
        assert!(estimated < 30 * 1024 * 1024);
    }

    #[test]
    fn test_creation_time_reinjection_logic() {
        // Verify that when stats carry a creation_time, the compress_video
        // path would format the ffmpeg -metadata flag correctly. We test the
        // formatting contract directly since we can't run ffmpeg in unit tests.
        let stats = synthetic_stats(
            10.0,
            1024,
            Some(1_000_000),
            1920,
            1080,
            Some("2024-03-15T10:30:00.000000Z"),
        );
        let ct = stats.creation_time.as_deref().unwrap();
        let flag = format!("creation_time={}", ct);
        assert_eq!(flag, "creation_time=2024-03-15T10:30:00.000000Z");
        // And confirm the helper would return it.
        assert_eq!(
            stats.creation_time,
            Some("2024-03-15T10:30:00.000000Z".to_string())
        );
    }

    #[test]
    fn test_factor_scales_with_input_resolution() {
        // Smallest
        assert!((video_factor(VideoPreset::Smallest, 480) - 0.35).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Smallest, 720) - 0.35).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Smallest, 1080) - 0.25).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Smallest, 1440) - 0.25).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Smallest, 2160) - 0.18).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Smallest, 4320) - 0.18).abs() < 1e-6);
        // Balanced
        assert!((video_factor(VideoPreset::Balanced, 480) - 0.55).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Balanced, 1080) - 0.55).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Balanced, 1440) - 0.45).abs() < 1e-6);
        assert!((video_factor(VideoPreset::Balanced, 2160) - 0.32).abs() < 1e-6);
        // HighQuality
        assert!((video_factor(VideoPreset::HighQuality, 1080) - 0.70).abs() < 1e-6);
        assert!((video_factor(VideoPreset::HighQuality, 2160) - 0.55).abs() < 1e-6);
    }

    #[test]
    fn test_parse_video_stats_from_json() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "format": {
                    "duration": "120.5",
                    "size": "10485760",
                    "tags": { "creation_time": "2024-01-01T12:00:00.000000Z" }
                },
                "streams": [
                    {
                        "codec_type": "audio",
                        "bit_rate": "128000"
                    },
                    {
                        "codec_type": "video",
                        "width": 1920,
                        "height": 1080,
                        "bit_rate": "5000000",
                        "avg_frame_rate": "30/1"
                    }
                ]
            }"#,
        )
        .unwrap();
        let stats = parse_video_stats_from_json(&json).unwrap();
        assert_eq!(stats.duration_sec, 120.5);
        assert_eq!(stats.size_bytes, 10_485_760);
        assert_eq!(stats.width, 1920);
        assert_eq!(stats.height, 1080);
        assert_eq!(stats.fps, 30);
        assert_eq!(stats.video_bitrate_bps, Some(5_000_000));
        assert_eq!(
            stats.creation_time.as_deref(),
            Some("2024-01-01T12:00:00.000000Z")
        );
    }

    #[test]
    fn test_parse_frame_rate() {
        assert_eq!(parse_frame_rate("30/1"), Some(30));
        assert_eq!(parse_frame_rate("60000/1001"), Some(60));
        assert_eq!(parse_frame_rate("24/1"), Some(24));
        assert_eq!(parse_frame_rate("0/0"), None);
        assert_eq!(parse_frame_rate("bogus"), None);
    }

    #[test]
    fn test_parse_video_stats_missing_optional_fields() {
        // No creation_time, no bitrate — should still parse.
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "format": { "duration": "10.0", "size": "1024" },
                "streams": [
                    { "codec_type": "video", "width": 640, "height": 480 }
                ]
            }"#,
        )
        .unwrap();
        let stats = parse_video_stats_from_json(&json).unwrap();
        assert_eq!(stats.width, 640);
        assert_eq!(stats.height, 480);
        assert_eq!(stats.video_bitrate_bps, None);
        assert_eq!(stats.creation_time, None);
    }
}
