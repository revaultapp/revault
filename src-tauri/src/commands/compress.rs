use crate::core::compression;

#[tauri::command]
pub async fn compress_images(
    paths: Vec<String>,
    quality_preset: Option<compression::QualityPreset>,
    format: Option<compression::OutputFormat>,
    output_dir: Option<String>,
    strip_gps: Option<bool>,
) -> Result<Vec<compression::CompressionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(compression::compress_batch(
            &paths,
            quality_preset.unwrap_or(compression::QualityPreset::Balanced),
            format,
            output_dir.as_deref(),
            "_compressed",
            strip_gps.unwrap_or(false),
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Preview compression on a sample of files to estimate savings.
/// - Takes ALL file paths (for accurate total size)
/// - Compresses only the first 5 (sample) for speed
/// - Returns total batch size + sample compression results
#[tauri::command]
pub async fn preview_compress(
    all_paths: Vec<String>,
    quality_preset: Option<compression::QualityPreset>,
    format: Option<compression::OutputFormat>,
) -> Result<compression::PreviewResponse, String> {
    use tempfile::TempDir;

    let preset = quality_preset.unwrap_or(compression::QualityPreset::Balanced);
    let sample_size = 5;

    tauri::async_runtime::spawn_blocking(move || {
        // First: read sizes of ALL files (fast - just metadata)
        let mut total_original_size: u64 = 0;
        let mut all_file_sizes: Vec<(String, u64)> = Vec::with_capacity(all_paths.len());

        for path in &all_paths {
            if let Ok(meta) = std::fs::metadata(path) {
                let size = meta.len();
                total_original_size += size;
                all_file_sizes.push((path.clone(), size));
            } else {
                all_file_sizes.push((path.clone(), 0));
            }
        }

        // Second: compress only the sample (top 5 largest by actual size)
        // Sort all paths by size descending, take top 5
        let mut path_size_pairs: Vec<(String, u64)> = all_file_sizes;
        path_size_pairs.sort_by_key(|p| std::cmp::Reverse(p.1)); // descending by size
        let sample_paths: Vec<String> = path_size_pairs
            .iter()
            .take(sample_size)
            .map(|(p, _)| p.clone())
            .collect();

        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        let mut sample_results = Vec::with_capacity(sample_paths.len());
        for path in &sample_paths {
            let fmt = format.unwrap_or_else(|| compression::detect_format(path));
            let output =
                match compression::resolve_output_path(path, &fmt, Some(&temp_path), "_preview") {
                    Ok(o) => o,
                    Err(e) => {
                        sample_results.push(compression::PreviewResult {
                            input_path: path.clone(),
                            original_size: 0,
                            compressed_size: 0,
                            may_increase: false,
                            error: Some(e),
                        });
                        continue;
                    }
                };

            let result = compression::compress_image(path, &output, &fmt, preset);
            let (compressed_size, error) = match result {
                Ok(r) => (r.compressed_size, r.error),
                Err(e) => (0, Some(e.to_string())),
            };

            let original_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let may_increase = error.is_none() && compressed_size >= original_size;

            sample_results.push(compression::PreviewResult {
                input_path: path.clone(),
                original_size,
                compressed_size,
                may_increase,
                error,
            });
        }

        Ok(compression::PreviewResponse {
            total_original_bytes: total_original_size,
            sample_results,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
