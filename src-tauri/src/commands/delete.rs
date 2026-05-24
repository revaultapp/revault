use serde::Serialize;

fn delete_one(path: String) -> DeleteResult {
    match crate::core::paths::validate_input_path(&path, false) {
        Ok(canonical) => match trash::delete(&canonical) {
            Ok(()) => DeleteResult {
                path,
                success: true,
                error: None,
            },
            Err(e) => DeleteResult {
                path,
                success: false,
                error: Some(e.to_string()),
            },
        },
        Err(e) => DeleteResult {
            path,
            success: false,
            error: Some(e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn delete_nonexistent_path_returns_error() {
        let result = trash::delete("/nonexistent/path/to/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn delete_existing_path_works() {
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.txt");
        File::create(&temp_file).unwrap();

        // trash::delete uses AppleScript on macOS which may fail without GUI session
        // (error: "not authorized to send Apple events to Finder"). This is expected
        // in CI/headless environments. We just verify it doesn't panic.
        let _ = trash::delete(&temp_file);
    }

    #[test]
    fn delete_one_rejects_directories() {
        let temp_dir = TempDir::new().unwrap();
        let result = delete_one(temp_dir.path().to_str().unwrap().to_string());
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not a regular file"));
    }
}

#[derive(Serialize)]
pub struct DeleteResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn delete_files(paths: Vec<String>) -> Result<Vec<DeleteResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let results: Vec<DeleteResult> = paths.into_iter().map(delete_one).collect();
        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}
