use crate::core::image_io::{checked_size, write_preserving_timestamps};
use lopdf::{dictionary, Dictionary, Document, Object, SaveOptions};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct PdfResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub output_size: u64,
    pub error: Option<String>,
}

impl PdfResult {
    fn ok(input: &str, output: &str, original_size: u64, output_size: u64) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: output.to_string(),
            original_size,
            output_size,
            error: None,
        }
    }

    fn err(input: &str, msg: String) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: String::new(),
            original_size: 0,
            output_size: 0,
            error: Some(msg),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PdfOptions {
    pub strip_metadata: bool,
    pub compress_streams: bool,
    pub compress_images: bool,
}

pub fn process_pdf(
    input: &str,
    output: &str,
    opts: PdfOptions,
) -> Result<PdfResult, Box<dyn Error>> {
    crate::core::paths::validate_input_path(input, false)?;
    let original_size = checked_size(input)?;
    let mut doc = Document::load(input)?;

    if opts.strip_metadata {
        doc.trailer.remove(b"Info");
        if let Ok(catalog) = doc.catalog_mut() {
            catalog.remove(b"Metadata");
        }
    }

    if opts.compress_streams {
        doc.traverse_objects(|obj| {
            if let Object::Stream(s) = obj {
                if let Err(e) = s.compress() {
                    eprintln!("PDF stream compression failed, leaving stream uncompressed: {e}");
                }
            }
        });
    }

    if opts.compress_images {
        compress_pdf_images(&mut doc);
    }

    let mut buffer = Vec::new();
    if opts.compress_streams {
        doc.save_with_options(
            &mut buffer,
            SaveOptions::builder()
                .use_object_streams(true)
                .compression_level(6)
                .build(),
        )?;
    } else {
        doc.save_to(&mut buffer)?;
    }

    write_preserving_timestamps(input, output, &buffer)?;
    let output_size = std::fs::metadata(output)?.len();

    Ok(PdfResult::ok(input, output, original_size, output_size))
}

fn first_available_path(base: &Path, reserved: &mut HashSet<PathBuf>) -> PathBuf {
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("pdf");

    for n in 1..10_000 {
        let candidate = if n == 1 {
            base.to_path_buf()
        } else {
            parent.join(format!("{stem}_{n}.{ext}"))
        };
        if !candidate.exists() && reserved.insert(candidate.clone()) {
            return candidate;
        }
    }

    base.to_path_buf()
}

fn build_output_path(
    input: &str,
    output_dir: Option<&str>,
    reserved: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("pdf");
    let dir = match output_dir {
        Some(d) => {
            let canon = std::fs::canonicalize(d)
                .map_err(|e| format!("Invalid output dir '{}': {}", d, e))?;
            if !canon.is_dir() {
                return Err(format!("Output path is not a directory: {}", d));
            }
            canon
        }
        None => p.parent().unwrap_or(Path::new(".")).to_path_buf(),
    };
    let output = first_available_path(&dir.join(format!("{stem}_private.{ext}")), reserved);
    output
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid output path".to_string())
}

fn build_output_paths(paths: &[String], output_dir: Option<&str>) -> Vec<Result<String, String>> {
    let mut reserved = HashSet::new();
    paths
        .iter()
        .map(|input| build_output_path(input, output_dir, &mut reserved))
        .collect()
}

pub fn process_batch(
    paths: &[String],
    output_dir: Option<&str>,
    opts: PdfOptions,
) -> Vec<PdfResult> {
    let outputs = build_output_paths(paths, output_dir);
    paths
        .par_iter()
        .zip(outputs.into_par_iter())
        .map(|(input, output)| match output {
            Ok(output) => match process_pdf(input, &output, opts) {
                Ok(r) => r,
                Err(e) => PdfResult::err(input, e.to_string()),
            },
            Err(e) => PdfResult::err(input, e),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct MergeResult {
    pub output_path: String,
    pub output_size: u64,
    pub page_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum SplitMode {
    Range { start: u32, end: u32 },
    EachPage,
}

fn resolve_output_dir(output_dir: Option<&str>, fallback: &Path) -> Result<PathBuf, String> {
    match output_dir {
        Some(d) => {
            let canon = std::fs::canonicalize(d)
                .map_err(|e| format!("Invalid output dir '{}': {}", d, e))?;
            if !canon.is_dir() {
                return Err(format!("Output path is not a directory: {}", d));
            }
            Ok(canon)
        }
        None => Ok(fallback.to_path_buf()),
    }
}

// Page objects can inherit MediaBox from an ancestor Pages node instead of
// declaring their own. Merging flattens the Pages tree (every page gets a
// single fresh Parent), so an inherited box must be baked onto the page
// itself first or it silently reverts to the reader's default page size.
fn inherited_media_box(doc: &Document, page_dict: &Dictionary) -> Option<Object> {
    let mut parent_ref = page_dict
        .get(b"Parent")
        .ok()
        .and_then(|o| o.as_reference().ok());
    while let Some(id) = parent_ref {
        let parent_dict = doc.objects.get(&id).and_then(|o| o.as_dict().ok())?;
        if let Ok(mb) = parent_dict.get(b"MediaBox") {
            return Some(mb.clone());
        }
        parent_ref = parent_dict
            .get(b"Parent")
            .ok()
            .and_then(|o| o.as_reference().ok());
    }
    None
}

pub fn merge_pdfs(
    paths: &[String],
    output_dir: Option<&str>,
) -> Result<MergeResult, Box<dyn Error>> {
    if paths.is_empty() {
        return Err("no input paths provided".into());
    }
    for p in paths {
        crate::core::paths::validate_input_path(p, false)?;
        checked_size(p)?;
    }

    let mut version = "1.4".to_string();
    let mut next_id = 1u32;
    let mut merged_objects: BTreeMap<lopdf::ObjectId, Object> = BTreeMap::new();
    let mut merged_pages: Vec<(lopdf::ObjectId, Object)> = Vec::new();

    for path in paths {
        let mut doc = Document::load(path)?;
        if doc.version > version {
            version = doc.version.clone();
        }
        doc.renumber_objects_with(next_id);
        next_id = doc.max_id + 1;

        for page_id in doc.get_pages().into_values() {
            let Some(page_obj) = doc.objects.get(&page_id) else {
                continue;
            };
            let mut page_obj = page_obj.clone();
            let needs_media_box = matches!(page_obj.as_dict(), Ok(dict) if !dict.has(b"MediaBox"));
            if needs_media_box {
                let mb = page_obj
                    .as_dict()
                    .ok()
                    .and_then(|dict| inherited_media_box(&doc, dict));
                if let Some(mb) = mb {
                    if let Ok(dict) = page_obj.as_dict_mut() {
                        dict.set("MediaBox", mb);
                    }
                }
            }
            merged_pages.push((page_id, page_obj));
        }

        for (id, obj) in doc.objects {
            if !matches!(obj.type_name(), Ok(b"Page") | Ok(b"Pages") | Ok(b"Catalog")) {
                merged_objects.insert(id, obj);
            }
        }
    }

    let mut document = Document::with_version(version);
    document.max_id = next_id - 1;
    document.objects = merged_objects;

    let pages_id = document.new_object_id();
    let kids: Vec<Object> = merged_pages
        .iter()
        .map(|(id, _)| Object::Reference(*id))
        .collect();
    let page_count = kids.len();

    for (page_id, obj) in merged_pages {
        if let Ok(dict) = obj.as_dict() {
            let mut dict = dict.clone();
            dict.set("Parent", pages_id);
            document.objects.insert(page_id, Object::Dictionary(dict));
        }
    }

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => page_count as i64,
        }),
    );

    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let first_input = Path::new(&paths[0]);
    let first_stem = first_input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let fallback_dir = first_input.parent().unwrap_or(Path::new("."));
    let dir = resolve_output_dir(output_dir, fallback_dir)?;
    let mut reserved = HashSet::new();
    let output = first_available_path(&dir.join(format!("{first_stem}_merged.pdf")), &mut reserved);

    let mut buffer = Vec::new();
    document.save_to(&mut buffer)?;
    std::fs::write(&output, &buffer)?;
    let output_size = buffer.len() as u64;

    Ok(MergeResult {
        output_path: output.to_string_lossy().into_owned(),
        output_size,
        page_count,
    })
}

pub fn split_pdf(
    input: &str,
    mode: SplitMode,
    output_dir: Option<&str>,
) -> Result<Vec<String>, Box<dyn Error>> {
    crate::core::paths::validate_input_path(input, false)?;
    checked_size(input)?;
    let doc = Document::load(input)?;
    let page_count = doc.get_pages().len() as u32;

    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let fallback_dir = p.parent().unwrap_or(Path::new("."));
    let dir = resolve_output_dir(output_dir, fallback_dir)?;
    let mut reserved = HashSet::new();

    match mode {
        SplitMode::Range { start, end } => {
            if start < 1 || end < start || end > page_count {
                return Err(format!(
                    "page range {start}-{end} out of bounds (document has {page_count} pages)"
                )
                .into());
            }
            let output = first_available_path(
                &dir.join(format!("{stem}_pages_{start}-{end}.pdf")),
                &mut reserved,
            );
            let mut sub = doc.clone();
            let to_delete: Vec<u32> = (1..=page_count)
                .filter(|n| *n < start || *n > end)
                .collect();
            sub.delete_pages(&to_delete);
            let mut buffer = Vec::new();
            sub.save_to(&mut buffer)?;
            std::fs::write(&output, &buffer)?;
            Ok(vec![output.to_string_lossy().into_owned()])
        }
        SplitMode::EachPage => {
            let mut outputs = Vec::with_capacity(page_count as usize);
            for n in 1..=page_count {
                let output =
                    first_available_path(&dir.join(format!("{stem}_page_{n}.pdf")), &mut reserved);
                let mut sub = Document::load(input)?;
                let to_delete: Vec<u32> = (1..=page_count).filter(|p| *p != n).collect();
                sub.delete_pages(&to_delete);
                let mut buffer = Vec::new();
                sub.save_to(&mut buffer)?;
                std::fs::write(&output, &buffer)?;
                outputs.push(output.to_string_lossy().into_owned());
            }
            Ok(outputs)
        }
    }
}

fn compress_pdf_images(doc: &mut Document) -> u32 {
    let page_ids: Vec<lopdf::ObjectId> = doc.get_pages().values().copied().collect();
    let mut count = 0u32;

    for page_id in page_ids {
        let candidates: Vec<(lopdf::ObjectId, Vec<String>)> = match doc.get_page_images(page_id) {
            Ok(images) => images
                .iter()
                .filter_map(|img| {
                    // Skip CMYK — color space conversion is complex and risky
                    if let Some(ref cs) = img.color_space {
                        if cs.contains("CMYK") {
                            return None;
                        }
                    }
                    // Skip images with SMask (alpha channel)
                    if img.origin_dict.get(b"SMask").is_ok() {
                        return None;
                    }
                    img.filters.as_ref().map(|f| (img.id, f.clone()))
                })
                .filter(|(_, filters)| filters.iter().any(|f| f == "DCTDecode"))
                .collect(),
            Err(_) => continue,
        };

        for (obj_id, _) in candidates {
            let jpeg_bytes = match doc
                .objects
                .get(&obj_id)
                .and_then(|o| o.as_stream().ok())
                .map(|s| s.content.clone())
            {
                Some(b) => b,
                None => continue,
            };

            if let Ok(new_bytes) = crate::core::compression::compress_jpeg_data(&jpeg_bytes, 78.0) {
                if let Some(obj) = doc.objects.get_mut(&obj_id) {
                    if let Ok(stream) = obj.as_stream_mut() {
                        let len = new_bytes.len();
                        stream.content = new_bytes;
                        stream.dict.set("Length", Object::Integer(len as i64));
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};
    use std::fs;

    fn minimal_pdf() -> Document {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => lopdf::Object::Array(vec![]),
            "Count" => 0,
        });
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc
    }

    fn multi_page_pdf(page_count: u32) -> Document {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for _ in 0..page_count {
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
            });
            kids.push(Object::Reference(page_id));
        }
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => kids,
                "Count" => page_count as i64,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc
    }

    fn save_doc(doc: &mut Document) -> Vec<u8> {
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn jpeg_bytes(width: usize, height: usize, quality: f32) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height * 3];
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 3;
                pixels[i] = (x % 256) as u8;
                pixels[i + 1] = (y % 256) as u8;
                pixels[i + 2] = ((x + y) % 256) as u8;
            }
        }
        let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        cinfo.set_size(width, height);
        cinfo.set_quality(quality);
        let mut cinfo = cinfo.start_compress(Vec::new()).unwrap();
        cinfo.write_scanlines(&pixels).unwrap();
        cinfo.finish().unwrap()
    }

    /// Builds a one-page PDF with a single DCTDecode (JPEG) image XObject,
    /// referenced from the page's Resources — the shape `get_page_images` expects.
    fn pdf_with_jpeg_image(jpeg: &[u8], width: i64, height: i64) -> Document {
        let mut doc = Document::with_version("1.4");

        let image_stream = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => width,
                "Height" => height,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            jpeg.to_vec(),
        );
        let image_id = doc.add_object(image_stream);

        let resources_id = doc.add_object(dictionary! {
            "XObject" => dictionary! { "Im0" => image_id },
        });

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Resources" => resources_id,
        });

        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => Object::Array(vec![Object::Reference(page_id)]),
            "Count" => 1,
        });
        doc.get_dictionary_mut(page_id)
            .unwrap()
            .set("Parent", pages_id);

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        doc
    }

    #[test]
    fn strip_metadata_removes_info_dict() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("test.pdf");

        let mut doc = minimal_pdf();
        let info_id = doc.add_object(dictionary! {
            "Author" => lopdf::Object::string_literal("Alice"),
            "Creator" => lopdf::Object::string_literal("MyApp"),
        });
        doc.trailer.set("Info", info_id);

        let data = save_doc(&mut doc);
        fs::write(&input_path, &data).unwrap();

        let output_path = dir.path().join("test_private.pdf");
        let opts = PdfOptions {
            strip_metadata: true,
            compress_streams: false,
            compress_images: false,
        };
        let result = process_pdf(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            opts,
        )
        .unwrap();

        assert!(result.error.is_none());

        let out_doc = Document::load(output_path.to_str().unwrap()).unwrap();
        assert!(out_doc.trailer.get(b"Info").is_err());
    }

    #[test]
    fn compress_reduces_uncompressed_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("test.pdf");

        let mut doc = minimal_pdf();
        // Add an uncompressed stream with repetitive content that compresses well
        let content = b"BT /F1 12 Tf 100 700 Td (Hello World Hello World Hello World Hello World Hello World) Tj ET";
        let stream = Stream::new(dictionary! {}, content.to_vec());
        doc.add_object(stream);

        let data = save_doc(&mut doc);
        fs::write(&input_path, &data).unwrap();

        let original_size = data.len() as u64;

        let output_path = dir.path().join("test_private.pdf");
        let opts = PdfOptions {
            strip_metadata: false,
            compress_streams: true,
            compress_images: false,
        };
        let result = process_pdf(
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            opts,
        )
        .unwrap();

        assert!(result.error.is_none());
        // Output should be <= original (compression never makes it larger for this content)
        assert!(result.output_size <= original_size + 512); // allow small overhead
    }

    #[test]
    fn process_batch_mixed() {
        let dir = tempfile::tempdir().unwrap();
        let valid_path = dir.path().join("valid.pdf");

        let mut doc = minimal_pdf();
        let data = save_doc(&mut doc);
        fs::write(&valid_path, &data).unwrap();

        let missing_path = dir.path().join("nonexistent.pdf");

        let paths = vec![
            valid_path.to_str().unwrap().to_string(),
            missing_path.to_str().unwrap().to_string(),
        ];
        let opts = PdfOptions {
            strip_metadata: true,
            compress_streams: false,
            compress_images: false,
        };
        let results = process_batch(&paths, None, opts);

        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_some());
    }

    #[test]
    fn compress_images_option_exists() {
        let opts = PdfOptions {
            strip_metadata: false,
            compress_streams: false,
            compress_images: true,
        };
        assert!(opts.compress_images);
    }

    #[test]
    fn compress_images_on_pdf_without_images_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        let output = dir.path().join("out.pdf");
        let data = save_doc(&mut minimal_pdf());
        fs::write(&input, &data).unwrap();
        let opts = PdfOptions {
            strip_metadata: false,
            compress_streams: false,
            compress_images: true,
        };
        let result = process_pdf(input.to_str().unwrap(), output.to_str().unwrap(), opts).unwrap();
        assert!(result.error.is_none());
    }

    #[test]
    fn compress_images_reencodes_embedded_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        let output = dir.path().join("out.pdf");

        let original_jpeg = jpeg_bytes(200, 200, 100.0);
        let mut doc = pdf_with_jpeg_image(&original_jpeg, 200, 200);
        let data = save_doc(&mut doc);
        fs::write(&input, &data).unwrap();

        let opts = PdfOptions {
            strip_metadata: false,
            compress_streams: false,
            compress_images: true,
        };
        let result = process_pdf(input.to_str().unwrap(), output.to_str().unwrap(), opts).unwrap();
        assert!(result.error.is_none());

        let out_doc = Document::load(output.to_str().unwrap()).unwrap();
        let page_id = *out_doc.get_pages().values().next().unwrap();
        let images = out_doc.get_page_images(page_id).unwrap();
        assert_eq!(images.len(), 1);
        assert!(
            images[0].content.len() < original_jpeg.len(),
            "expected re-encoded image ({}) to be smaller than original ({})",
            images[0].content.len(),
            original_jpeg.len()
        );

        let stream = out_doc
            .objects
            .get(&images[0].id)
            .unwrap()
            .as_stream()
            .unwrap();
        let declared_len = stream.dict.get(b"Length").unwrap().as_i64().unwrap();
        assert_eq!(declared_len as usize, stream.content.len());
    }

    #[test]
    fn batch_deduplicates_output_paths() {
        let dir = tempfile::tempdir().unwrap();

        // Two PDFs with the same basename in different subdirs
        let sub_a = dir.path().join("a");
        let sub_b = dir.path().join("b");
        fs::create_dir_all(&sub_a).unwrap();
        fs::create_dir_all(&sub_b).unwrap();

        let mut doc = minimal_pdf();
        let data = save_doc(&mut doc);

        let path_a = sub_a.join("report.pdf");
        let path_b = sub_b.join("report.pdf");
        fs::write(&path_a, &data).unwrap();
        fs::write(&path_b, &data).unwrap();

        let paths = vec![
            path_a.to_str().unwrap().to_string(),
            path_b.to_str().unwrap().to_string(),
        ];
        let opts = PdfOptions {
            strip_metadata: true,
            compress_streams: false,
            compress_images: false,
        };
        let results = process_batch(&paths, Some(dir.path().to_str().unwrap()), opts);

        assert_eq!(results.len(), 2);
        // Both should succeed
        assert!(
            results[0].error.is_none(),
            "first failed: {:?}",
            results[0].error
        );
        assert!(
            results[1].error.is_none(),
            "second failed: {:?}",
            results[1].error
        );
        // Output paths must differ
        assert_ne!(results[0].output_path, results[1].output_path);
    }

    #[test]
    fn merge_combines_page_counts() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.pdf");
        let path_b = dir.path().join("b.pdf");
        fs::write(&path_a, save_doc(&mut multi_page_pdf(2))).unwrap();
        fs::write(&path_b, save_doc(&mut multi_page_pdf(3))).unwrap();

        let paths = vec![
            path_a.to_str().unwrap().to_string(),
            path_b.to_str().unwrap().to_string(),
        ];
        let result = merge_pdfs(&paths, None).unwrap();

        assert_eq!(result.page_count, 5);
        let out_doc = Document::load(&result.output_path).unwrap();
        assert_eq!(out_doc.get_pages().len(), 5);
    }

    #[test]
    fn merge_single_input_is_degenerate_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.pdf");
        fs::write(&path_a, save_doc(&mut multi_page_pdf(3))).unwrap();

        let paths = vec![path_a.to_str().unwrap().to_string()];
        let result = merge_pdfs(&paths, None).unwrap();

        assert_eq!(result.page_count, 3);
    }

    #[test]
    fn merge_empty_paths_returns_error() {
        assert!(merge_pdfs(&[], None).is_err());
    }

    #[test]
    fn merge_no_clobber_when_output_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("a.pdf");
        let path_b = dir.path().join("b.pdf");
        fs::write(&path_a, save_doc(&mut multi_page_pdf(1))).unwrap();
        fs::write(&path_b, save_doc(&mut multi_page_pdf(1))).unwrap();
        // Pre-occupy the name merge_pdfs would naturally pick.
        fs::write(dir.path().join("a_merged.pdf"), b"existing").unwrap();

        let paths = vec![
            path_a.to_str().unwrap().to_string(),
            path_b.to_str().unwrap().to_string(),
        ];
        let result = merge_pdfs(&paths, None).unwrap();

        assert_ne!(
            result.output_path,
            dir.path().join("a_merged.pdf").to_str().unwrap()
        );
    }

    #[test]
    fn split_range_returns_page_subset() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("doc.pdf");
        fs::write(&input, save_doc(&mut multi_page_pdf(5))).unwrap();

        let outputs = split_pdf(
            input.to_str().unwrap(),
            SplitMode::Range { start: 2, end: 4 },
            None,
        )
        .unwrap();

        assert_eq!(outputs.len(), 1);
        let out_doc = Document::load(&outputs[0]).unwrap();
        assert_eq!(out_doc.get_pages().len(), 3);
    }

    #[test]
    fn split_range_out_of_bounds_returns_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("doc.pdf");
        fs::write(&input, save_doc(&mut multi_page_pdf(5))).unwrap();

        let err = split_pdf(
            input.to_str().unwrap(),
            SplitMode::Range { start: 3, end: 9 },
            None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("out of bounds"));
        assert!(err.to_string().contains("5 pages"));
    }

    #[test]
    fn split_each_page_produces_n_outputs() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("doc.pdf");
        fs::write(&input, save_doc(&mut multi_page_pdf(4))).unwrap();

        let outputs = split_pdf(input.to_str().unwrap(), SplitMode::EachPage, None).unwrap();

        assert_eq!(outputs.len(), 4);
        for output in &outputs {
            let out_doc = Document::load(output).unwrap();
            assert_eq!(out_doc.get_pages().len(), 1);
        }
    }

    #[test]
    fn split_no_clobber_when_output_exists() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("doc.pdf");
        fs::write(&input, save_doc(&mut multi_page_pdf(4))).unwrap();
        // Pre-occupy the name split_pdf would naturally pick for pages 1-2.
        fs::write(dir.path().join("doc_pages_1-2.pdf"), b"existing").unwrap();

        let outputs = split_pdf(
            input.to_str().unwrap(),
            SplitMode::Range { start: 1, end: 2 },
            None,
        )
        .unwrap();

        assert_ne!(
            outputs[0],
            dir.path().join("doc_pages_1-2.pdf").to_str().unwrap()
        );
    }
}
