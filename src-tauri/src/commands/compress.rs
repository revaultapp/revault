use crate::core::compression::{self, OutputFormat};
use std::path::Path;

fn detect_format(path: &str) -> OutputFormat {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("png") => OutputFormat::Png,
        Some("webp") => OutputFormat::Webp,
        Some("heic") | Some("heif") => OutputFormat::Jpeg,
        _ => OutputFormat::Jpeg,
    }
}

#[tauri::command]
pub async fn compress_images(
    paths: Vec<String>,
    quality: f32,
    format: Option<OutputFormat>,
    output_dir: Option<String>,
) -> Result<Vec<compression::CompressionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut results = Vec::with_capacity(paths.len());

        for path in &paths {
            let input = Path::new(path);
            let stem = match input.file_stem() {
                Some(s) => s,
                None => {
                    results.push(compression::CompressionResult {
                        input_path: path.to_string(),
                        output_path: String::new(),
                        original_size: 0,
                        compressed_size: 0,
                        error: Some(format!("invalid filename: {path}")),
                    });
                    continue;
                }
            };
            let parent = match input.parent() {
                Some(p) => p,
                None => {
                    results.push(compression::CompressionResult {
                        input_path: path.to_string(),
                        output_path: String::new(),
                        original_size: 0,
                        compressed_size: 0,
                        error: Some(format!("invalid path: {path}")),
                    });
                    continue;
                }
            };

            let fmt = format.clone().unwrap_or_else(|| detect_format(path));
            let ext = match fmt {
                OutputFormat::Jpeg => "jpg",
                OutputFormat::Png => "png",
                OutputFormat::Webp => "webp",
            };
            let out_base = output_dir.as_deref().map(Path::new).unwrap_or(parent);
            let output = out_base.join(format!("{}_compressed.{ext}", stem.to_string_lossy()));

            match compression::compress_image(path, &output.to_string_lossy(), &fmt, quality) {
                Ok(result) => results.push(result),
                Err(e) => results.push(compression::CompressionResult {
                    input_path: path.to_string(),
                    output_path: String::new(),
                    original_size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                    compressed_size: 0,
                    error: Some(e.to_string()),
                }),
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}
