//! PDF page rasterization via the bundled pdfium dynamic library.
//!
//! All Pdfium access is serialized through a single dedicated thread: the
//! `thread_safe` cargo feature of pdfium-render is disabled on purpose (its
//! blanket `Send + Sync` impl is unsound — upstream issue #262, segfaults
//! under concurrent access). The actor pattern below makes the bug
//! unreachable by construction: the `Pdfium` instance is created on the
//! render thread and never leaves it.

use crate::core::image_io::{checked_size, MAX_IMAGE_DIMENSION};
use pdfium_render::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSelection {
    All,
    Range { start: u32, end: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterFormat {
    Jpg,
    Png,
}

#[derive(Debug, Clone, Copy)]
pub struct PdfToImagesOptions {
    pub pages: PageSelection,
    pub dpi: u32,
    pub format: RasterFormat,
}

#[derive(Debug, Clone, Serialize)]
pub struct RasterizeProgress {
    pub current: u32,
    pub total: u32,
}

fn ext_for(format: RasterFormat) -> &'static str {
    match format {
        RasterFormat::Jpg => "jpg",
        RasterFormat::Png => "png",
    }
}

fn validate_dpi(dpi: u32) -> Result<(), String> {
    if !(72..=600).contains(&dpi) {
        return Err(format!("DPI {dpi} is out of range (72-600)"));
    }
    Ok(())
}

/// dpi/72 scale, clamped so the longest page side never exceeds
/// `MAX_IMAGE_DIMENSION` pixels — an A0 poster at 600 DPI degrades gracefully
/// instead of attempting a multi-gigabyte allocation.
fn raster_scale(page_w_pt: f32, page_h_pt: f32, dpi: u32) -> f32 {
    let scale = dpi as f32 / 72.0;
    let longest_pt = page_w_pt.max(page_h_pt).max(1.0);
    let cap = MAX_IMAGE_DIMENSION as f32 / longest_pt;
    // Floor FIRST, then cap — so the anti-decompression-bomb cap always wins.
    // A page with an extreme MediaBox (cap < 0.01) must not be floored back up
    // past the cap, or px_w would exceed MAX_IMAGE_DIMENSION.
    scale.max(0.01).min(cap)
}

/// Upper bound on pages rendered in one job — caps how long a single crafted
/// PDF can occupy the sole render actor and how many files it can spew.
const MAX_PAGES_PER_JOB: u32 = 2000;

fn resolve_page_range(selection: PageSelection, page_count: u32) -> Result<(u32, u32), String> {
    if page_count == 0 {
        return Err("PDF has no pages".to_string());
    }
    let (start, end) = match selection {
        PageSelection::All => (1, page_count),
        PageSelection::Range { start, end } => {
            if start < 1 || end < start || end > page_count {
                return Err(format!(
                    "page range {start}-{end} out of bounds (document has {page_count} pages)"
                ));
            }
            (start, end)
        }
    };
    if end - start + 1 > MAX_PAGES_PER_JOB {
        return Err(format!(
            "too many pages to convert at once ({}); limit is {MAX_PAGES_PER_JOB} — use a page range",
            end - start + 1
        ));
    }
    Ok((start, end))
}

fn map_pdfium_load_err(e: PdfiumError) -> String {
    let msg = format!("{e:?}");
    if msg.contains("Password") {
        "This PDF is password-protected — unlock it before converting".to_string()
    } else if msg.contains("Format") || msg.contains("File") {
        "This file is not a valid PDF or is corrupted".to_string()
    } else {
        format!("Cannot open PDF: {msg}")
    }
}

struct RenderRequest {
    lib_dir: PathBuf,
    input: String,
    output_dir: Option<String>,
    opts: PdfToImagesOptions,
    cancelled: Arc<AtomicBool>,
    progress: Box<dyn Fn(RasterizeProgress) + Send>,
    reply: mpsc::Sender<Result<Vec<String>, String>>,
}

static RENDER_TX: Mutex<Option<mpsc::Sender<RenderRequest>>> = Mutex::new(None);

fn render_sender() -> Result<mpsc::Sender<RenderRequest>, String> {
    let mut guard = RENDER_TX
        .lock()
        .map_err(|_| "PDF render thread lock poisoned".to_string())?;
    if let Some(tx) = guard.as_ref() {
        return Ok(tx.clone());
    }
    let (tx, rx) = mpsc::channel::<RenderRequest>();
    std::thread::Builder::new()
        .name("pdfium-render".to_string())
        .spawn(move || render_thread_main(rx))
        .map_err(|e| format!("Failed to start PDF render thread: {e}"))?;
    *guard = Some(tx.clone());
    Ok(tx)
}

fn render_thread_main(rx: mpsc::Receiver<RenderRequest>) {
    // Bound lazily on the first request (binding needs the library path) and
    // cached for the thread's lifetime; the dylib stays loaded.
    let mut pdfium: Option<Pdfium> = None;
    while let Ok(req) = rx.recv() {
        // catch_unwind so a panic inside the pdfium FFI / image-encode path
        // (mirroring the mozjpeg guard in core/compression.rs) becomes an Err
        // reply instead of killing this actor thread — a dead thread would
        // leave the cached RENDER_TX sender permanently disconnected, breaking
        // the feature for the rest of the app session.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle_request(&mut pdfium, &req)
        }))
        .unwrap_or_else(|payload| {
            // A panic mid-render may leave the Pdfium/document state
            // inconsistent — drop the binding so the next request rebinds clean.
            pdfium = None;
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            Err(format!("Internal error while rendering PDF: {msg}"))
        });
        let _ = req.reply.send(result);
    }
}

fn handle_request(
    pdfium_slot: &mut Option<Pdfium>,
    req: &RenderRequest,
) -> Result<Vec<String>, String> {
    let pdfium: &Pdfium = match pdfium_slot {
        Some(p) => p,
        None => {
            let lib_path = Pdfium::pdfium_platform_library_name_at_path(&req.lib_dir);
            let bindings = Pdfium::bind_to_library(&lib_path).map_err(|e| {
                format!(
                    "PDF rendering component missing or unloadable at '{}' — reinstall the app \
                     (dev: run scripts/fetch-pdfium.sh): {e:?}",
                    lib_path.display()
                )
            })?;
            pdfium_slot.insert(Pdfium::new(bindings))
        }
    };

    // Early check so a cancel issued while the request sat in the actor's
    // queue skips validation/probing entirely (symmetry with compress_video
    // and extract_audio's up-front checks).
    if req.cancelled.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    crate::core::paths::validate_input_path(&req.input, false)?;
    checked_size(&req.input).map_err(|e| e.to_string())?;

    let doc = pdfium
        .load_pdf_from_file(&req.input, None)
        .map_err(map_pdfium_load_err)?;
    let page_count = doc.pages().len() as u32;
    let (start, end) = resolve_page_range(req.opts.pages, page_count)?;

    let input_path = Path::new(&req.input);
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");
    let dir = crate::core::paths::resolve_output_dir(
        req.output_dir.as_deref(),
        input_path.parent().unwrap_or(Path::new(".")),
    )?;
    let ext = ext_for(req.opts.format);

    let total = end - start + 1;
    let mut reserved = HashSet::new();
    let mut outputs: Vec<PathBuf> = Vec::with_capacity(total as usize);

    // The whole per-page loop funnels through one Result so that ANY exit —
    // cancellation OR a mid-batch failure (malformed page, encode error,
    // write error) — removes everything already written. "No half-finished
    // batches" must hold on the error paths too, or a failure on page 7 of 10
    // strands 6 orphaned files the frontend never learns about.
    let render_result: Result<(), String> = (|| {
        for (i, n) in (start..=end).enumerate() {
            if req.cancelled.load(Ordering::SeqCst) {
                return Err("cancelled".to_string());
            }

            let page = doc
                .pages()
                .get((n - 1) as i32)
                .map_err(|e| format!("Cannot read page {n}: {e:?}"))?;
            let scale = raster_scale(page.width().value, page.height().value, req.opts.dpi);
            let px_w = ((page.width().value * scale).round() as i32).max(1);
            let config = PdfRenderConfig::new().set_target_width(px_w);

            let bitmap = page
                .render_with_config(&config)
                .map_err(|e| format!("Failed to render page {n}: {e:?}"))?;
            let rgb = bitmap
                .as_image()
                .map_err(|e| format!("Failed to render page {n}: {e:?}"))?
                .to_rgb8();

            let bytes = match req.opts.format {
                RasterFormat::Jpg => crate::core::compression::encode_jpeg_bytes(
                    rgb.width() as usize,
                    rgb.height() as usize,
                    rgb.as_raw(),
                    crate::core::pdf::PDF_JPEG_QUALITY,
                )
                .map_err(|e| format!("Failed to encode page {n}: {e}"))?,
                RasterFormat::Png => {
                    // Plain image-crate PNG encode, deliberately NOT the
                    // oxipng/zopfli pipeline the Optimize feature uses: zopfli
                    // is slow by design and a job may span hundreds of pages —
                    // per-page zopfli would turn a quick export into minutes.
                    let mut buf = Vec::new();
                    image::DynamicImage::ImageRgb8(rgb)
                        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                        .map_err(|e| format!("Failed to encode page {n}: {e}"))?;
                    buf
                }
            };

            let out = crate::core::paths::first_available_path(
                &dir.join(format!("{stem}_page_{n}.{ext}")),
                &mut reserved,
            );
            std::fs::write(&out, &bytes).map_err(|e| format!("Failed to write page {n}: {e}"))?;
            outputs.push(out);
            (req.progress)(RasterizeProgress {
                current: (i as u32) + 1,
                total,
            });
        }
        Ok(())
    })();

    if let Err(e) = render_result {
        for p in &outputs {
            let _ = std::fs::remove_file(p);
        }
        return Err(e);
    }

    Ok(outputs
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

pub fn pdf_to_images(
    lib_dir: &Path,
    input: &str,
    output_dir: Option<&str>,
    opts: PdfToImagesOptions,
    cancelled: Arc<AtomicBool>,
    progress_cb: impl Fn(RasterizeProgress) + Send + 'static,
) -> Result<Vec<String>, String> {
    validate_dpi(opts.dpi)?;
    let (reply_tx, reply_rx) = mpsc::channel();
    let request = RenderRequest {
        lib_dir: lib_dir.to_path_buf(),
        input: input.to_string(),
        output_dir: output_dir.map(String::from),
        opts,
        cancelled,
        progress: Box::new(progress_cb),
        reply: reply_tx,
    };
    render_sender()?
        .send(request)
        .map_err(|_| "PDF render thread is not available".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "PDF render thread terminated unexpectedly".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_bounds_are_enforced() {
        assert!(validate_dpi(71).is_err());
        assert!(validate_dpi(72).is_ok());
        assert!(validate_dpi(150).is_ok());
        assert!(validate_dpi(600).is_ok());
        assert!(validate_dpi(601).is_err());
    }

    #[test]
    fn raster_scale_matches_dpi_for_normal_pages() {
        // A4 portrait: 595.28 x 841.89 pt. At 150 DPI the long side is
        // ~1754 px — far below the cap, so scale is exactly dpi/72.
        let s = raster_scale(595.28, 841.89, 150);
        assert!((s - 150.0 / 72.0).abs() < 1e-4, "got {s}");
    }

    #[test]
    fn raster_scale_clamps_giant_pages() {
        // A page 20,000 pt wide at 300 DPI would be ~83,000 px. The clamp
        // must bring the longest side down to MAX_IMAGE_DIMENSION.
        let s = raster_scale(20_000.0, 500.0, 300);
        let longest = 20_000.0 * s;
        assert!(
            longest <= MAX_IMAGE_DIMENSION as f32 + 1.0,
            "longest side {longest} exceeds cap"
        );
        assert!(s < 300.0 / 72.0);
    }

    #[test]
    fn raster_scale_cap_wins_for_extreme_mediabox() {
        // A malicious PDF declaring a ~10,000,000pt MediaBox: cap (~0.0008)
        // drops far below the 0.01 floor. The floor must NOT override the cap,
        // or px would blow past MAX_IMAGE_DIMENSION into a multi-GB allocation.
        let s = raster_scale(10_000_000.0, 10_000_000.0, 150);
        let longest_px = 10_000_000.0 * s;
        assert!(
            longest_px <= MAX_IMAGE_DIMENSION as f32 + 1.0,
            "longest side {longest_px}px exceeds cap {MAX_IMAGE_DIMENSION}"
        );
    }

    #[test]
    fn page_range_all_covers_document() {
        assert_eq!(resolve_page_range(PageSelection::All, 5).unwrap(), (1, 5));
    }

    #[test]
    fn page_range_rejects_over_the_per_job_cap() {
        // A crafted PDF with a huge page tree must not tie up the actor.
        let err = resolve_page_range(PageSelection::All, MAX_PAGES_PER_JOB + 1).unwrap_err();
        assert!(err.contains("too many pages"), "got: {err}");
        // Exactly at the cap is allowed.
        assert!(resolve_page_range(PageSelection::All, MAX_PAGES_PER_JOB).is_ok());
        // A bounded range within a huge document is fine.
        assert_eq!(
            resolve_page_range(PageSelection::Range { start: 1, end: 10 }, 100_000).unwrap(),
            (1, 10)
        );
    }

    #[test]
    fn page_range_validates_bounds() {
        let err = resolve_page_range(PageSelection::Range { start: 3, end: 9 }, 5).unwrap_err();
        assert!(err.contains("out of bounds"));
        assert!(err.contains("5 pages"));
        assert!(resolve_page_range(PageSelection::Range { start: 0, end: 2 }, 5).is_err());
        assert!(resolve_page_range(PageSelection::Range { start: 4, end: 2 }, 5).is_err());
        assert_eq!(
            resolve_page_range(PageSelection::Range { start: 2, end: 4 }, 5).unwrap(),
            (2, 4)
        );
    }

    #[test]
    fn empty_document_is_rejected() {
        assert!(resolve_page_range(PageSelection::All, 0).is_err());
    }

    #[test]
    fn extension_follows_format() {
        assert_eq!(ext_for(RasterFormat::Jpg), "jpg");
        assert_eq!(ext_for(RasterFormat::Png), "png");
    }

    #[test]
    fn password_errors_map_to_friendly_message() {
        let msg = map_pdfium_load_err(PdfiumError::PdfiumLibraryInternalError(
            PdfiumInternalError::PasswordError,
        ));
        assert!(msg.contains("password-protected"), "got: {msg}");
    }

    /// Real end-to-end render through the actor + the actual pdfium dylib.
    /// Ignored in CI (no dylib there); run locally with:
    ///   REVAULT_PDFIUM_PATH=resources/pdfium/libpdfium.dylib cargo test -- --ignored
    #[test]
    #[ignore]
    fn pdf_to_images_renders_real_pdf() {
        let lib_path = std::env::var("REVAULT_PDFIUM_PATH")
            .expect("set REVAULT_PDFIUM_PATH to the pdfium dylib to run this test");
        let lib_dir = std::path::Path::new(&lib_path)
            .parent()
            .expect("lib path has no parent")
            .to_path_buf();

        // Two-page fixture built with lopdf: 200x100pt and 300x300pt pages.
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("fixture.pdf");
        {
            use lopdf::{dictionary, Document, Object};
            let mut doc = Document::with_version("1.4");
            let pages_id = doc.new_object_id();
            let mut kids = Vec::new();
            for (w, h) in [(200, 100), (300, 300)] {
                let page_id = doc.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), w.into(), h.into()],
                });
                kids.push(Object::Reference(page_id));
            }
            doc.objects.insert(
                pages_id,
                Object::Dictionary(dictionary! {
                    "Type" => "Pages", "Kids" => kids, "Count" => 2,
                }),
            );
            let catalog_id = doc.add_object(dictionary! {
                "Type" => "Catalog", "Pages" => pages_id,
            });
            doc.trailer.set("Root", catalog_id);
            let mut buf = Vec::new();
            doc.save_to(&mut buf).unwrap();
            std::fs::write(&input, &buf).unwrap();
        }

        let opts = PdfToImagesOptions {
            pages: PageSelection::All,
            dpi: 144, // scale 2.0 → page 1 should be ~400px wide
            format: RasterFormat::Png,
        };
        let outputs = pdf_to_images(
            &lib_dir,
            input.to_str().unwrap(),
            None,
            opts,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap();

        assert_eq!(outputs.len(), 2);
        let img1 = image::open(&outputs[0]).unwrap();
        assert!(
            (img1.width() as i64 - 400).abs() <= 2,
            "page 1 width {} ≉ 400",
            img1.width()
        );
        // Second render request through the same (already-bound) actor must work.
        let again = pdf_to_images(
            &lib_dir,
            input.to_str().unwrap(),
            None,
            PdfToImagesOptions {
                pages: PageSelection::Range { start: 2, end: 2 },
                dpi: 72,
                format: RasterFormat::Jpg,
            },
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap();
        assert_eq!(again.len(), 1);
    }
}
