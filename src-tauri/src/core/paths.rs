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
}
