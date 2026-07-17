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

#[tauri::command]
pub async fn merge_pdfs(
    paths: Vec<String>,
    output_dir: Option<String>,
) -> Result<pdf::MergeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        pdf::merge_pdfs(&paths, output_dir.as_deref()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn images_to_pdf(
    paths: Vec<String>,
    output_dir: Option<String>,
    page_size: String,
    margin: String,
) -> Result<pdf::ImagesToPdfResult, String> {
    let page_size = match page_size.as_str() {
        "fit" => pdf::PageSize::Fit,
        "a4" => pdf::PageSize::A4,
        "letter" => pdf::PageSize::Letter,
        other => return Err(format!("unknown page size: {other}")),
    };
    let margin = match margin.as_str() {
        "none" => pdf::PageMargin::None,
        "small" => pdf::PageMargin::Small,
        "big" => pdf::PageMargin::Big,
        other => return Err(format!("unknown margin: {other}")),
    };
    tauri::async_runtime::spawn_blocking(move || {
        pdf::images_to_pdf(
            &paths,
            output_dir.as_deref(),
            pdf::ImagesToPdfOptions { page_size, margin },
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn split_pdf(
    input: String,
    mode: String,
    start: Option<u32>,
    end: Option<u32>,
    output_dir: Option<String>,
) -> Result<Vec<String>, String> {
    let split_mode = match mode.as_str() {
        "each" => pdf::SplitMode::EachPage,
        "range" => pdf::SplitMode::Range {
            start: start.ok_or("start is required for range mode")?,
            end: end.ok_or("end is required for range mode")?,
        },
        other => return Err(format!("unknown split mode: {other}")),
    };
    tauri::async_runtime::spawn_blocking(move || {
        pdf::split_pdf(&input, split_mode, output_dir.as_deref()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
