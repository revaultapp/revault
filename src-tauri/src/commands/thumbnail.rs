#[tauri::command]
pub async fn generate_thumbnail(path: String, size: Option<u32>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let size = size.unwrap_or(80).clamp(32, 512);
        crate::core::image_io::generate_thumbnail(&path, size).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn validated_file_size(path: &str) -> Result<u64, String> {
    crate::core::paths::validate_input_path(path, false).and_then(|canonical| {
        std::fs::metadata(&canonical)
            .map(|m| m.len())
            .map_err(|e| format!("{}: {}", canonical.display(), e))
    })
}

#[tauri::command]
pub async fn get_file_sizes(paths: Vec<String>) -> Result<Vec<u64>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(|p| validated_file_size(p))
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_file_size_rejects_directories() {
        let dir = tempfile::tempdir().unwrap();
        let err = validated_file_size(dir.path().to_str().unwrap()).unwrap_err();
        assert!(err.contains("not a regular file"));
    }

    #[test]
    fn validated_file_size_returns_regular_file_size() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.jpg");
        std::fs::write(&file, b"12345").unwrap();
        assert_eq!(validated_file_size(file.to_str().unwrap()).unwrap(), 5);
    }
}

#[tauri::command]
pub async fn get_image_dimensions(path: String) -> Result<(u32, u32), String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::core::image_io::read_dimensions(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
