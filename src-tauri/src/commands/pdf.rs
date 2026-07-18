use crate::core::pdf;
use crate::core::pdf_render;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::path::BaseDirectory;
use tauri::{Emitter, Manager};

static PDF_RENDER_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

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

// IPC boundary: each param is deserialized by name from the JS invoke call,
// mirroring the flat-argument shape of the other PDF commands (split_pdf etc.).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn pdf_to_images(
    app: tauri::AppHandle,
    input: String,
    pages_mode: String,
    start: Option<u32>,
    end: Option<u32>,
    dpi: u32,
    format: String,
    output_dir: Option<String>,
) -> Result<Vec<String>, String> {
    let pages = match pages_mode.as_str() {
        "all" => pdf_render::PageSelection::All,
        "range" => pdf_render::PageSelection::Range {
            start: start.ok_or("start is required for range mode")?,
            end: end.ok_or("end is required for range mode")?,
        },
        other => return Err(format!("unknown pages mode: {other}")),
    };
    let format = match format.as_str() {
        "jpg" => pdf_render::RasterFormat::Jpg,
        "png" => pdf_render::RasterFormat::Png,
        other => return Err(format!("unknown image format: {other}")),
    };
    // Bundled dylib directory. In debug builds only, REVAULT_PDFIUM_PATH (the
    // library FILE) overrides it for dev without a full bundle — gated behind
    // debug_assertions so release binaries can never be coaxed into dlopen-ing
    // an attacker-controlled dylib via the process environment.
    #[cfg(debug_assertions)]
    let dev_override: Option<PathBuf> = std::env::var("REVAULT_PDFIUM_PATH")
        .ok()
        .and_then(|p| PathBuf::from(&p).parent().map(|d| d.to_path_buf()));
    #[cfg(not(debug_assertions))]
    let dev_override: Option<PathBuf> = None;

    let lib_dir: PathBuf = match dev_override {
        Some(dir) => dir,
        None => app
            .path()
            .resolve("resources/pdfium", BaseDirectory::Resource)
            .map_err(|e| e.to_string())?,
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut active = PDF_RENDER_CANCEL.lock().map_err(|e| e.to_string())?;
        if active.is_some() {
            return Err("PDF conversion already running".to_string());
        }
        *active = Some(cancel_flag.clone());
    }
    let cancel_for_worker = cancel_flag.clone();
    let app_for_emit = app.clone();

    let join_result = tauri::async_runtime::spawn_blocking(move || {
        pdf_render::pdf_to_images(
            &lib_dir,
            &input,
            output_dir.as_deref(),
            pdf_render::PdfToImagesOptions { pages, dpi, format },
            cancel_for_worker,
            move |progress| {
                let _ = app_for_emit.emit("pdf-rasterize-progress", &progress);
            },
        )
    })
    .await;

    let mut active = PDF_RENDER_CANCEL.lock().map_err(|e| e.to_string())?;
    if active
        .as_ref()
        .map(|flag| Arc::ptr_eq(flag, &cancel_flag))
        .unwrap_or(false)
    {
        *active = None;
    }
    join_result.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cancel_pdf_to_images() -> Result<(), String> {
    if let Some(flag) = PDF_RENDER_CANCEL
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
    {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
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
