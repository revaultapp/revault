use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn validate_input_path(input: &str, allow_dirs: bool) -> Result<PathBuf, String> {
    let canonical = Path::new(input)
        .canonicalize()
        .map_err(|e| format!("invalid path '{}': {}", input, e))?;
    if allow_dirs {
        if !canonical.exists() {
            return Err(format!("path does not exist: {}", canonical.display()));
        }
    } else if !canonical.is_file() {
        return Err(format!("not a regular file: {}", canonical.display()));
    }
    Ok(canonical)
}

pub fn validate_output_suffix(suffix: &str) -> Result<(), String> {
    if suffix.is_empty() {
        return Err("output suffix cannot be empty".to_string());
    }
    if suffix
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Ok(())
    } else {
        Err("output suffix may only contain letters, numbers, '_' or '-'".to_string())
    }
}

/// Builds a hidden sibling path (`.{stem}.revault-tmp-{pid}-{nonce}.{ext}`) next
/// to `final_path`, used so encoders write to a temp file that gets atomically
/// hard-linked into place on success (see `install_temp_output`). `label` is
/// used only in the "no parent directory" error message (e.g. "GIF output" /
/// "Output"); `default_stem`/`default_ext` are used when `final_path` itself
/// lacks a stem/extension.
pub fn temporary_output_path(
    final_path: &Path,
    label: &str,
    default_stem: &str,
    default_ext: &str,
) -> Result<PathBuf, String> {
    let parent = final_path
        .parent()
        .ok_or_else(|| format!("{} path has no parent directory", label))?;
    let stem = final_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(default_stem);
    let ext = final_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or(default_ext);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(parent.join(format!(
        ".{stem}.revault-tmp-{}-{nonce}.{ext}",
        std::process::id()
    )))
}

/// Hard-links `temp_path` onto `final_path` (atomic, no partial-write window)
/// and removes the temp file. Refuses to clobber an existing final file.
/// `label` names the thing being moved in error messages (e.g. "GIF" / "output").
pub fn install_temp_output(temp_path: &Path, final_path: &Path, label: &str) -> Result<(), String> {
    match std::fs::hard_link(temp_path, final_path) {
        Ok(()) => std::fs::remove_file(temp_path)
            .map_err(|e| format!("Failed to clean temporary {}: {}", label, e)),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = std::fs::remove_file(temp_path);
            Err(format!(
                "Output already exists, refusing to overwrite: {}",
                final_path.display()
            ))
        }
        Err(e) => {
            let _ = std::fs::remove_file(temp_path);
            Err(format!("Failed to move {} into place: {}", label, e))
        }
    }
}

/// Canonicalizes `dir` and verifies it exists and is a directory. Shared by
/// every feature that accepts an optional output folder from the frontend
/// (compression, resize, privacy, video, PDF tools) so they all fail the
/// same way on a missing path or a path that isn't a directory.
pub fn validate_output_dir(dir: &str) -> Result<PathBuf, String> {
    let canon =
        std::fs::canonicalize(dir).map_err(|e| format!("Invalid output dir '{}': {}", dir, e))?;
    if !canon.is_dir() {
        return Err(format!("Output path is not a directory: {}", dir));
    }
    Ok(canon)
}

/// Returns `base` if it's free, otherwise the first `{stem}_{n}.{ext}`
/// sibling that doesn't already exist on disk or in `reserved`. `reserved`
/// lets a batch resolve every output path up front without two inputs
/// landing on the same not-yet-written candidate.
pub fn first_available_path(base: &Path, reserved: &mut HashSet<PathBuf>) -> PathBuf {
    if !base.exists() && reserved.insert(base.to_path_buf()) {
        return base.to_path_buf();
    }
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = base.extension().and_then(|e| e.to_str());
    let mut n = 2;
    loop {
        let candidate = match ext {
            Some(ext) => parent.join(format!("{stem}_{n}.{ext}")),
            None => parent.join(format!("{stem}_{n}")),
        };
        if !candidate.exists() && reserved.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn validate_input_path_happy() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.txt");
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(b"hi").unwrap();
        let result = validate_input_path(p.to_str().unwrap(), false).unwrap();
        assert!(result.is_absolute());
        assert!(result.is_file());
    }

    #[test]
    fn validate_input_path_missing() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("does_not_exist.jpg");
        let err = validate_input_path(p.to_str().unwrap(), false).unwrap_err();
        assert!(err.contains("invalid path"));
    }

    #[test]
    fn validate_input_path_rejects_directory() {
        let dir = tempfile::tempdir().unwrap();
        let err = validate_input_path(dir.path().to_str().unwrap(), false).unwrap_err();
        assert!(err.contains("not a regular file"));
    }

    #[test]
    fn validate_input_path_allows_directory_when_flag_set() {
        let dir = tempfile::tempdir().unwrap();
        let result = validate_input_path(dir.path().to_str().unwrap(), true).unwrap();
        assert!(result.is_dir());
    }

    #[test]
    fn validate_output_suffix_accepts_safe_suffix() {
        assert!(validate_output_suffix("_instagram-portrait").is_ok());
    }

    #[test]
    fn validate_output_suffix_rejects_empty_or_path_like_suffix() {
        assert!(validate_output_suffix("").is_err());
        assert!(validate_output_suffix("/../../original").is_err());
        assert!(validate_output_suffix("\\evil").is_err());
    }

    #[test]
    fn temporary_output_path_uses_final_path_stem_and_extension() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("clip.gif");
        let temp = temporary_output_path(&final_path, "GIF output", "gif", "gif").unwrap();
        assert_eq!(temp.parent().unwrap(), dir.path());
        let name = temp.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".clip.revault-tmp-"), "got: {}", name);
        assert!(name.ends_with(".gif"), "got: {}", name);
    }

    #[test]
    fn temporary_output_path_falls_back_to_default_ext_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("clip"); // no extension
        let temp = temporary_output_path(&final_path, "Output", "video", "mp4").unwrap();
        let name = temp.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".clip.revault-tmp-"), "got: {}", name);
        assert!(name.ends_with(".mp4"), "got: {}", name);
    }

    #[test]
    fn temporary_output_path_errors_with_label_when_no_parent_directory() {
        let err = temporary_output_path(Path::new("/"), "GIF output", "gif", "gif").unwrap_err();
        assert_eq!(err, "GIF output path has no parent directory");
    }

    #[test]
    fn install_temp_output_moves_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let temp = dir.path().join(".clip.revault-tmp.gif");
        let final_path = dir.path().join("clip.gif");
        std::fs::write(&temp, b"new").unwrap();

        install_temp_output(&temp, &final_path, "GIF").unwrap();

        assert_eq!(std::fs::read(&final_path).unwrap(), b"new");
        assert!(!temp.exists());
    }

    #[test]
    fn validate_output_dir_happy() {
        let dir = tempfile::tempdir().unwrap();
        let canon = validate_output_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(canon, fs::canonicalize(dir.path()).unwrap());
    }

    #[test]
    fn validate_output_dir_missing() {
        let err = validate_output_dir("/nonexistent/revault-test-xyz").unwrap_err();
        assert!(err.contains("Invalid output dir"), "got: {}", err);
    }

    #[test]
    fn validate_output_dir_rejects_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.txt");
        fs::write(&file, b"hi").unwrap();
        let err = validate_output_dir(file.to_str().unwrap()).unwrap_err();
        assert_eq!(
            err,
            format!("Output path is not a directory: {}", file.to_str().unwrap())
        );
    }

    #[test]
    fn first_available_path_returns_base_when_free() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("out.jpg");
        let mut reserved = HashSet::new();
        assert_eq!(first_available_path(&base, &mut reserved), base);
    }

    #[test]
    fn first_available_path_increments_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("out.jpg");
        fs::write(&base, b"old").unwrap();
        let mut reserved = HashSet::new();
        let result = first_available_path(&base, &mut reserved);
        assert_eq!(result, dir.path().join("out_2.jpg"));
    }

    #[test]
    fn first_available_path_respects_reserved_set_across_calls() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("out.jpg");
        let mut reserved = HashSet::new();
        let first = first_available_path(&base, &mut reserved);
        let second = first_available_path(&base, &mut reserved);
        assert_ne!(first, second);
        assert_eq!(second, dir.path().join("out_2.jpg"));
    }

    #[test]
    fn install_temp_output_refuses_existing_final() {
        let dir = tempfile::tempdir().unwrap();
        let temp = dir.path().join(".clip.revault-tmp.gif");
        let final_path = dir.path().join("clip.gif");
        std::fs::write(&temp, b"new").unwrap();
        std::fs::write(&final_path, b"old").unwrap();

        let err = install_temp_output(&temp, &final_path, "GIF").unwrap_err();
        assert!(err.contains("refusing to overwrite"));
        assert_eq!(std::fs::read(&final_path).unwrap(), b"old");
        assert!(!temp.exists());
    }
}
