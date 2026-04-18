use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::core::paths::validate_input_path;
use crate::core::video::{get_ffmpeg_path, probe_video_stats};

pub const GIFSKI_VERSION: &str = "1.34.0";
const GIFSKI_RELEASE_BASE: &str = "https://github.com/revaultapp/revault/releases/download";

const MAX_RANGE_SEC: f32 = 15.0;
const ALLOWED_FPS: [u32; 3] = [10, 15, 24];
const ALLOWED_WIDTH: [u32; 5] = [320, 480, 640, 720, 1080];

#[derive(Debug, Serialize)]
pub struct GifResult {
    pub output_path: String,
    pub size_bytes: u64,
    pub duration_sec: f32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GifOptions {
    pub start_sec: f32,
    pub end_sec: f32,
    pub fps: u32,
    pub width: u32,
    pub quality: u8,
}

impl GifOptions {
    pub fn validate(&self) -> Result<(), String> {
        if !ALLOWED_FPS.contains(&self.fps) {
            return Err(format!(
                "invalid fps {}: allowed {:?}",
                self.fps, ALLOWED_FPS
            ));
        }
        if !ALLOWED_WIDTH.contains(&self.width) {
            return Err(format!(
                "invalid width {}: allowed {:?}",
                self.width, ALLOWED_WIDTH
            ));
        }
        if self.quality < 1 || self.quality > 100 {
            return Err(format!("invalid quality {}: must be 1-100", self.quality));
        }
        if !(self.start_sec.is_finite() && self.end_sec.is_finite()) {
            return Err("start/end must be finite".to_string());
        }
        if self.start_sec < 0.0 {
            return Err("start_sec must be >= 0".to_string());
        }
        if self.end_sec <= self.start_sec {
            return Err("end_sec must be > start_sec".to_string());
        }
        if self.end_sec - self.start_sec > MAX_RANGE_SEC {
            return Err(format!("range exceeds {} second cap", MAX_RANGE_SEC as u32));
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub fn resolve_gif_output_path(
    input_path: &str,
    output_dir: Option<&str>,
) -> Result<String, String> {
    let path = Path::new(input_path);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;
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
    let output = dir.join(format!("{}.gif", stem));
    output
        .to_str()
        .map(|s| s.to_string())
        .ok_or("Invalid path".to_string())
}

/// Heuristic only — for UI preview before encoding. Assumes ~50KB/frame at
/// default resolutions/quality. Real output varies ±50% based on content entropy.
pub fn estimate_gif_size(opts: &GifOptions) -> u64 {
    let duration = (opts.end_sec - opts.start_sec).max(0.0);
    let frames = (duration * opts.fps as f32) as u64;
    let width_factor = opts.width as f32 / 480.0;
    let quality_factor = opts.quality as f32 / 85.0;
    let per_frame = (50_000.0 * width_factor * quality_factor) as u64;
    frames * per_frame
}

fn gifski_filename() -> &'static str {
    if cfg!(windows) {
        "gifski.exe"
    } else {
        "gifski"
    }
}

pub fn gifski_binary_path(app_data_dir: &Path) -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("REVAULT_GIFSKI_PATH") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(format!(
            "REVAULT_GIFSKI_PATH points to missing file: {}",
            pb.display()
        ));
    }
    let candidate = app_data_dir.join("bin").join(gifski_filename());
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err("gifski binary not found — install via `download_gifski`".to_string())
}

pub fn target_triple() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        _ => Err("Esta plataforma no está soportada para exportar GIFs".to_string()),
    }
}

pub fn download_url(target: &str) -> String {
    let ext = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };
    format!(
        "{}/gifski-v{}/gifski-{}-{}.{}",
        GIFSKI_RELEASE_BASE, GIFSKI_VERSION, GIFSKI_VERSION, target, ext
    )
}

pub fn gifski_installed_version(binary: &Path) -> Result<String, String> {
    let out = std::process::Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|e| format!("No se pudo ejecutar gifski: {}", e))?;
    if !out.status.success() {
        return Err("La instalación ha fallado".to_string());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Expect stdout like "gifski 1.34.0\n"
    let first = stdout.lines().next().unwrap_or("").trim();
    let version = first
        .strip_prefix("gifski ")
        .ok_or_else(|| "La instalación ha fallado".to_string())?
        .trim();
    if version.is_empty() {
        return Err("La instalación ha fallado".to_string());
    }
    Ok(version.to_string())
}

pub fn check_gifski(app_data_dir: &Path) -> Result<bool, String> {
    let path = match gifski_binary_path(app_data_dir) {
        Ok(p) => p,
        Err(_) => return Ok(false),
    };
    match gifski_installed_version(&path) {
        Ok(v) => Ok(v == GIFSKI_VERSION),
        Err(_) => Ok(false),
    }
}

fn extract_tar_gz(archive_bytes: &[u8], dest_dir: &Path) -> Result<PathBuf, String> {
    let gz = flate2::read::GzDecoder::new(archive_bytes);
    let mut archive = tar::Archive::new(gz);
    let mut binary_out: Option<PathBuf> = None;
    let unverified = dest_dir.join(format!("{}.unverified", gifski_filename()));
    let license_out = dest_dir.join("gifski-LICENSE.txt");
    for entry in archive
        .entries()
        .map_err(|e| format!("La descarga se corrompió, inténtalo otra vez ({})", e))?
    {
        let mut entry =
            entry.map_err(|e| format!("La descarga se corrompió, inténtalo otra vez ({})", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("La descarga se corrompió, inténtalo otra vez ({})", e))?
            .into_owned();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == "gifski" {
            let mut f = std::fs::File::create(&unverified)
                .map_err(|e| format!("No se pudo escribir el binario: {}", e))?;
            std::io::copy(&mut entry, &mut f)
                .map_err(|e| format!("La descarga se corrompió, inténtalo otra vez ({})", e))?;
            binary_out = Some(unverified.clone());
        } else if name.eq_ignore_ascii_case("LICENSE")
            || name.eq_ignore_ascii_case("LICENSE.txt")
            || name.eq_ignore_ascii_case("LICENSE.md")
        {
            let mut f = std::fs::File::create(&license_out)
                .map_err(|e| format!("No se pudo escribir la licencia: {}", e))?;
            let _ = std::io::copy(&mut entry, &mut f);
        }
    }
    binary_out.ok_or_else(|| "La descarga se corrompió, inténtalo otra vez".to_string())
}

fn extract_zip(archive_bytes: &[u8], dest_dir: &Path) -> Result<PathBuf, String> {
    let reader = std::io::Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("La descarga se corrompió, inténtalo otra vez ({})", e))?;
    let unverified = dest_dir.join(format!("{}.unverified", gifski_filename()));
    let license_out = dest_dir.join("gifski-LICENSE.txt");
    let mut binary_out: Option<PathBuf> = None;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("La descarga se corrompió, inténtalo otra vez ({})", e))?;
        let enclosed = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let name = enclosed
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == "gifski.exe" {
            let mut f = std::fs::File::create(&unverified)
                .map_err(|e| format!("No se pudo escribir el binario: {}", e))?;
            std::io::copy(&mut entry, &mut f)
                .map_err(|e| format!("La descarga se corrompió, inténtalo otra vez ({})", e))?;
            binary_out = Some(unverified.clone());
        } else if name.eq_ignore_ascii_case("LICENSE")
            || name.eq_ignore_ascii_case("LICENSE.txt")
            || name.eq_ignore_ascii_case("LICENSE.md")
        {
            let mut f = std::fs::File::create(&license_out)
                .map_err(|e| format!("No se pudo escribir la licencia: {}", e))?;
            let _ = std::io::copy(&mut entry, &mut f);
        }
    }
    binary_out.ok_or_else(|| "La descarga se corrompió, inténtalo otra vez".to_string())
}

pub fn download_and_install<F>(app_data_dir: &Path, mut emit_progress: F) -> Result<PathBuf, String>
where
    F: FnMut(u64, u64),
{
    eprintln!(
        "[gifski] download_and_install start, app_data_dir={}",
        app_data_dir.display()
    );
    let target = target_triple()?;
    let url = download_url(target);
    eprintln!("[gifski] target={} url={}", target, url);
    let bin_dir = app_data_dir.join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| {
        eprintln!("[gifski] create bin_dir failed: {}", e);
        format!("No se pudo crear el directorio: {}", e)
    })?;
    eprintln!("[gifski] bin_dir={}", bin_dir.display());

    let resp = ureq::get(&url).call().map_err(|e| {
        eprintln!("[gifski] HTTP request failed: {:?}", e);
        "No se pudo descargar el componente".to_string()
    })?;
    eprintln!("[gifski] HTTP {} {}", resp.status(), url);
    let total: u64 = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    eprintln!("[gifski] content-length={} bytes", total);

    let tmp_path = bin_dir.join(format!("{}.download.tmp", gifski_filename()));
    let mut reader = resp.into_body().into_reader();
    let mut buffer = Vec::with_capacity(if total > 0 { total as usize } else { 1_024_000 });
    {
        let mut file = std::fs::File::create(&tmp_path).map_err(|e| {
            eprintln!("[gifski] create tmp_path failed: {}", e);
            format!("No se pudo crear archivo temporal: {}", e)
        })?;
        let mut chunk = vec![0u8; 64 * 1024];
        let mut done: u64 = 0;
        loop {
            let n = reader.read(&mut chunk).map_err(|e| {
                eprintln!("[gifski] stream read failed at {} bytes: {}", done, e);
                "No se pudo descargar el componente".to_string()
            })?;
            if n == 0 {
                break;
            }
            use std::io::Write;
            file.write_all(&chunk[..n]).map_err(|e| {
                eprintln!("[gifski] write to tmp failed: {}", e);
                format!("No se pudo escribir descarga: {}", e)
            })?;
            buffer.extend_from_slice(&chunk[..n]);
            done += n as u64;
            emit_progress(done, total);
        }
        eprintln!("[gifski] download complete, {} bytes written", done);
    }

    let unverified = if target.contains("windows") {
        eprintln!("[gifski] extracting .zip");
        extract_zip(&buffer, &bin_dir)?
    } else {
        eprintln!("[gifski] extracting .tar.gz");
        extract_tar_gz(&buffer, &bin_dir)?
    };
    eprintln!("[gifski] extracted to {}", unverified.display());
    let _ = std::fs::remove_file(&tmp_path);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&unverified, perms).map_err(|e| {
            eprintln!("[gifski] chmod failed: {}", e);
            format!("No se pudo marcar ejecutable: {}", e)
        })?;
        eprintln!("[gifski] chmod 755 OK");
    }

    match gifski_installed_version(&unverified) {
        Ok(v) if v == GIFSKI_VERSION => {
            eprintln!("[gifski] verify OK: installed version = {}", v);
        }
        Ok(v) => {
            eprintln!(
                "[gifski] verify FAIL: expected {}, got {}",
                GIFSKI_VERSION, v
            );
            let _ = std::fs::remove_file(&unverified);
            return Err("La instalación ha fallado".to_string());
        }
        Err(e) => {
            eprintln!("[gifski] verify FAIL: {}", e);
            let _ = std::fs::remove_file(&unverified);
            return Err("La instalación ha fallado".to_string());
        }
    }

    let final_path = bin_dir.join(gifski_filename());
    eprintln!(
        "[gifski] rename {} → {}",
        unverified.display(),
        final_path.display()
    );
    std::fs::rename(&unverified, &final_path)
        .map_err(|e| format!("No se pudo instalar el binario: {}", e))?;
    Ok(final_path)
}

pub fn export_gif(
    app_data_dir: &Path,
    input_path: &str,
    output_path: &str,
    opts: GifOptions,
) -> Result<GifResult, String> {
    opts.validate()?;
    validate_input_path(input_path, false)?;
    let gifski = gifski_binary_path(app_data_dir)?;

    let tmp = tempfile::tempdir().map_err(|e| format!("tempdir: {}", e))?;
    let frame_pattern = tmp.path().join("frame_%04d.png");
    let frame_pattern_str = frame_pattern
        .to_str()
        .ok_or("tempdir path is not valid UTF-8")?;

    let filter = format!("fps={},scale={}:-1:flags=lanczos", opts.fps, opts.width);
    let mut ff = FfmpegCommand::new_with_path(get_ffmpeg_path());
    ff.arg("-ss")
        .arg(format!("{}", opts.start_sec))
        .arg("-to")
        .arg(format!("{}", opts.end_sec))
        .input(input_path)
        .arg("-vf")
        .arg(filter)
        .overwrite()
        .arg(frame_pattern_str);

    let mut child = ff.spawn().map_err(|e| format!("ffmpeg spawn: {}", e))?;
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
        return Err(format!(
            "ffmpeg frame extraction failed: {}",
            ffmpeg_errors.join("; ")
        ));
    }

    let mut frames: Vec<PathBuf> = std::fs::read_dir(tmp.path())
        .map_err(|e| format!("read tempdir: {}", e))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("frame_") && n.ends_with(".png"))
                .unwrap_or(false)
        })
        .collect();
    if frames.is_empty() {
        return Err("ffmpeg produced no frames — check start/end range".to_string());
    }
    frames.sort();

    let mut cmd = std::process::Command::new(&gifski);
    cmd.arg("--fps")
        .arg(opts.fps.to_string())
        .arg("--width")
        .arg(opts.width.to_string())
        .arg("--quality")
        .arg(opts.quality.to_string())
        .arg("-o")
        .arg(output_path)
        .args(&frames);
    let out = cmd.output().map_err(|e| format!("gifski spawn: {}", e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("gifski failed: {}", stderr));
    }

    let size_bytes = std::fs::metadata(output_path)
        .map(|m| m.len())
        .map_err(|e| format!("gifski produced no output: {}", e))?;

    let stats = probe_video_stats(output_path)
        .map_err(|e| format!("could not read GIF metadata: {}", e))?;

    Ok(GifResult {
        output_path: output_path.to_string(),
        size_bytes,
        duration_sec: stats.duration_sec as f32,
        width: stats.width,
        height: stats.height,
        fps: if stats.fps > 0 { stats.fps } else { opts.fps },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_opts() -> GifOptions {
        GifOptions {
            start_sec: 0.0,
            end_sec: 3.0,
            fps: 15,
            width: 480,
            quality: 85,
        }
    }

    #[test]
    fn resolve_gif_output_path_default_dir() {
        let result = resolve_gif_output_path("/tmp/clip.mp4", None).unwrap();
        let expected = Path::new("/tmp")
            .join("clip.gif")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_gif_output_path_with_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let dir_str = dir.path().to_str().unwrap();
        let result = resolve_gif_output_path("/tmp/clip.mov", Some(dir_str)).unwrap();
        let expected = dir.path().join("clip.gif").to_string_lossy().to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_gif_output_path_rejects_missing_dir() {
        let err = resolve_gif_output_path("/tmp/clip.mp4", Some("/nonexistent/xyz")).unwrap_err();
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn gif_result_serializes_expected_fields() {
        let r = GifResult {
            output_path: "/tmp/clip.gif".to_string(),
            size_bytes: 1234,
            duration_sec: 3.5,
            width: 480,
            height: 270,
            fps: 15,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["output_path"], "/tmp/clip.gif");
        assert_eq!(json["size_bytes"], 1234);
        assert_eq!(json["duration_sec"], 3.5);
        assert_eq!(json["width"], 480);
        assert_eq!(json["height"], 270);
        assert_eq!(json["fps"], 15);
    }

    #[test]
    fn estimate_gif_size_returns_positive_for_valid_opts() {
        let size = estimate_gif_size(&base_opts());
        assert!(size > 0, "expected >0, got {}", size);
    }

    #[test]
    fn estimate_gif_size_scales_with_duration_and_width() {
        let small = estimate_gif_size(&GifOptions {
            end_sec: 2.0,
            width: 320,
            ..base_opts()
        });
        let large = estimate_gif_size(&GifOptions {
            end_sec: 10.0,
            width: 720,
            ..base_opts()
        });
        assert!(small > 0);
        assert!(
            large > small * 4,
            "expected large >> small, got {} vs {}",
            large,
            small
        );
    }

    #[test]
    fn validate_rejects_bad_fps() {
        let err = GifOptions {
            fps: 30,
            ..base_opts()
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("fps"));
    }

    #[test]
    fn validate_rejects_bad_width() {
        let err = GifOptions {
            width: 500,
            ..base_opts()
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("width"));
    }

    #[test]
    fn validate_rejects_bad_quality() {
        assert!(GifOptions {
            quality: 0,
            ..base_opts()
        }
        .validate()
        .is_err());
        assert!(GifOptions {
            quality: 101,
            ..base_opts()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn validate_rejects_inverted_range() {
        let err = GifOptions {
            start_sec: 5.0,
            end_sec: 5.0,
            ..base_opts()
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("end_sec"));
    }

    #[test]
    fn validate_rejects_over_cap() {
        let err = GifOptions {
            start_sec: 0.0,
            end_sec: 20.0,
            ..base_opts()
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("cap"));
    }

    use std::sync::Mutex;
    // Serialize env-var tests to avoid races.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn gifski_path_env_var_hit() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("gifski-fake");
        std::fs::write(&fake, b"").unwrap();
        let prev = std::env::var("REVAULT_GIFSKI_PATH").ok();
        std::env::set_var("REVAULT_GIFSKI_PATH", &fake);
        let got = gifski_binary_path(dir.path()).unwrap();
        assert_eq!(got, fake);
        match prev {
            Some(v) => std::env::set_var("REVAULT_GIFSKI_PATH", v),
            None => std::env::remove_var("REVAULT_GIFSKI_PATH"),
        }
    }

    #[test]
    fn gifski_path_env_var_missing_file_errors() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::var("REVAULT_GIFSKI_PATH").ok();
        std::env::set_var("REVAULT_GIFSKI_PATH", "/definitely/not/a/file/xyz");
        let err = gifski_binary_path(dir.path()).unwrap_err();
        assert!(err.contains("REVAULT_GIFSKI_PATH"));
        match prev {
            Some(v) => std::env::set_var("REVAULT_GIFSKI_PATH", v),
            None => std::env::remove_var("REVAULT_GIFSKI_PATH"),
        }
    }

    #[test]
    fn gifski_path_falls_back_to_app_data() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("REVAULT_GIFSKI_PATH").ok();
        std::env::remove_var("REVAULT_GIFSKI_PATH");
        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let target = bin_dir.join(gifski_filename());
        std::fs::write(&target, b"").unwrap();
        let got = gifski_binary_path(dir.path()).unwrap();
        assert_eq!(got, target);
        if let Some(v) = prev {
            std::env::set_var("REVAULT_GIFSKI_PATH", v);
        }
    }

    #[test]
    fn gifski_path_errors_when_nothing_exists() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("REVAULT_GIFSKI_PATH").ok();
        std::env::remove_var("REVAULT_GIFSKI_PATH");
        let dir = tempfile::tempdir().unwrap();
        let err = gifski_binary_path(dir.path()).unwrap_err();
        assert!(err.contains("not found"));
        if let Some(v) = prev {
            std::env::set_var("REVAULT_GIFSKI_PATH", v);
        }
    }

    #[test]
    fn target_triple_current_platform_supported_or_errors_friendly() {
        // Just verify it returns a well-formed result on this platform.
        match target_triple() {
            Ok(t) => {
                assert!(!t.is_empty());
                assert!(t.contains('-'));
            }
            Err(e) => assert!(e.contains("no está soportada")),
        }
    }

    #[test]
    fn download_url_formats_correctly_for_unix_targets() {
        let url = download_url("aarch64-apple-darwin");
        assert_eq!(
            url,
            format!(
                "{}/gifski-v{}/gifski-{}-aarch64-apple-darwin.tar.gz",
                GIFSKI_RELEASE_BASE, GIFSKI_VERSION, GIFSKI_VERSION
            )
        );
        let url = download_url("x86_64-unknown-linux-gnu");
        assert!(url.ends_with("-x86_64-unknown-linux-gnu.tar.gz"));
    }

    #[test]
    fn download_url_uses_zip_for_windows() {
        let url = download_url("x86_64-pc-windows-msvc");
        assert!(url.ends_with("-x86_64-pc-windows-msvc.zip"));
    }

    #[test]
    fn gifski_installed_version_parses_stdout() {
        // Use a shell stub: create a script that prints "gifski 1.34.0".
        #[cfg(unix)]
        {
            let dir = tempfile::tempdir().unwrap();
            let stub = dir.path().join("stub.sh");
            std::fs::write(&stub, "#!/bin/sh\necho 'gifski 1.34.0'\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
            let v = gifski_installed_version(&stub).unwrap();
            assert_eq!(v, "1.34.0");
        }
    }

    #[test]
    fn gifski_installed_version_errors_on_bad_output() {
        #[cfg(unix)]
        {
            let dir = tempfile::tempdir().unwrap();
            let stub = dir.path().join("stub.sh");
            std::fs::write(&stub, "#!/bin/sh\necho 'not gifski at all'\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(gifski_installed_version(&stub).is_err());
        }
    }

    #[test]
    fn check_gifski_returns_false_when_missing() {
        let _g = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("REVAULT_GIFSKI_PATH").ok();
        std::env::remove_var("REVAULT_GIFSKI_PATH");
        let dir = tempfile::tempdir().unwrap();
        assert!(!check_gifski(dir.path()).unwrap());
        if let Some(v) = prev {
            std::env::set_var("REVAULT_GIFSKI_PATH", v);
        }
    }

    #[test]
    fn check_gifski_true_when_version_matches() {
        #[cfg(unix)]
        {
            let _g = ENV_LOCK.lock().unwrap();
            let prev = std::env::var("REVAULT_GIFSKI_PATH").ok();
            std::env::remove_var("REVAULT_GIFSKI_PATH");
            let dir = tempfile::tempdir().unwrap();
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir_all(&bin_dir).unwrap();
            let target = bin_dir.join(gifski_filename());
            std::fs::write(
                &target,
                format!("#!/bin/sh\necho 'gifski {}'\n", GIFSKI_VERSION),
            )
            .unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(check_gifski(dir.path()).unwrap());
            if let Some(v) = prev {
                std::env::set_var("REVAULT_GIFSKI_PATH", v);
            }
        }
    }

    #[test]
    fn check_gifski_false_when_version_mismatch() {
        #[cfg(unix)]
        {
            let _g = ENV_LOCK.lock().unwrap();
            let prev = std::env::var("REVAULT_GIFSKI_PATH").ok();
            std::env::remove_var("REVAULT_GIFSKI_PATH");
            let dir = tempfile::tempdir().unwrap();
            let bin_dir = dir.path().join("bin");
            std::fs::create_dir_all(&bin_dir).unwrap();
            let target = bin_dir.join(gifski_filename());
            std::fs::write(&target, "#!/bin/sh\necho 'gifski 0.0.0'\n").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(!check_gifski(dir.path()).unwrap());
            if let Some(v) = prev {
                std::env::set_var("REVAULT_GIFSKI_PATH", v);
            }
        }
    }
}
