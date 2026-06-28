use crate::core::pdf;

#[tauri::command]
pub async fn reveal_pdf_output(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::core::video::reveal_in_file_manager(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn process_pdfs(
    paths: Vec<String>,
    output_dir: Option<String>,
    strip_metadata: bool,
    compress_streams: bool,
    compress_images: bool,
) -> Result<Vec<pdf::PdfResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(pdf::process_batch(
            &paths,
            output_dir.as_deref(),
            pdf::PdfOptions {
                strip_metadata,
                compress_streams,
                compress_images,
            },
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
