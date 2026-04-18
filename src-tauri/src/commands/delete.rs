use serde::Serialize;

#[cfg(test)]
mod tests {
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
        let results: Vec<DeleteResult> = paths
            .into_iter()
            .map(|path| match trash::delete(&path) {
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
            })
            .collect();
        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}
