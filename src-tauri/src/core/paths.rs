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
