use crate::core::image_io::{checked_size, ext_lowercase, write_preserving_timestamps};
use exif::{In, Reader, Tag, Value};
use img_parts::ImageEXIF;
use little_exif::metadata::Metadata;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct GpsInfo {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetadataResult {
    pub path: String,
    pub gps: Option<GpsInfo>,
    pub device: Option<String>,
    pub datetime: Option<String>,
    pub author: Option<String>,
    pub technical: Option<String>,
    pub has_metadata: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StripResult {
    pub input_path: String,
    pub output_path: String,
    pub original_size: u64,
    pub stripped_size: u64,
    pub error: Option<String>,
}

impl StripResult {
    fn ok(input: &str, output: &str, original_size: u64, stripped_size: u64) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: output.to_string(),
            original_size,
            stripped_size,
            error: None,
        }
    }

    fn err(input: &str, msg: String) -> Self {
        Self {
            input_path: input.to_string(),
            output_path: String::new(),
            original_size: 0,
            stripped_size: 0,
            error: Some(msg),
        }
    }
}

fn dms_to_decimal(dms: &[exif::Rational], ref_tag: Option<&str>) -> Option<f64> {
    if dms.len() < 3 {
        return None;
    }
    let deg = dms[0].to_f64();
    let min = dms[1].to_f64();
    let sec = dms[2].to_f64();
    let decimal = deg + min / 60.0 + sec / 3600.0;
    match ref_tag {
        Some("S") | Some("W") => Some(-decimal),
        _ => Some(decimal),
    }
}

fn field_as_string(exif: &exif::Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    let s = field.display_value().to_string();
    let s = s.trim().trim_matches('"').to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn extract_gps(exif: &exif::Exif) -> Option<GpsInfo> {
    let lat_field = exif.get_field(Tag::GPSLatitude, In::PRIMARY)?;
    let lon_field = exif.get_field(Tag::GPSLongitude, In::PRIMARY)?;

    let lat_ref = field_as_string(exif, Tag::GPSLatitudeRef);
    let lon_ref = field_as_string(exif, Tag::GPSLongitudeRef);

    let lat = match &lat_field.value {
        Value::Rational(v) => dms_to_decimal(v, lat_ref.as_deref()),
        _ => None,
    }?;
    let lon = match &lon_field.value {
        Value::Rational(v) => dms_to_decimal(v, lon_ref.as_deref()),
        _ => None,
    }?;

    let altitude = exif
        .get_field(Tag::GPSAltitude, In::PRIMARY)
        .and_then(|f| match &f.value {
            Value::Rational(v) if !v.is_empty() => {
                let alt = v[0].to_f64();
                let below = exif
                    .get_field(Tag::GPSAltitudeRef, In::PRIMARY)
                    .and_then(|r| r.value.get_uint(0))
                    .unwrap_or(0);
                Some(if below == 1 { -alt } else { alt })
            }
            _ => None,
        });

    Some(GpsInfo {
        latitude: lat,
        longitude: lon,
        altitude,
    })
}

fn extract_device(exif: &exif::Exif) -> Option<String> {
    let make = field_as_string(exif, Tag::Make);
    let model = field_as_string(exif, Tag::Model);
    match (make, model) {
        (Some(m), Some(d)) => {
            if d.starts_with(&m) {
                Some(d)
            } else {
                Some(format!("{m} {d}"))
            }
        }
        (Some(m), None) => Some(m),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    }
}

fn extract_technical(exif: &exif::Exif) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(f) = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
        if let Some(iso) = f.value.get_uint(0) {
            parts.push(format!("ISO {iso}"));
        }
    }
    if let Some(f) = exif.get_field(Tag::FNumber, In::PRIMARY) {
        if let Value::Rational(ref v) = f.value {
            if !v.is_empty() {
                parts.push(format!("f/{:.1}", v[0].to_f64()));
            }
        }
    }
    if let Some(f) = exif.get_field(Tag::ExposureTime, In::PRIMARY) {
        if let Value::Rational(ref v) = f.value {
            if !v.is_empty() {
                let r = &v[0];
                if r.num < r.denom {
                    parts.push(format!("{}/{}s", r.num, r.denom));
                } else {
                    parts.push(format!("{}s", r.to_f64()));
                }
            }
        }
    }
    if let Some(f) = exif.get_field(Tag::FocalLength, In::PRIMARY) {
        if let Value::Rational(ref v) = f.value {
            if !v.is_empty() {
                parts.push(format!("{}mm", v[0].to_f64()));
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" \u{b7} "))
    }
}

pub fn read_metadata(path: &str) -> Result<MetadataResult, Box<dyn Error>> {
    crate::core::paths::validate_input_path(path, false)
        .map_err(|e| -> Box<dyn Error> { e.into() })?;
    checked_size(path)?;

    let file = fs::File::open(path)?;
    let exif = match Reader::new().read_from_container(&mut BufReader::new(&file)) {
        Ok(e) => e,
        Err(_) => {
            return Ok(MetadataResult {
                path: path.to_string(),
                gps: None,
                device: None,
                datetime: None,
                author: None,
                technical: None,
                has_metadata: false,
            });
        }
    };

    let gps = extract_gps(&exif);
    let device = extract_device(&exif);
    let datetime = field_as_string(&exif, Tag::DateTimeOriginal);
    let author =
        field_as_string(&exif, Tag::Artist).or_else(|| field_as_string(&exif, Tag::Copyright));
    let technical = extract_technical(&exif);
    let has_metadata = gps.is_some()
        || device.is_some()
        || datetime.is_some()
        || author.is_some()
        || technical.is_some();

    Ok(MetadataResult {
        path: path.to_string(),
        gps,
        device,
        datetime,
        author,
        technical,
        has_metadata,
    })
}

pub fn strip_metadata(input: &str, output: &str) -> Result<StripResult, Box<dyn Error>> {
    crate::core::paths::validate_input_path(input, false)?;
    let original_size = checked_size(input)?;
    let data = fs::read(input)?;
    let bytes = img_parts::Bytes::from(data);

    let stripped = match ext_lowercase(input).as_deref() {
        Some("jpg" | "jpeg") => {
            let mut jpeg = img_parts::jpeg::Jpeg::from_bytes(bytes)?;
            jpeg.set_exif(None);
            jpeg.encoder().bytes().to_vec()
        }
        Some("png") => {
            let mut png = img_parts::png::Png::from_bytes(bytes)?;
            png.set_exif(None);
            png.encoder().bytes().to_vec()
        }
        Some("webp") => {
            let mut webp = img_parts::webp::WebP::from_bytes(bytes)?;
            webp.set_exif(None);
            webp.encoder().bytes().to_vec()
        }
        Some("heic" | "heif") => {
            let mtime = filetime::FileTime::from_last_modification_time(&fs::metadata(input)?);
            fs::copy(input, output)?;
            Metadata::file_clear_metadata(Path::new(output))?;
            filetime::set_file_mtime(output, mtime)?;
            let stripped_size = fs::metadata(output)?.len();
            return Ok(StripResult::ok(input, output, original_size, stripped_size));
        }
        Some(ext) => return Err(format!("metadata stripping not supported for .{ext}").into()),
        None => return Err("file has no extension".into()),
    };

    write_preserving_timestamps(input, output, &stripped)?;
    let stripped_size = fs::metadata(output)?.len();

    Ok(StripResult::ok(input, output, original_size, stripped_size))
}

fn supported_strip_extension(input: &str) -> Result<(), Box<dyn Error>> {
    match ext_lowercase(input).as_deref() {
        Some("jpg" | "jpeg" | "png" | "webp" | "heic" | "heif") => Ok(()),
        Some(ext) => Err(format!("metadata stripping not supported for .{ext}").into()),
        None => Err("file has no extension".into()),
    }
}

fn build_output_path(
    input: &str,
    output_dir: Option<&str>,
    reserved: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let dir = match output_dir {
        Some(d) => crate::core::paths::validate_output_dir(d)?,
        None => p.parent().unwrap_or(Path::new(".")).to_path_buf(),
    };
    let output = crate::core::paths::first_available_path(
        &dir.join(format!("{stem}_stripped.{ext}")),
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

pub fn strip_batch(paths: &[String], output_dir: Option<&str>) -> Vec<StripResult> {
    let outputs = build_output_paths(paths, output_dir);
    paths
        .par_iter()
        .zip(outputs.into_par_iter())
        .map(|(input, output)| match output {
            Ok(output) => match strip_metadata(input, &output) {
                Ok(r) => r,
                Err(e) => StripResult::err(input, e.to_string()),
            },
            Err(e) => StripResult::err(input, e),
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct StripOptions {
    pub gps: bool,
    pub device: bool,
    pub datetime: bool,
    pub author: bool,
}

impl StripOptions {
    fn any_selected(&self) -> bool {
        self.gps || self.device || self.datetime || self.author
    }
}

/// GPS tag hex IDs (all in GPS IFD group)
const GPS_TAG_IDS: &[u16] = &[
    0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008, 0x0009, 0x000a, 0x000b,
    0x000c, 0x000d, 0x000e, 0x000f, 0x0010, 0x0011, 0x0012, 0x0013, 0x0014, 0x0015, 0x0016, 0x0017,
    0x0018, 0x0019, 0x001a, 0x001b, 0x001c, 0x001d, 0x001e, 0x001f,
];

/// Device tag hex IDs: Make, Model in GENERIC; LensMake, LensModel, SerialNumber, LensSerialNumber in EXIF
const DEVICE_GENERIC_TAG_IDS: &[u16] = &[0x010f, 0x0110];
const DEVICE_EXIF_TAG_IDS: &[u16] = &[0xa430, 0xa431, 0xa433, 0xa434, 0xa435];

/// DateTime tag hex IDs: ModifyDate in GENERIC; DateTimeOriginal, CreateDate, OffsetTime* SubSec* in EXIF
const DATETIME_GENERIC_TAG_IDS: &[u16] = &[0x0132];
const DATETIME_EXIF_TAG_IDS: &[u16] = &[
    0x9003, 0x9004, 0x9010, 0x9011, 0x9012, 0x9290, 0x9291, 0x9292,
];

/// Author tag hex IDs: Artist in GENERIC, Copyright in GENERIC
const AUTHOR_GENERIC_TAG_IDS: &[u16] = &[0x013b, 0x8298];
const AUTHOR_EXIF_TAG_IDS: &[u16] = &[];

fn remove_tags_by_group(
    metadata: &mut little_exif::metadata::Metadata,
    tag_ids: &[u16],
    group: little_exif::ifd::ExifTagGroup,
) {
    for &tag_id in tag_ids {
        metadata.remove_tag_by_hex_group(tag_id, group);
    }
}

pub fn strip_metadata_selective(
    input: &str,
    output: &str,
    opts: StripOptions,
) -> Result<StripResult, Box<dyn Error>> {
    if !opts.any_selected() {
        return Err("no metadata categories selected for stripping".into());
    }

    crate::core::paths::validate_input_path(input, false)?;
    supported_strip_extension(input)?;
    let original_size = checked_size(input)?;
    let mtime = filetime::FileTime::from_last_modification_time(&fs::metadata(input)?);

    fs::copy(input, output)?;

    let output_path = Path::new(output);
    // Fall back to full strip (img-parts) if selective parsing fails — safer to over-strip
    let mut metadata = match Metadata::new_from_path(output_path) {
        Ok(m) => m,
        Err(_) => {
            let result = strip_metadata(input, output);
            if result.is_err() {
                let _ = fs::remove_file(output);
            }
            return result;
        }
    };

    use little_exif::ifd::ExifTagGroup;

    if opts.gps {
        remove_tags_by_group(&mut metadata, GPS_TAG_IDS, ExifTagGroup::GPS);
        metadata.remove_tag_by_hex_group(0x8825, ExifTagGroup::GENERIC);
    }

    if opts.device {
        remove_tags_by_group(&mut metadata, DEVICE_GENERIC_TAG_IDS, ExifTagGroup::GENERIC);
        remove_tags_by_group(&mut metadata, DEVICE_EXIF_TAG_IDS, ExifTagGroup::EXIF);
    }

    if opts.datetime {
        remove_tags_by_group(
            &mut metadata,
            DATETIME_GENERIC_TAG_IDS,
            ExifTagGroup::GENERIC,
        );
        remove_tags_by_group(&mut metadata, DATETIME_EXIF_TAG_IDS, ExifTagGroup::EXIF);
    }

    if opts.author {
        remove_tags_by_group(&mut metadata, AUTHOR_GENERIC_TAG_IDS, ExifTagGroup::GENERIC);
        remove_tags_by_group(&mut metadata, AUTHOR_EXIF_TAG_IDS, ExifTagGroup::EXIF);
    }

    if let Err(e) = metadata.write_to_file(output_path) {
        let _ = fs::remove_file(output);
        return Err(format!("failed to write metadata: {e}").into());
    }

    filetime::set_file_mtime(output, mtime)?;
    let stripped_size = fs::metadata(output)?.len();

    Ok(StripResult::ok(input, output, original_size, stripped_size))
}

pub fn strip_selective_batch(
    paths: &[String],
    opts: StripOptions,
    output_dir: Option<&str>,
) -> Vec<StripResult> {
    let outputs = build_output_paths(paths, output_dir);
    paths
        .par_iter()
        .zip(outputs.into_par_iter())
        .map(|(input, output)| match output {
            Ok(output) => match strip_metadata_selective(input, &output, opts) {
                Ok(r) => r,
                Err(e) => StripResult::err(input, e.to_string()),
            },
            Err(e) => StripResult::err(input, e),
        })
        .collect()
}

/// Strip only GPS metadata from a file in-place (used by compression flow).
pub fn strip_gps_in_place(path: &str) -> Result<(), Box<dyn Error>> {
    crate::core::paths::validate_input_path(path, false)
        .map_err(|e| -> Box<dyn Error> { e.into() })?;
    let file_path = Path::new(path);

    let mtime = filetime::FileTime::from_last_modification_time(&fs::metadata(file_path)?);

    let mut metadata = match Metadata::new_from_path(file_path) {
        // File has no parseable metadata — nothing to strip
        Err(_) => return Ok(()),
        Ok(m) => m,
    };

    use little_exif::ifd::ExifTagGroup;
    remove_tags_by_group(&mut metadata, GPS_TAG_IDS, ExifTagGroup::GPS);
    metadata.remove_tag_by_hex_group(0x8825, ExifTagGroup::GENERIC);

    metadata
        .write_to_file(file_path)
        .map_err(|e| format!("failed to write metadata: {e}"))?;

    filetime::set_file_mtime(path, mtime)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dms_to_decimal_north() {
        let dms = [
            exif::Rational { num: 40, denom: 1 },
            exif::Rational { num: 26, denom: 1 },
            exif::Rational {
                num: 46,
                denom: 100,
            },
        ];
        let val = dms_to_decimal(&dms, Some("N")).unwrap();
        assert!((val - 40.43346).abs() < 0.001);
    }

    #[test]
    fn dms_to_decimal_south() {
        let dms = [
            exif::Rational { num: 33, denom: 1 },
            exif::Rational { num: 51, denom: 1 },
            exif::Rational { num: 54, denom: 1 },
        ];
        let val = dms_to_decimal(&dms, Some("S")).unwrap();
        assert!((val - -33.865).abs() < 0.001);
    }

    #[test]
    fn dms_to_decimal_too_short() {
        let dms = [exif::Rational { num: 40, denom: 1 }];
        assert!(dms_to_decimal(&dms, Some("N")).is_none());
    }

    #[test]
    fn read_metadata_no_exif() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jpg");
        fs::write(&path, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
        let result = read_metadata(path.to_str().unwrap()).unwrap();
        assert!(!result.has_metadata);
        assert!(result.gps.is_none());
    }

    #[test]
    fn strip_metadata_unsupported_format() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.avif");
        fs::write(&input, b"fake avif data").unwrap();
        let output = dir.path().join("test_stripped.avif");
        let result = strip_metadata(input.to_str().unwrap(), output.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }

    #[test]
    fn strip_selective_unsupported_format_leaves_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.avif");
        fs::write(&input, b"fake avif data").unwrap();
        let output = dir.path().join("test_stripped.avif");
        let opts = StripOptions {
            gps: true,
            device: false,
            datetime: false,
            author: false,
        };

        let result =
            strip_metadata_selective(input.to_str().unwrap(), output.to_str().unwrap(), opts);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
        assert!(!output.exists());
    }

    #[test]
    fn strip_batch_mixed_results() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("valid.jpg");
        fs::write(&valid, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
        let missing = dir.path().join("missing.jpg");

        let paths = vec![
            valid.to_str().unwrap().to_string(),
            missing.to_str().unwrap().to_string(),
        ];
        let results = strip_batch(&paths, None);
        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_some());
    }

    #[test]
    fn strip_selective_no_options_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");
        fs::write(&input, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
        let output = dir.path().join("test_stripped.jpg");
        let opts = StripOptions {
            gps: false,
            device: false,
            datetime: false,
            author: false,
        };
        let result =
            strip_metadata_selective(input.to_str().unwrap(), output.to_str().unwrap(), opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no metadata"));
    }

    #[test]
    fn strip_selective_gps_produces_output() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("test.jpg");
        fs::write(&input, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
        let output = dir.path().join("test_stripped.jpg");
        let opts = StripOptions {
            gps: true,
            device: false,
            datetime: false,
            author: false,
        };
        let result =
            strip_metadata_selective(input.to_str().unwrap(), output.to_str().unwrap(), opts)
                .unwrap();
        assert!(result.error.is_none());
        assert!(output.exists());
    }

    #[test]
    fn strip_selective_batch_mixed_results() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("valid.jpg");
        fs::write(&valid, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
        let missing = dir.path().join("missing.jpg");

        let paths = vec![
            valid.to_str().unwrap().to_string(),
            missing.to_str().unwrap().to_string(),
        ];
        let opts = StripOptions {
            gps: true,
            device: true,
            datetime: false,
            author: false,
        };
        let results = strip_selective_batch(&paths, opts, None);
        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_some());
    }

    #[test]
    fn strip_gps_in_place_minimal_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jpg");
        fs::write(&path, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
        // Should succeed even on a minimal JPEG with no metadata
        let result = strip_gps_in_place(path.to_str().unwrap());
        assert!(result.is_ok());
    }

    /// Builds a synthetic JPEG whose Exif APP1 segment has GPS in *two*
    /// places: IFD0 (the "normal" spot) and IFD1 (the thumbnail IFD), the
    /// latter linked via its own GPSInfo (0x8825) pointer to a private GPS
    /// sub-IFD. This mirrors the real-world leak where a naive "strip EXIF"
    /// only clears IFD0 and leaves the embedded thumbnail's own GPS-tagged
    /// metadata intact. See TIFF 6.0 + Exif 2.3 IFD-linking spec.
    fn jpeg_with_gps_in_thumbnail_ifd() -> Vec<u8> {
        let mut tiff = Vec::new();
        tiff.extend([0x49, 0x49, 0x2A, 0x00]); // "II" + TIFF magic number (LE)
        tiff.extend(8u32.to_le_bytes()); // offset to IFD0

        // IFD0 @8: no tags of its own, links straight to IFD1 @14
        tiff.extend(0u16.to_le_bytes());
        tiff.extend(14u32.to_le_bytes());

        // IFD1 @14: GPSInfo pointer (-> GPS sub-IFD @56) + thumbnail data ptr
        tiff.extend(3u16.to_le_bytes());
        tiff.extend(0x8825u16.to_le_bytes());
        tiff.extend(0x0004u16.to_le_bytes()); // INT32U
        tiff.extend(1u32.to_le_bytes());
        tiff.extend(56u32.to_le_bytes()); // -> GPS sub-IFD offset
        tiff.extend(0x0201u16.to_le_bytes()); // ThumbnailOffset
        tiff.extend(0x0004u16.to_le_bytes());
        tiff.extend(1u32.to_le_bytes());
        tiff.extend(74u32.to_le_bytes()); // -> thumbnail JPEG bytes offset
        tiff.extend(0x0202u16.to_le_bytes()); // ThumbnailLength
        tiff.extend(0x0004u16.to_le_bytes());
        tiff.extend(1u32.to_le_bytes());
        tiff.extend(4u32.to_le_bytes()); // thumbnail length
        tiff.extend(0u32.to_le_bytes()); // no more generic IFDs

        // GPS sub-IFD @56, owned by IFD1: GPSLatitudeRef = "N"
        tiff.extend(1u16.to_le_bytes());
        tiff.extend(0x0001u16.to_le_bytes());
        tiff.extend(0x0002u16.to_le_bytes()); // STRING
        tiff.extend(2u32.to_le_bytes()); // "N\0"
        tiff.extend([b'N', 0x00, 0x00, 0x00]);
        tiff.extend(0u32.to_le_bytes());

        // Thumbnail data @74: minimal valid JPEG (SOI+EOI)
        tiff.extend([0xFF, 0xD8, 0xFF, 0xD9]);
        assert_eq!(tiff.len(), 78);

        let mut app1 = Vec::new();
        app1.extend(*b"Exif\0\0");
        app1.extend(&tiff);

        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpeg.extend(((app1.len() + 2) as u16).to_be_bytes());
        jpeg.extend(&app1);
        jpeg.extend([0xFF, 0xD9]);
        jpeg
    }

    #[test]
    fn strip_metadata_full_strip_removes_thumbnail_ifd_entirely() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("gps_thumb.jpg");
        fs::write(&input, jpeg_with_gps_in_thumbnail_ifd()).unwrap();
        let output = dir.path().join("gps_thumb_stripped.jpg");

        strip_metadata(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();

        // The full-strip path (img-parts) drops the entire APP1/Exif segment
        // as one opaque blob, so there is no Exif container left to parse at
        // all — IFD0, IFD1/thumbnail wrapper and its GPS sub-IFD are all gone.
        let after = fs::read(&output).unwrap();
        let result = Reader::new().read_from_container(&mut std::io::Cursor::new(after.as_slice()));
        assert!(result.is_err());
    }

    #[test]
    fn strip_gps_in_place_clears_gps_from_thumbnail_ifd_too() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gps_thumb.jpg");
        fs::write(&path, jpeg_with_gps_in_thumbnail_ifd()).unwrap();

        // Sanity check: the thumbnail IFD really does carry its own GPS tag
        // before stripping (otherwise this test would prove nothing).
        let before = fs::read(&path).unwrap();
        let exif_before = Reader::new()
            .read_from_container(&mut std::io::Cursor::new(before.as_slice()))
            .unwrap();
        assert!(exif_before
            .get_field(Tag::GPSLatitudeRef, In::THUMBNAIL)
            .is_some());

        strip_gps_in_place(path.to_str().unwrap()).unwrap();

        let after = fs::read(&path).unwrap();
        let exif_after = Reader::new()
            .read_from_container(&mut std::io::Cursor::new(after.as_slice()))
            .unwrap();
        assert!(exif_after
            .get_field(Tag::GPSLatitudeRef, In::THUMBNAIL)
            .is_none());
        assert!(exif_after
            .get_field(Tag::GPSLatitudeRef, In::PRIMARY)
            .is_none());
    }

    #[test]
    fn strip_metadata_selective_gps_clears_thumbnail_ifd_too() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("gps_thumb.jpg");
        fs::write(&input, jpeg_with_gps_in_thumbnail_ifd()).unwrap();
        let output = dir.path().join("gps_thumb_stripped.jpg");
        let opts = StripOptions {
            gps: true,
            device: false,
            datetime: false,
            author: false,
        };

        strip_metadata_selective(input.to_str().unwrap(), output.to_str().unwrap(), opts).unwrap();

        let after = fs::read(&output).unwrap();
        let exif_after = Reader::new()
            .read_from_container(&mut std::io::Cursor::new(after.as_slice()))
            .unwrap();
        assert!(exif_after
            .get_field(Tag::GPSLatitudeRef, In::THUMBNAIL)
            .is_none());
    }

    #[test]
    fn build_output_path_same_dir() {
        let mut reserved = HashSet::new();
        let out = build_output_path("/photos/vacation.jpg", None, &mut reserved).unwrap();
        assert_eq!(Path::new(&out), Path::new("/photos/vacation_stripped.jpg"));
    }

    #[test]
    fn build_output_path_custom_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut reserved = HashSet::new();
        let out = build_output_path(
            "/photos/vacation.jpg",
            Some(dir.path().to_str().unwrap()),
            &mut reserved,
        )
        .unwrap();
        let expected = std::fs::canonicalize(dir.path())
            .unwrap()
            .join("vacation_stripped.jpg");
        assert_eq!(Path::new(&out), expected);
    }

    #[test]
    fn build_output_paths_avoids_existing_and_batch_collisions() {
        let dir = tempfile::tempdir().unwrap();
        let input_a = dir.path().join("a").join("photo.jpg");
        let input_b = dir.path().join("b").join("photo.jpg");
        std::fs::create_dir_all(input_a.parent().unwrap()).unwrap();
        std::fs::create_dir_all(input_b.parent().unwrap()).unwrap();
        fs::write(&input_a, b"a").unwrap();
        fs::write(&input_b, b"b").unwrap();
        fs::write(dir.path().join("photo_stripped.jpg"), b"existing").unwrap();

        let paths = vec![
            input_a.to_str().unwrap().to_string(),
            input_b.to_str().unwrap().to_string(),
        ];
        let outputs = build_output_paths(&paths, Some(dir.path().to_str().unwrap()));
        let first = outputs[0].as_ref().unwrap();
        let second = outputs[1].as_ref().unwrap();

        assert!(first.ends_with("photo_stripped_2.jpg"));
        assert!(second.ends_with("photo_stripped_3.jpg"));
        assert_ne!(first, second);
    }
}
