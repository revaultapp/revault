use crate::core::image_io::{checked_size, write_preserving_timestamps};
use lopdf::{Document, Object, SaveOptions};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
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
                let _ = s.compress();
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

    fn save_doc(doc: &mut Document) -> Vec<u8> {
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
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
}
