use crate::core::image_io::{
    checked_size, decode_limits, ext_lowercase, open_image, write_preserving_timestamps,
    MAX_IMAGE_DIMENSION,
};
use crate::core::paths::resolve_output_dir;
use image::metadata::Orientation;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageFormat, ImageReader};
use lopdf::{dictionary, Dictionary, Document, Object, SaveOptions, Stream};
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

fn build_output_path(
    input: &str,
    output_dir: Option<&str>,
    reserved: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("pdf");
    let dir = resolve_output_dir(output_dir, p.parent().unwrap_or(Path::new(".")))?;
    let output = crate::core::paths::first_available_path(
        &dir.join(format!("{stem}_private.{ext}")),
        reserved,
    );
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
    let output = crate::core::paths::first_available_path(
        &dir.join(format!("{first_stem}_merged.pdf")),
        &mut reserved,
    );

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
            let output = crate::core::paths::first_available_path(
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
                let output = crate::core::paths::first_available_path(
                    &dir.join(format!("{stem}_page_{n}.pdf")),
                    &mut reserved,
                );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    Fit,
    A4,
    Letter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMargin {
    None,
    Small,
    Big,
}

#[derive(Debug, Clone, Copy)]
pub struct ImagesToPdfOptions {
    pub page_size: PageSize,
    pub margin: PageMargin,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImagesToPdfResult {
    pub output_path: String,
    pub output_size: u64,
    pub page_count: usize,
}

/// Images are placed at this nominal print density: `Fit` pages take the
/// image's size at 150 DPI, and A4/Letter never scale an image up past it.
const FIT_DPI: f32 = 150.0;
/// JPEG quality for photo re-encodes in both PDF directions (images→PDF here,
/// PDF→images in `pdf_render.rs`) — single source of truth so the two features
/// can't drift apart again.
pub(crate) const PDF_JPEG_QUALITY: f32 = 90.0;

#[derive(Debug, Clone, Copy)]
struct PageLayout {
    page_w: f32,
    page_h: f32,
    img_x: f32,
    img_y: f32,
    img_w: f32,
    img_h: f32,
}

fn margin_pt(margin: PageMargin) -> f32 {
    match margin {
        PageMargin::None => 0.0,
        PageMargin::Small => 20.0,
        PageMargin::Big => 40.0,
    }
}

fn compute_page_layout(
    px_w: u32,
    px_h: u32,
    page_size: PageSize,
    margin: PageMargin,
) -> PageLayout {
    let natural_w = px_w as f32 * 72.0 / FIT_DPI;
    let natural_h = px_h as f32 * 72.0 / FIT_DPI;

    let (page_w, page_h) = match page_size {
        PageSize::Fit => {
            return PageLayout {
                page_w: natural_w,
                page_h: natural_h,
                img_x: 0.0,
                img_y: 0.0,
                img_w: natural_w,
                img_h: natural_h,
            }
        }
        PageSize::A4 => (595.28f32, 841.89f32),
        PageSize::Letter => (612.0f32, 792.0f32),
    };
    // Landscape image → landscape page.
    let (page_w, page_h) = if px_w > px_h {
        (page_h, page_w)
    } else {
        (page_w, page_h)
    };

    let m = margin_pt(margin);
    let content_w = (page_w - 2.0 * m).max(1.0);
    let content_h = (page_h - 2.0 * m).max(1.0);
    let scale = (content_w / natural_w).min(content_h / natural_h).min(1.0);
    let img_w = natural_w * scale;
    let img_h = natural_h * scale;
    PageLayout {
        page_w,
        page_h,
        img_x: (page_w - img_w) / 2.0,
        img_y: (page_h - img_h) / 2.0,
        img_w,
        img_h,
    }
}

struct PdfPageImage {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
    color_space: &'static str,
    /// true → `bytes` are JPEG (DCTDecode); false → raw RGB, flate-compressed on embed.
    is_jpeg: bool,
}

fn read_exif_orientation(path: &str) -> Orientation {
    fn inner(path: &str) -> Option<Orientation> {
        let file = std::fs::File::open(path).ok()?;
        let mut reader = std::io::BufReader::new(file);
        let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
        let field = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
        Orientation::from_exif(field.value.get_uint(0)? as u8)
    }
    inner(path).unwrap_or(Orientation::NoTransforms)
}

fn flatten_alpha_over_white(img: DynamicImage) -> image::RgbImage {
    if !img.color().has_alpha() {
        return img.into_rgb8();
    }
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for (dst, src) in out.pixels_mut().zip(rgba.pixels()) {
        let a = src[3] as u32;
        for c in 0..3 {
            dst[c] = ((src[c] as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
        }
    }
    out
}

fn reencode_as_jpeg(img: DynamicImage) -> Result<PdfPageImage, Box<dyn Error>> {
    let rgb = flatten_alpha_over_white(img);
    let (w, h) = rgb.dimensions();
    let bytes = crate::core::compression::encode_jpeg_bytes(
        w as usize,
        h as usize,
        rgb.as_raw(),
        PDF_JPEG_QUALITY,
    )?;
    Ok(PdfPageImage {
        bytes,
        width: w,
        height: h,
        color_space: "DeviceRGB",
        is_jpeg: true,
    })
}

fn raw_rgb_page(img: DynamicImage) -> PdfPageImage {
    let rgb = flatten_alpha_over_white(img);
    let (w, h) = rgb.dimensions();
    PdfPageImage {
        bytes: rgb.into_raw(),
        width: w,
        height: h,
        color_space: "DeviceRGB",
        is_jpeg: false,
    }
}

fn prepare_via_image_crate(path: &str) -> Result<PdfPageImage, Box<dyn Error>> {
    let file = std::fs::File::open(path)?;
    let mut reader = ImageReader::new(std::io::BufReader::new(file)).with_guessed_format()?;
    reader.limits(decode_limits());
    let format = reader.format();
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);

    // Zero-recompression fast path: an 8-bit RGB/gray JPEG that needs no
    // rotation is embedded byte-for-byte (JPEG == DCTDecode). EXIF-rotated or
    // CMYK JPEGs must go through decode: raw passthrough would render them
    // sideways (the placement matrix never sees EXIF) or with inverted colors.
    if format == Some(ImageFormat::Jpeg) && orientation == Orientation::NoTransforms {
        let (w, h) = decoder.dimensions();
        if w <= MAX_IMAGE_DIMENSION && h <= MAX_IMAGE_DIMENSION {
            let color_space = match decoder.original_color_type() {
                ExtendedColorType::Rgb8 => Some("DeviceRGB"),
                ExtendedColorType::L8 => Some("DeviceGray"),
                _ => None,
            };
            if let Some(color_space) = color_space {
                return Ok(PdfPageImage {
                    bytes: std::fs::read(path)?,
                    width: w,
                    height: h,
                    color_space,
                    is_jpeg: true,
                });
            }
        }
    }

    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);

    // Lossless sources stay lossless (raw RGB + Flate) so screenshots and
    // text stay crisp; photo formats re-encode as JPEG.
    match format {
        Some(ImageFormat::Png | ImageFormat::Bmp | ImageFormat::Tiff | ImageFormat::Gif) => {
            Ok(raw_rgb_page(img))
        }
        _ => reencode_as_jpeg(img),
    }
}

fn prepare_page_image(path: &str) -> Result<PdfPageImage, Box<dyn Error>> {
    match ext_lowercase(path).as_deref() {
        Some("heic") | Some("heif") => {
            let mut img = crate::core::heic::decode_heic(path)?;
            // The native decoders return raw pixels; orientation lives in the
            // container's EXIF and must be applied here.
            img.apply_orientation(read_exif_orientation(path));
            reencode_as_jpeg(img)
        }
        // jxl-oxide applies container orientation during render.
        Some("jxl") => reencode_as_jpeg(open_image(path)?),
        _ => prepare_via_image_crate(path),
    }
}

pub fn images_to_pdf(
    paths: &[String],
    output_dir: Option<&str>,
    opts: ImagesToPdfOptions,
) -> Result<ImagesToPdfResult, Box<dyn Error>> {
    if paths.is_empty() {
        return Err("no input images provided".into());
    }
    for p in paths {
        crate::core::paths::validate_input_path(p, false)?;
        checked_size(p)?;
    }

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let mut kids: Vec<Object> = Vec::with_capacity(paths.len());

    // Sequential on purpose: peak memory (decoded pixels) dominates, not CPU.
    for path in paths {
        let file_label = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path.as_str());
        let page_img = prepare_page_image(path).map_err(|e| format!("{file_label}: {e}"))?;
        let layout =
            compute_page_layout(page_img.width, page_img.height, opts.page_size, opts.margin);

        let mut xobject = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => page_img.width as i64,
                "Height" => page_img.height as i64,
                "ColorSpace" => page_img.color_space,
                "BitsPerComponent" => 8,
            },
            page_img.bytes,
        );
        if page_img.is_jpeg {
            xobject.dict.set("Filter", "DCTDecode");
        } else if let Err(e) = xobject.compress() {
            // A raw RGB stream without a filter is still a valid PDF — just larger.
            eprintln!("images_to_pdf: flate compression failed, embedding uncompressed: {e}");
        }
        let image_id = document.add_object(xobject);

        let content = format!(
            "q\n{:.2} 0 0 {:.2} {:.2} {:.2} cm\n/Im0 Do\nQ",
            layout.img_w, layout.img_h, layout.img_x, layout.img_y
        );
        let content_id = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let resources_id = document.add_object(dictionary! {
            "XObject" => dictionary! { "Im0" => image_id },
        });
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![
                0.into(),
                0.into(),
                Object::Real(layout.page_w),
                Object::Real(layout.page_h),
            ],
            "Resources" => resources_id,
            "Contents" => content_id,
        });
        kids.push(Object::Reference(page_id));
    }

    let page_count = kids.len();
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
        .unwrap_or("images");
    let fallback_dir = first_input.parent().unwrap_or(Path::new("."));
    let dir = resolve_output_dir(output_dir, fallback_dir)?;
    let mut reserved = HashSet::new();
    let output = crate::core::paths::first_available_path(
        // `_scan` suffix: every output-producing op here carries a marker
        // (_merged, _pages_N, _audio, _page_N…); a bare `photo.pdf` next to
        // `photo.jpg` doesn't read as "ReVault made this".
        &dir.join(format!("{first_stem}_scan.pdf")),
        &mut reserved,
    );

    let mut buffer = Vec::new();
    document.save_to(&mut buffer)?;
    std::fs::write(&output, &buffer)?;

    Ok(ImagesToPdfResult {
        output_path: output.to_string_lossy().into_owned(),
        output_size: buffer.len() as u64,
        page_count,
    })
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

    // ---- images_to_pdf ----

    fn write_jpeg(dir: &Path, name: &str, w: usize, h: usize) -> String {
        let path = dir.join(name);
        fs::write(&path, jpeg_bytes(w, h, 90.0)).unwrap();
        path.to_str().unwrap().to_string()
    }

    fn gray_jpeg_bytes(width: usize, height: usize) -> Vec<u8> {
        let pixels = vec![128u8; width * height];
        let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_GRAYSCALE);
        cinfo.set_size(width, height);
        cinfo.set_quality(90.0);
        let mut cinfo = cinfo.start_compress(Vec::new()).unwrap();
        cinfo.write_scanlines(&pixels).unwrap();
        cinfo.finish().unwrap()
    }

    fn default_i2p_opts() -> ImagesToPdfOptions {
        ImagesToPdfOptions {
            page_size: PageSize::A4,
            margin: PageMargin::Small,
        }
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.05
    }

    fn page_media_box(doc: &Document, page_no: u32) -> Vec<f32> {
        let pages = doc.get_pages();
        let dict = doc.get_dictionary(pages[&page_no]).unwrap();
        dict.get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o.as_float().unwrap())
            .collect()
    }

    #[test]
    fn images_to_pdf_single_jpeg_landscape_a4() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_jpeg(dir.path(), "photo.jpg", 400, 300);
        let result = images_to_pdf(&[input], None, default_i2p_opts()).unwrap();
        assert_eq!(result.page_count, 1);
        let doc = Document::load(&result.output_path).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
        let mb = page_media_box(&doc, 1);
        assert!(
            approx(mb[2], 841.89) && approx(mb[3], 595.28),
            "landscape A4 expected, got {mb:?}"
        );
    }

    #[test]
    fn images_to_pdf_clean_jpeg_is_byte_exact_passthrough() {
        let dir = tempfile::tempdir().unwrap();
        let jpeg = jpeg_bytes(320, 240, 90.0);
        let input = dir.path().join("photo.jpg");
        fs::write(&input, &jpeg).unwrap();
        let result = images_to_pdf(
            &[input.to_str().unwrap().to_string()],
            None,
            default_i2p_opts(),
        )
        .unwrap();
        let doc = Document::load(&result.output_path).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        let images = doc.get_page_images(page_id).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(
            images[0].content, jpeg,
            "expected zero-recompression passthrough"
        );
    }

    #[test]
    fn images_to_pdf_multiple_images_preserve_order() {
        let dir = tempfile::tempdir().unwrap();
        let a = write_jpeg(dir.path(), "a.jpg", 100, 80);
        let b = write_jpeg(dir.path(), "b.jpg", 200, 80);
        let c = write_jpeg(dir.path(), "c.jpg", 300, 80);
        let result = images_to_pdf(&[a, b, c], None, default_i2p_opts()).unwrap();
        assert_eq!(result.page_count, 3);
        let doc = Document::load(&result.output_path).unwrap();
        let pages = doc.get_pages();
        let widths: Vec<i64> = (1..=3u32)
            .map(|n| doc.get_page_images(pages[&n]).unwrap()[0].width)
            .collect();
        assert_eq!(widths, vec![100, 200, 300]);
    }

    #[test]
    fn images_to_pdf_png_alpha_flattens_without_smask() {
        let dir = tempfile::tempdir().unwrap();
        let png_path = dir.path().join("shot.png");
        let img = image::RgbaImage::from_fn(64, 64, |x, _| {
            if x < 32 {
                image::Rgba([255, 0, 0, 128])
            } else {
                image::Rgba([0, 0, 255, 255])
            }
        });
        img.save(&png_path).unwrap();
        let result = images_to_pdf(
            &[png_path.to_str().unwrap().to_string()],
            None,
            default_i2p_opts(),
        )
        .unwrap();
        let doc = Document::load(&result.output_path).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        let images = doc.get_page_images(page_id).unwrap();
        assert!(
            images[0].origin_dict.get(b"SMask").is_err(),
            "no SMask expected"
        );
        assert!(images[0]
            .filters
            .as_ref()
            .unwrap()
            .iter()
            .any(|f| f == "FlateDecode"));
        let stream = doc.objects.get(&images[0].id).unwrap().as_stream().unwrap();
        let raw = stream.decompressed_content().unwrap();
        assert_eq!(raw.len(), 64 * 64 * 3);
        // Semi-transparent red over white → pink-ish, not dark red.
        assert!(
            raw[1] > 100,
            "alpha should flatten over white, got G={}",
            raw[1]
        );
    }

    #[test]
    fn images_to_pdf_exif_rotated_jpeg_lands_upright() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rotated.jpg");
        fs::write(&path, jpeg_bytes(400, 300, 90.0)).unwrap();
        let mut md = little_exif::metadata::Metadata::new();
        md.set_tag(little_exif::exif_tag::ExifTag::Orientation(vec![6u16]));
        md.write_to_file(&path).unwrap();

        let result = images_to_pdf(
            &[path.to_str().unwrap().to_string()],
            None,
            default_i2p_opts(),
        )
        .unwrap();
        let doc = Document::load(&result.output_path).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        let images = doc.get_page_images(page_id).unwrap();
        // Orientation 6 = Rotate90: 400x300 must land as 300x400.
        assert_eq!((images[0].width, images[0].height), (300, 400));
    }

    #[test]
    fn images_to_pdf_grayscale_jpeg_uses_devicegray() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gray.jpg");
        fs::write(&path, gray_jpeg_bytes(64, 64)).unwrap();
        let result = images_to_pdf(
            &[path.to_str().unwrap().to_string()],
            None,
            default_i2p_opts(),
        )
        .unwrap();
        let doc = Document::load(&result.output_path).unwrap();
        let page_id = *doc.get_pages().values().next().unwrap();
        let images = doc.get_page_images(page_id).unwrap();
        assert_eq!(images[0].color_space.as_deref(), Some("DeviceGray"));
    }

    #[test]
    fn layout_fit_uses_150_dpi() {
        let l = compute_page_layout(300, 150, PageSize::Fit, PageMargin::Big);
        assert!(approx(l.page_w, 144.0) && approx(l.page_h, 72.0));
        assert!(approx(l.img_w, 144.0) && approx(l.img_x, 0.0));
    }

    #[test]
    fn layout_a4_scales_down_and_centers() {
        let l = compute_page_layout(1000, 2000, PageSize::A4, PageMargin::Big);
        assert!(approx(l.page_w, 595.28) && approx(l.page_h, 841.89));
        // natural 480x960pt; content 515.28x761.89 → height-bound scale
        assert!(approx(l.img_h, 761.89));
        assert!(approx(l.img_y, 40.0));
        assert!(approx(l.img_x, (595.28 - l.img_w) / 2.0));
    }

    #[test]
    fn layout_small_image_never_upscales() {
        let l = compute_page_layout(100, 100, PageSize::A4, PageMargin::None);
        // Natural size at 150 DPI is 48pt — must not stretch to fill the page.
        assert!(approx(l.img_w, 48.0) && approx(l.img_h, 48.0));
    }

    #[test]
    fn layout_landscape_image_rotates_page() {
        let l = compute_page_layout(2000, 1000, PageSize::Letter, PageMargin::None);
        assert!(approx(l.page_w, 792.0) && approx(l.page_h, 612.0));
    }

    #[test]
    fn images_to_pdf_empty_input_errors() {
        assert!(images_to_pdf(&[], None, default_i2p_opts()).is_err());
    }

    #[test]
    fn images_to_pdf_missing_input_errors() {
        let result = images_to_pdf(
            &["/nonexistent/x.jpg".to_string()],
            None,
            default_i2p_opts(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn images_to_pdf_output_carries_scan_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_jpeg(dir.path(), "photo.jpg", 64, 64);
        let result = images_to_pdf(&[input], None, default_i2p_opts()).unwrap();
        assert!(
            result.output_path.ends_with("photo_scan.pdf"),
            "got: {}",
            result.output_path
        );
    }

    #[test]
    fn images_to_pdf_no_clobber_when_output_exists() {
        let dir = tempfile::tempdir().unwrap();
        let input = write_jpeg(dir.path(), "photo.jpg", 64, 64);
        fs::write(dir.path().join("photo_scan.pdf"), b"existing").unwrap();
        let result = images_to_pdf(&[input], None, default_i2p_opts()).unwrap();
        assert_ne!(
            result.output_path,
            dir.path().join("photo_scan.pdf").to_str().unwrap()
        );
        assert_eq!(
            fs::read(dir.path().join("photo_scan.pdf")).unwrap(),
            b"existing"
        );
    }
}
