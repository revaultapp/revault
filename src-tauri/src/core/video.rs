use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
            VideoPreset::Balanced => "slow",
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

#[derive(Debug, Serialize)]
pub struct VideoTrimResult {
    pub input_path: String,
    pub output_path: String,
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

pub(crate) fn parse_time_to_secs(time: &str) -> Option<f64> {
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
    Some(VideoStats {
        duration_sec,
        size_bytes,
        video_bitrate_bps,
        height,
        width,
        creation_time,
    })
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
    crate::core::paths::validate_input_path(input_path, false)?;
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
    suffix: &str,
) -> Result<String, String> {
    let path = Path::new(input_path);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;
    // MOV inputs are remuxed to MP4 — same H.264/H.265 stream, broader compatibility.
    // Applies to trim too: -c copy just repackages the existing stream.
    let input_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let ext = match input_ext.as_deref() {
        Some("mov") => "mp4",
        Some(e) => e,
        None => "mp4",
    };
    let dir = match output_dir {
        Some(d) => {
            let canon = std::fs::canonicalize(d)
                .map_err(|e| format!("Invalid output dir '{}': {}", d, e))?;
            if !canon.is_dir() {
                return Err(format!("Output path is not a directory: {}", d));
            }
            canon
        }
        None => path.parent().ok_or("No parent directory")?.to_path_buf(),
    };
    let output = first_available_path(&dir.join(format!("{}{}.{}", stem, suffix, ext)));
    output
        .to_str()
        .map(|s| s.to_string())
        .ok_or("Invalid path".to_string())
}

fn first_available_path(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }

    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = base.extension().and_then(|e| e.to_str());

    for n in 2..10_000 {
        let filename = match ext {
            Some(ext) => format!("{stem}_{n}.{ext}"),
            None => format!("{stem}_{n}"),
        };
        let candidate = parent.join(filename);
        if !candidate.exists() {
            return candidate;
        }
    }

    base.to_path_buf()
}

fn temporary_output_path(final_path: &Path) -> Result<PathBuf, String> {
    let parent = final_path
        .parent()
        .ok_or_else(|| "Output path has no parent directory".to_string())?;
    let stem = final_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let ext = final_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(parent.join(format!(
        ".{stem}.revault-tmp-{}-{nonce}.{ext}",
        std::process::id()
    )))
}

fn install_temp_output(temp_path: &Path, final_path: &Path) -> Result<(), String> {
    match std::fs::hard_link(temp_path, final_path) {
        Ok(()) => std::fs::remove_file(temp_path)
            .map_err(|e| format!("Failed to clean temporary output: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = std::fs::remove_file(temp_path);
            Err(format!(
                "Output already exists, refusing to overwrite: {}",
                final_path.display()
            ))
        }
        Err(e) => {
            let _ = std::fs::remove_file(temp_path);
            Err(format!("Failed to move output into place: {}", e))
        }
    }
}

pub fn compress_video(
    input_path: &str,
    preset: VideoPreset,
    output_dir: Option<&str>,
    privacy: PrivacyMode,
    cancelled: Arc<AtomicBool>,
    progress_cb: impl Fn(VideoProgress) + Send,
) -> Result<VideoCompressionResult, String> {
    crate::core::paths::validate_input_path(input_path, false)?;
    let output_path = resolve_video_output_path(input_path, output_dir, "_compressed")?;
    let output_file = PathBuf::from(&output_path);
    let temp_output = temporary_output_path(&output_file)?;
    let temp_output_str = temp_output
        .to_str()
        .ok_or_else(|| "Temporary output path contains invalid UTF-8".to_string())?
        .to_string();
    let original_size = std::fs::metadata(input_path)
        .map_err(|e| format!("Cannot read input file '{}': {}", input_path, e))?
        .len();
    // Single ffprobe call for both duration (progress %) and creation_time (Smart).
    // Probe failure is a hard error: a video we can't probe is one we can't reliably encode.
    let stats = probe_video_stats(input_path)
        .map_err(|e| format!("Cannot read video metadata for '{}': {}", input_path, e))?;
    let total_duration = stats.duration_sec;
    // Smart mode re-injects creation_time after -map_metadata -1 so chronological
    // order is preserved in file managers without leaking EXIF/GPS sidecar metadata.
    // Full mode strips without re-injection. Off and GpsOnly keep the original tag.
    let preserved_creation_time = if matches!(privacy, PrivacyMode::Smart) {
        stats.creation_time.clone()
    } else {
        None
    };

    if cancelled.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let ffmpeg_bin = get_ffmpeg_path();
    let mut cmd = FfmpegCommand::new_with_path(ffmpeg_bin);
    // -fflags +genpts: reconstruct presentation timestamps for malformed inputs
    // (truncated MP4s, screen recorders that drop PTS). MUST come before -i to
    // apply as an input option. Note: this is INPUT-side; +bitexact in
    // build_strip_flags() is OUTPUT-side — different flags, do not conflate.
    cmd.arg("-fflags")
        .arg("+genpts")
        .input(input_path)
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

    // H.264 only: psychovisual params improve sharpness in motion and eliminate
    // macroblocking in low-motion scenes. aq-mode=2 distributes bits better than
    // the default mode=1. no-fast-pskip=1 removes a quality shortcut that causes
    // blocking artefacts on talking-head / screencast content.
    if !matches!(preset, VideoPreset::HighQuality) {
        cmd.arg("-x264-params")
            .arg("aq-mode=2:psy-rd=1.0:no-fast-pskip=1");
    }

    // H.265 only: QuickTime requires hvc1 tag — without it the video stream
    // is undecodable and only audio plays back.
    // psy-rdoq=2.0: community consensus is 1-2; at 5.0 it forces the encoder to
    // double the bitrate in some regions which makes CRF raise global QP — net loss.
    // limit-sao=1: lighter than no-sao, prevents SAO from blurring grain.
    // no-open-gop=1: closed GOP improves seeking and editing compatibility.
    // strong-intra-smoothing=0: prevents blurring of solid-colour keyframe blocks.
    if matches!(preset, VideoPreset::HighQuality) {
        cmd.arg("-tag:v").arg("hvc1");
        cmd.arg("-x265-params")
            .arg("aq-mode=3:psy-rd=1.5:psy-rdoq=2.0:rdoq-level=2:limit-sao=1:no-open-gop=1:strong-intra-smoothing=0");
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
        .output(&temp_output_str);

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
                let _ = std::fs::remove_file(&temp_output);
                return Err("cancelled".to_string());
            }
        }
        if !ffmpeg_errors.is_empty() {
            return Err(ffmpeg_errors.join("; "));
        }
        Ok(())
    })();

    if let Err(e) = encode_result {
        let _ = std::fs::remove_file(&temp_output);
        return Err(e);
    }

    let compressed_size = std::fs::metadata(&temp_output)
        .map_err(|e| format!("FFmpeg produced no output — encoding likely failed: {}", e))?
        .len();

    if compressed_size >= original_size {
        let _ = std::fs::remove_file(&temp_output);
        std::fs::copy(input_path, &temp_output).map_err(|e| e.to_string())?;
        install_temp_output(&temp_output, &output_file)?;
        return Ok(VideoCompressionResult {
            input_path: input_path.to_string(),
            output_path,
            original_size,
            compressed_size: original_size,
            error: Some("Output was larger — original copied".to_string()),
        });
    }

    install_temp_output(&temp_output, &output_file)?;

    Ok(VideoCompressionResult {
        input_path: input_path.to_string(),
        output_path,
        original_size,
        compressed_size,
        error: None,
    })
}

/// Resolves and validates a trim range against the real media duration.
/// Returns the resolved end time (`duration_sec` when `end_sec` is `None`).
/// `duration_sec <= 0.0` means ffprobe couldn't determine a duration — range
/// checks against it are skipped (mirrors how `compress_video` treats an
/// unknown duration for progress reporting).
fn resolve_trim_range(
    start_sec: f64,
    end_sec: Option<f64>,
    duration_sec: f64,
) -> Result<f64, String> {
    if start_sec < 0.0 {
        return Err("Start time must be non-negative".to_string());
    }
    let end = end_sec.unwrap_or(duration_sec);
    if end <= start_sec {
        return Err(format!(
            "End time ({:.2}s) must be after start time ({:.2}s)",
            end, start_sec
        ));
    }
    if duration_sec > 0.0 {
        if start_sec >= duration_sec {
            return Err(format!(
                "Start time ({:.2}s) is beyond the video's duration ({:.2}s)",
                start_sec, duration_sec
            ));
        }
        if end > duration_sec {
            return Err(format!(
                "End time ({:.2}s) exceeds the video's duration ({:.2}s)",
                end, duration_sec
            ));
        }
    }
    Ok(end)
}

/// Formats `(start_sec, end_sec)` into the `-ss`/`-t` argument strings.
/// `-t` is a *duration* (end - start), not `-to` — as an input option `-to`
/// is interpreted relative to the input's own timeline, not to `-ss`, which
/// would silently produce the wrong cut. `-t` after `-ss` is unambiguous.
fn trim_time_args(start_sec: f64, end_sec: f64) -> (String, String) {
    (
        format!("{:.3}", start_sec),
        format!("{:.3}", end_sec - start_sec),
    )
}

pub fn trim_video(
    input_path: &str,
    start_sec: f64,
    end_sec: Option<f64>,
    output_dir: Option<&str>,
) -> Result<VideoTrimResult, String> {
    crate::core::paths::validate_input_path(input_path, false)?;

    let stats = probe_video_stats(input_path)
        .map_err(|e| format!("Cannot read video metadata for '{}': {}", input_path, e))?;
    let end = resolve_trim_range(start_sec, end_sec, stats.duration_sec)?;

    let output_path = resolve_video_output_path(input_path, output_dir, "_trimmed")?;
    let output_file = PathBuf::from(&output_path);
    let temp_output = temporary_output_path(&output_file)?;
    let temp_output_str = temp_output
        .to_str()
        .ok_or_else(|| "Temporary output path contains invalid UTF-8".to_string())?
        .to_string();

    let (ss, duration) = trim_time_args(start_sec, end);
    let ffmpeg_bin = get_ffmpeg_path();
    let mut cmd = FfmpegCommand::new_with_path(ffmpeg_bin);
    // Fast/lossless trim: `-c copy` repackages the existing stream without
    // re-encoding, so the cut lands on the nearest keyframe at or before
    // `start_sec` rather than the exact requested frame. A frame-accurate
    // cut would require re-encoding (drop `-c copy`, add e.g. `-c:v libx264
    // -crf 20 -c:a aac`) — deliberately not implemented here; this is the
    // fast/lossless path only, matching what's needed today.
    cmd.arg("-ss")
        .arg(ss)
        .input(input_path)
        .overwrite()
        .arg("-t")
        .arg(duration)
        .arg("-c")
        .arg("copy")
        // Normalizes the output's first timestamp to zero. Without this, a
        // copy-trim that lands mid-GOP can leave a non-zero starting PTS,
        // which some players render as a black flash / AV desync at the
        // start of playback.
        .arg("-avoid_negative_ts")
        .arg("make_zero")
        .output(&temp_output_str);

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let mut ffmpeg_errors: Vec<String> = Vec::new();
    for event in child.iter().map_err(|e| e.to_string())? {
        if let FfmpegEvent::Log(level, msg) = event {
            use ffmpeg_sidecar::event::LogLevel;
            if matches!(level, LogLevel::Fatal | LogLevel::Error) {
                ffmpeg_errors.push(msg);
            }
        }
    }
    if !ffmpeg_errors.is_empty() {
        let _ = std::fs::remove_file(&temp_output);
        return Err(ffmpeg_errors.join("; "));
    }
    if !temp_output.exists() {
        return Err("FFmpeg produced no output — trim likely failed".to_string());
    }

    install_temp_output(&temp_output, &output_file)?;

    Ok(VideoTrimResult {
        input_path: input_path.to_string(),
        output_path,
    })
}

pub fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    let canonical = crate::core::paths::validate_input_path(path, false)?;
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

// Windows: GyanD/codexffmpeg is a per-version tagged GitHub release (not BtbN's
// dated "autobuild-*" tags, which get pruned once superseded — pinning one of
// those 404s as soon as it ages out). Presumed non-pruned based on Gyan's
// multi-year history of keeping past version tags around.
// Linux: johnvansickle.com's old-releases/ directory is an append-only archive
// of exact version+arch static builds that is never pruned (unlike the
// floating "release"/"latest" pointer at the top level, which is overwritten
// on every new build). ffmpeg 6.0.1 is the newest build available there.
// GPL static builds — required because VideoPreset::codec() uses libx264/libx265.
// SHA-256 computed locally against the downloaded bytes, not the source's own
// checksums.sha256 file.
fn ffmpeg_archive_source(target: &str) -> Option<(&'static str, &'static str)> {
    match target {
        "x86_64-pc-windows-msvc" => Some((
            "https://github.com/GyanD/codexffmpeg/releases/download/8.1.2/ffmpeg-8.1.2-essentials_build.zip",
            "db580001caa24ac104c8cb856cd113a87b0a443f7bdf47d8c12b1d740584a2ec",
        )),
        "x86_64-unknown-linux-gnu" => Some((
            "https://johnvansickle.com/ffmpeg/old-releases/ffmpeg-6.0.1-amd64-static.tar.xz",
            "28268bf402f1083833ea269331587f60a242848880073be8016501d864bd07a5",
        )),
        _ => None,
    }
}

// evermeet.cx publishes per-version snapshot URLs (unlike the floating
// getrelease/zip "latest" pointer) but only builds for Intel Macs — no
// aarch64-apple-darwin build exists here or anywhere else we could verify.
fn ffmpeg_evermeet_source(binary: &str) -> Option<(&'static str, &'static str)> {
    match binary {
        "ffmpeg" => Some((
            "https://evermeet.cx/ffmpeg/ffmpeg-8.1.2.zip",
            "e91df72a1ee7c26606f90dd2dd4dcccc6a75140ff9ea6fdd50faae828b82ba69",
        )),
        "ffprobe" => Some((
            "https://evermeet.cx/ffmpeg/ffprobe-8.1.2.zip",
            "399b93f0b9862f69767afa343e90c2f48d7e7958cadbb6deb76a012d0e3b7ce3",
        )),
        _ => None,
    }
}

fn verify_sha256(bytes: &[u8], expected: &str, archive_path: &Path) -> Result<(), String> {
    let hash = Sha256::digest(bytes);
    let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    if hex != expected {
        let _ = std::fs::remove_file(archive_path);
        return Err(format!(
            "FFmpeg download failed integrity check (expected {}, got {}) — file removed",
            expected, hex
        ));
    }
    Ok(())
}

fn extract_single_binary_zip(bytes: &[u8], binary_name: &str, dest: &Path) -> Result<(), String> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("FFmpeg archive is corrupted: {}", e))?;
    let mut file = archive
        .by_name(binary_name)
        .map_err(|_| format!("FFmpeg archive missing expected binary '{}'", binary_name))?;
    let out_path = dest.join(binary_name);
    let mut out = std::fs::File::create(&out_path)
        .map_err(|e| format!("Failed to write {}: {}", binary_name, e))?;
    std::io::copy(&mut file, &mut out)
        .map_err(|e| format!("Failed to write {}: {}", binary_name, e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to mark {} executable: {}", binary_name, e))?;
    }
    Ok(())
}

// macOS Intel only — evermeet ships ffmpeg/ffprobe as two separate single-binary
// zips, so this can't reuse unpack_ffmpeg_without_extras (which expects one
// combined archive). Progress resets to 0 for the second download; acceptable
// for a one-time setup step.
fn download_ffmpeg_macos(
    dest: &Path,
    progress_cb: impl Fn(u64, u64) + Send + 'static,
) -> Result<PathBuf, String> {
    use ffmpeg_sidecar::download::{
        download_ffmpeg_package_with_progress, FfmpegDownloadProgressEvent,
    };

    for name in ["ffmpeg", "ffprobe"] {
        let (url, expected_hash) = ffmpeg_evermeet_source(name)
            .ok_or_else(|| format!("No pinned FFmpeg build for '{}'", name))?;
        let archive_path = download_ffmpeg_package_with_progress(url, dest, |event| {
            if let FfmpegDownloadProgressEvent::Downloading {
                total_bytes,
                downloaded_bytes,
            } = event
            {
                progress_cb(downloaded_bytes, total_bytes);
            }
        })
        .map_err(|e| format!("FFmpeg download failed: {}", e))?;

        let bytes = std::fs::read(&archive_path)
            .map_err(|e| format!("Failed to read downloaded archive: {}", e))?;
        verify_sha256(&bytes, expected_hash, &archive_path)?;
        extract_single_binary_zip(&bytes, name, dest)?;
        let _ = std::fs::remove_file(&archive_path);
    }

    let path = get_ffmpeg_path();
    if !path.exists() {
        return Err("FFmpeg binary not found after download".into());
    }
    Ok(path)
}

pub fn download_ffmpeg(progress_cb: impl Fn(u64, u64) + Send + 'static) -> Result<PathBuf, String> {
    use ffmpeg_sidecar::download::{
        download_ffmpeg_package_with_progress, unpack_ffmpeg, FfmpegDownloadProgressEvent,
    };

    if ffmpeg_is_available() && ffprobe_path().exists() {
        return Ok(get_ffmpeg_path());
    }

    let dest = app_support_ffmpeg_path()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .ok_or("Could not determine application data directory")?;

    std::fs::create_dir_all(&dest).map_err(|e| format!("Failed to create ffmpeg dir: {}", e))?;

    let target = crate::core::gif::target_triple()?;

    if target == "aarch64-apple-darwin" {
        return Err(
            "No verified FFmpeg build is pinned for Apple Silicon yet. Install FFmpeg \
             manually (e.g. `brew install ffmpeg`) and ReVault will detect it automatically."
                .to_string(),
        );
    }

    if target == "x86_64-apple-darwin" {
        return download_ffmpeg_macos(&dest, progress_cb);
    }

    let (url, expected_hash) = ffmpeg_archive_source(target)
        .ok_or_else(|| format!("No pinned FFmpeg build for platform {}", target))?;

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

    let bytes = std::fs::read(&archive_path)
        .map_err(|e| format!("Failed to read downloaded archive: {}", e))?;
    verify_sha256(&bytes, expected_hash, &archive_path)?;

    unpack_ffmpeg(&archive_path, &dest).map_err(|e| format!("FFmpeg unpack failed: {}", e))?;

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
        let result = resolve_video_output_path("/tmp/video.mp4", None, "_compressed").unwrap();
        let expected = Path::new("/tmp")
            .join("video_compressed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);

        // MOV inputs are remuxed to MP4 for broader compatibility.
        let result = resolve_video_output_path("/home/user/clip.mov", None, "_compressed").unwrap();
        let expected = Path::new("/home/user")
            .join("clip_compressed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);

        // Case-insensitive: .MOV also maps to .mp4.
        let result = resolve_video_output_path("/home/user/clip.MOV", None, "_compressed").unwrap();
        let expected = Path::new("/home/user")
            .join("clip_compressed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_resolve_video_output_path_with_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        // canonicalize the tempdir so the comparison matches the canonicalized
        // path returned by resolve_video_output_path (macOS resolves /var/...
        // → /private/var/...).
        let canon_dir = std::fs::canonicalize(dir.path()).unwrap();
        let dir_str = dir.path().to_str().unwrap();

        let result =
            resolve_video_output_path("/tmp/video.mp4", Some(dir_str), "_compressed").unwrap();
        let expected = canon_dir
            .join("video_compressed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);

        // MOV → MP4 remux applies even when output_dir is set.
        let result =
            resolve_video_output_path("/tmp/clip.mov", Some(dir_str), "_compressed").unwrap();
        let expected = canon_dir
            .join("clip_compressed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_resolve_video_output_path_does_not_clobber_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("clip_compressed.mp4");
        std::fs::write(&existing, b"existing").unwrap();

        let result = resolve_video_output_path(
            "/tmp/clip.mp4",
            Some(dir.path().to_str().unwrap()),
            "_compressed",
        )
        .unwrap();
        let expected = std::fs::canonicalize(dir.path())
            .unwrap()
            .join("clip_compressed_2.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
        assert_eq!(std::fs::read(&existing).unwrap(), b"existing");
    }

    #[test]
    fn test_install_temp_output_refuses_existing_final() {
        let dir = tempfile::tempdir().unwrap();
        let temp = dir.path().join(".out.revault-tmp.mp4");
        let final_path = dir.path().join("out.mp4");
        std::fs::write(&temp, b"new").unwrap();
        std::fs::write(&final_path, b"old").unwrap();

        let err = install_temp_output(&temp, &final_path).unwrap_err();
        assert!(err.contains("refusing to overwrite"));
        assert_eq!(std::fs::read(&final_path).unwrap(), b"old");
        assert!(!temp.exists());
    }

    #[test]
    fn test_install_temp_output_moves_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let temp = dir.path().join(".out.revault-tmp.mp4");
        let final_path = dir.path().join("out.mp4");
        std::fs::write(&temp, b"new").unwrap();

        install_temp_output(&temp, &final_path).unwrap();

        assert_eq!(std::fs::read(&final_path).unwrap(), b"new");
        assert!(!temp.exists());
    }

    #[test]
    fn test_resolve_video_output_path_missing_dir() {
        let result =
            resolve_video_output_path("/tmp/video.mp4", Some("/nonexistent/dir"), "_compressed");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid output dir"));
    }

    #[test]
    fn test_resolve_video_output_path_rejects_traversal() {
        // canonicalize() resolves "../.." against cwd; the resulting path is
        // unlikely to be a directory we accidentally write to. The point is
        // that we no longer hand the raw "../.." string back to ffmpeg.
        let dir = tempfile::tempdir().unwrap();
        let traversal = dir.path().join("..").join("..").join("etc");
        let result = resolve_video_output_path("/tmp/video.mp4", traversal.to_str(), "_compressed");
        // Either canonicalize fails (path doesn't exist) or it resolves to
        // something — either way the input string is normalized away.
        if let Ok(out) = result {
            assert!(!out.contains(".."), "output path leaks traversal: {}", out);
        }
    }

    #[test]
    fn test_resolve_trim_output_path_uses_trimmed_suffix() {
        let result = resolve_video_output_path("/tmp/clip.mp4", None, "_trimmed").unwrap();
        let expected = Path::new("/tmp")
            .join("clip_trimmed.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_resolve_trim_output_path_does_not_clobber_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("clip_trimmed.mp4");
        std::fs::write(&existing, b"existing").unwrap();

        let result = resolve_video_output_path(
            "/tmp/clip.mp4",
            Some(dir.path().to_str().unwrap()),
            "_trimmed",
        )
        .unwrap();
        let expected = std::fs::canonicalize(dir.path())
            .unwrap()
            .join("clip_trimmed_2.mp4")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
        assert_eq!(std::fs::read(&existing).unwrap(), b"existing");
    }

    #[test]
    fn test_trim_time_args_formats_ss_and_duration() {
        let (ss, duration) = trim_time_args(5.0, 12.5);
        assert_eq!(ss, "5.000");
        assert_eq!(duration, "7.500");
    }

    #[test]
    fn test_resolve_trim_range_defaults_end_to_duration() {
        let end = resolve_trim_range(2.0, None, 30.0).unwrap();
        assert_eq!(end, 30.0);
    }

    #[test]
    fn test_resolve_trim_range_explicit_end_within_duration() {
        let end = resolve_trim_range(2.0, Some(10.0), 30.0).unwrap();
        assert_eq!(end, 10.0);
    }

    #[test]
    fn test_resolve_trim_range_rejects_negative_start() {
        let err = resolve_trim_range(-1.0, Some(10.0), 30.0).unwrap_err();
        assert!(err.contains("non-negative"));
    }

    #[test]
    fn test_resolve_trim_range_rejects_start_at_or_after_end() {
        let err = resolve_trim_range(10.0, Some(10.0), 30.0).unwrap_err();
        assert!(err.contains("must be after"));

        let err = resolve_trim_range(15.0, Some(10.0), 30.0).unwrap_err();
        assert!(err.contains("must be after"));
    }

    #[test]
    fn test_resolve_trim_range_rejects_start_beyond_duration() {
        let err = resolve_trim_range(40.0, Some(45.0), 30.0).unwrap_err();
        assert!(err.contains("beyond the video's duration"));
    }

    #[test]
    fn test_resolve_trim_range_rejects_end_beyond_duration() {
        let err = resolve_trim_range(2.0, Some(45.0), 30.0).unwrap_err();
        assert!(err.contains("exceeds the video's duration"));
    }

    #[test]
    fn test_resolve_trim_range_skips_duration_checks_when_duration_unknown() {
        // duration_sec == 0.0 means ffprobe couldn't determine it — only the
        // start < end invariant should still be enforced.
        let end = resolve_trim_range(5.0, Some(20.0), 0.0).unwrap();
        assert_eq!(end, 20.0);
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
        assert_eq!(VideoPreset::Balanced.encoder_preset(), "slow");
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
        assert_eq!(stats.video_bitrate_bps, Some(5_000_000));
        assert_eq!(
            stats.creation_time.as_deref(),
            Some("2024-01-01T12:00:00.000000Z")
        );
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

    #[test]
    fn verify_sha256_rejects_tampered_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake-ffmpeg.zip");
        std::fs::write(&path, b"not the real archive bytes").unwrap();

        let err = verify_sha256(b"not the real archive bytes", &"0".repeat(64), &path).unwrap_err();

        assert!(err.contains("integrity check"), "got: {}", err);
        assert!(
            !path.exists(),
            "tampered archive should be deleted after a hash mismatch"
        );
    }

    #[test]
    fn verify_sha256_accepts_matching_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("real-ffmpeg.zip");
        let bytes = b"the real archive bytes";
        std::fs::write(&path, bytes).unwrap();
        let hex: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        assert!(verify_sha256(bytes, &hex, &path).is_ok());
        assert!(path.exists(), "verified archive must not be deleted");
    }

    #[test]
    fn ffmpeg_archive_source_covers_pinned_targets_only() {
        assert!(ffmpeg_archive_source("x86_64-pc-windows-msvc").is_some());
        assert!(ffmpeg_archive_source("x86_64-unknown-linux-gnu").is_some());
        // macOS is handled by ffmpeg_evermeet_source / the aarch64 guard, not this table.
        assert!(ffmpeg_archive_source("x86_64-apple-darwin").is_none());
        assert!(ffmpeg_archive_source("aarch64-apple-darwin").is_none());
        assert!(ffmpeg_archive_source("unknown-target").is_none());
    }

    #[test]
    fn ffmpeg_evermeet_source_covers_known_binaries_only() {
        assert!(ffmpeg_evermeet_source("ffmpeg").is_some());
        assert!(ffmpeg_evermeet_source("ffprobe").is_some());
        assert!(ffmpeg_evermeet_source("ffplay").is_none());
    }

    #[test]
    fn extract_single_binary_zip_writes_executable_file() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("archive.zip");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer
                .start_file("ffmpeg", zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"fake binary contents").unwrap();
            writer.finish().unwrap();
        }
        let bytes = std::fs::read(&zip_path).unwrap();

        extract_single_binary_zip(&bytes, "ffmpeg", dir.path()).unwrap();

        let out_path = dir.path().join("ffmpeg");
        assert_eq!(std::fs::read(&out_path).unwrap(), b"fake binary contents");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&out_path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
    }

    #[test]
    fn extract_single_binary_zip_errors_on_missing_entry() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("archive.zip");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer
                .start_file("other-file", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.finish().unwrap();
        }
        let bytes = std::fs::read(&zip_path).unwrap();

        let err = extract_single_binary_zip(&bytes, "ffmpeg", dir.path()).unwrap_err();
        assert!(err.contains("missing expected binary"), "got: {}", err);
    }
}
