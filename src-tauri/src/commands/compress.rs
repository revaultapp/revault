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
        _ => OutputFormat::Jpeg,
    }
}

#[tauri::command]
pub fn compress_images(
    paths: Vec<String>,
    quality: f32,
    format: Option<OutputFormat>,
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

        let fmt = format.clone().unwrap_or_else(|| detect_format(path));
        let ext = match fmt {
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Png => "png",
            OutputFormat::Webp => "webp",
        };
        let output = parent.join(format!("{}_compressed.{ext}", stem.to_string_lossy()));

        let result = compression::compress_image(path, &output.to_string_lossy(), &fmt, quality)
            .map_err(|e| e.to_string())?;
        results.push(result);
    }

    Ok(results)
}
