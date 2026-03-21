use crate::core::image_io::{checked_size, ext_lowercase, write_preserving_timestamps};
use exif::{In, Reader, Tag, Value};
use img_parts::ImageEXIF;
use little_exif::metadata::Metadata;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::io::BufReader;
use std::path::Path;

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

fn build_output_path(input: &str, output_dir: Option<&str>) -> String {
    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let dir = output_dir
        .map(Path::new)
        .unwrap_or_else(|| p.parent().unwrap_or(Path::new(".")));
    dir.join(format!("{stem}_stripped.{ext}"))
        .to_string_lossy()
        .into_owned()
}

pub fn strip_batch(paths: &[String], output_dir: Option<&str>) -> Vec<StripResult> {
    let mut results = Vec::with_capacity(paths.len());
    for input in paths {
        let output = build_output_path(input, output_dir);
        match strip_metadata(input, &output) {
            Ok(r) => results.push(r),
            Err(e) => results.push(StripResult::err(input, e.to_string())),
        }
    }
    results
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

/// Author tag hex IDs: Artist in GENERIC, Copyright in GENERIC; OwnerName in EXIF
const AUTHOR_GENERIC_TAG_IDS: &[u16] = &[0x013b, 0x8298];
const AUTHOR_EXIF_TAG_IDS: &[u16] = &[0xa430];

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

    let original_size = checked_size(input)?;
    let mtime = filetime::FileTime::from_last_modification_time(&fs::metadata(input)?);

    fs::copy(input, output)?;

    let output_path = Path::new(output);
    // Fall back to full strip (img-parts) if selective parsing fails — safer to over-strip
    let mut metadata = match Metadata::new_from_path(output_path) {
        Ok(m) => m,
        Err(_) => {
            return strip_metadata(input, output);
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

    metadata
        .write_to_file(output_path)
        .map_err(|e| format!("failed to write metadata: {e}"))?;

    filetime::set_file_mtime(output, mtime)?;
    let stripped_size = fs::metadata(output)?.len();

    Ok(StripResult::ok(input, output, original_size, stripped_size))
}

pub fn strip_selective_batch(
    paths: &[String],
    opts: StripOptions,
    output_dir: Option<&str>,
) -> Vec<StripResult> {
    paths
        .iter()
        .map(|input| {
            let output = build_output_path(input, output_dir);
            match strip_metadata_selective(input, &output, opts) {
                Ok(r) => r,
                Err(e) => StripResult::err(input, e.to_string()),
            }
        })
        .collect()
}

/// Strip only GPS metadata from a file in-place (used by compression flow).
pub fn strip_gps_in_place(path: &str) -> Result<(), Box<dyn Error>> {
    let file_path = Path::new(path);
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
        fs::write(&path, &[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
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
    fn strip_batch_mixed_results() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("valid.jpg");
        fs::write(&valid, &[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
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
        fs::write(&input, &[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
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
        fs::write(&input, &[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
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
        fs::write(&valid, &[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
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
        fs::write(&path, &[0xFF, 0xD8, 0xFF, 0xD9]).unwrap();
        // Should succeed even on a minimal JPEG with no metadata
        let result = strip_gps_in_place(path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn build_output_path_same_dir() {
        let out = build_output_path("/photos/vacation.jpg", None);
        assert_eq!(Path::new(&out), Path::new("/photos/vacation_stripped.jpg"));
    }

    #[test]
    fn build_output_path_custom_dir() {
        let out = build_output_path("/photos/vacation.jpg", Some("/output"));
        assert_eq!(Path::new(&out), Path::new("/output/vacation_stripped.jpg"));
    }
}
