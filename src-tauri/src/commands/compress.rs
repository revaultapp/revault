use crate::core::compression;
use std::path::Path;

#[tauri::command]
pub fn compress_images(
    paths: Vec<String>,
    quality: f32,
) -> Result<Vec<compression::CompressionResult>, String> {
    let mut results = Vec::with_capacity(paths.len());

    for path in &paths {
        let input = Path::new(path);
        let stem = input
            .file_stem()
            .ok_or_else(|| format!("invalid filename: {path}"))?;
        let parent = input
            .parent()
            .ok_or_else(|| format!("invalid path: {path}"))?;
        let output = parent.join(format!("{}_compressed.jpg", stem.to_string_lossy()));

        let result = compression::compress_jpeg(path, &output.to_string_lossy(), quality)
            .map_err(|e| e.to_string())?;
        results.push(result);
    }

    Ok(results)
}
