use crate::core::date::civil_from_secs;
use exif::{In, Reader};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizeMode {
    pub copy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizeResult {
    pub moved: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}

impl OrganizeResult {
    pub fn new() -> Self {
        Self {
            moved: 0,
            skipped: 0,
            errors: Vec::new(),
        }
    }
}

impl Default for OrganizeResult {
    fn default() -> Self {
        Self::new()
    }
}

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "tif", "bmp", "gif", "avif", "jxl",
];

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn read_exif_date(path: &str) -> Option<(i32, u8, u8)> {
    let file = fs::File::open(path).ok()?;
    let exif = Reader::new()
        .read_from_container(&mut BufReader::new(&file))
        .ok()?;
    let field = exif.get_field(exif::Tag::DateTimeOriginal, In::PRIMARY)?;
    let s = field.display_value().to_string();
    let s = s.trim().trim_matches('"');
    if s.is_empty() {
        return None;
    }
    // Format: "YYYY:MM:DD HH:MM:SS"
    let parts: Vec<&str> = s.split([' ', ':']).collect();
    if parts.len() >= 3 {
        let year: i32 = parts[0].parse().ok()?;
        let month: u8 = parts[1].parse().ok()?;
        let day: u8 = parts[2].parse().ok()?;
        Some((year, month, day))
    } else {
        None
    }
}

fn get_date_from_path(path: &Path) -> Option<(i32, u8, u8)> {
    let path_str = path.to_string_lossy();
    read_exif_date(&path_str).or_else(|| {
        let metadata = fs::metadata(path).ok()?;
        let mtime = metadata.modified().ok()?;
        let dur = mtime.duration_since(std::time::UNIX_EPOCH).ok()?;
        let secs = dur.as_secs();
        let (year, month, day) = civil_from_secs(secs);
        Some((year, month, day))
    })
}

fn collect_images(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_file() && is_image(&entry.path()) {
            out.push(entry.path());
        } else if ft.is_dir() && recursive {
            collect_images(&entry.path(), true, out)?;
        }
    }
    Ok(())
}

pub fn organize_by_date(source_dir: &str, dest_dir: &str, mode: &OrganizeMode) -> OrganizeResult {
    let source = match Path::new(source_dir).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return OrganizeResult {
                moved: 0,
                skipped: 0,
                errors: vec![e.to_string()],
            }
        }
    };
    let dest = Path::new(dest_dir);

    let mut images = Vec::new();
    if let Err(e) = collect_images(&source, true, &mut images) {
        return OrganizeResult {
            moved: 0,
            skipped: 0,
            errors: vec![e.to_string()],
        };
    }

    let mut result = OrganizeResult::new();

    for img_path in images {
        let date = match get_date_from_path(&img_path) {
            Some(d) => d,
            None => {
                result.skipped += 1;
                result
                    .errors
                    .push(format!("no date for {}", img_path.display()));
                continue;
            }
        };

        let (year, month, _) = date;
        let year_dir = dest.join(year.to_string());
        let month_dir = year_dir.join(format!("{:02}", month));

        if let Err(e) = fs::create_dir_all(&month_dir) {
            result.errors.push(format!(
                "could not create dir {}: {}",
                month_dir.display(),
                e
            ));
            result.skipped += 1;
            continue;
        }

        let filename = img_path.file_name().unwrap_or_default();
        let mut dest_path = month_dir.join(filename);

        // Handle filename conflicts
        let mut counter = 1;
        while dest_path.exists() {
            let stem = img_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = img_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            dest_path = month_dir.join(format!("{}_{}.{}", stem, counter, ext));
            counter += 1;
            if counter > 999 {
                result
                    .errors
                    .push(format!("too many conflicts for {}", img_path.display()));
                result.skipped += 1;
                break;
            }
        }

        if counter > 999 {
            continue;
        }

        let img_path_str = img_path.to_string_lossy().into_owned();

        let move_ok = if mode.copy {
            fs::copy(&img_path, &dest_path).is_ok()
        } else {
            match fs::rename(&img_path, &dest_path) {
                Ok(_) => true,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::CrossesDevices {
                        // Cross-device rename fails silently; fall back to copy + delete
                        fs::copy(&img_path, &dest_path).is_ok()
                            && fs::remove_file(&img_path).is_ok()
                    } else {
                        false
                    }
                }
            }
        };

        if move_ok {
            result.moved += 1;
        } else {
            result
                .errors
                .push(format!("failed to move {}", img_path_str));
            result.skipped += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn create_test_jpeg(path: &Path) {
        let pixels = vec![0u8; 100 * 100 * 3];
        let mut cinfo = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        cinfo.set_size(100, 100);
        cinfo.set_quality(80.0);
        let mut cinfo = cinfo.start_compress(Vec::new()).unwrap();
        cinfo.write_scanlines(&pixels).unwrap();
        let data = cinfo.finish().unwrap();
        File::create(path).unwrap().write_all(&data).unwrap();
    }

    #[test]
    fn organize_copies_to_year_month_folder() {
        let source = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();

        let img = source.path().join("photo.jpg");
        create_test_jpeg(&img);

        let mode = OrganizeMode { copy: true };
        let result = organize_by_date(
            source.path().to_str().unwrap(),
            dest.path().to_str().unwrap(),
            &mode,
        );

        assert_eq!(result.moved, 1);
        assert_eq!(result.skipped, 0);
        assert!(result.errors.is_empty());

        // Should be in a YYYY/MM subfolder
        let entries: Vec<_> = fs::read_dir(dest.path()).unwrap().collect();
        assert_eq!(entries.len(), 1); // year folder
        let year_path = entries[0].as_ref().unwrap().path();
        let year_name = year_path.file_name().unwrap().to_str().unwrap();
        assert_eq!(year_name.len(), 4); // YYYY

        let month_entries: Vec<_> = fs::read_dir(&year_path).unwrap().collect();
        assert_eq!(month_entries.len(), 1); // month folder
    }

    #[test]
    fn organize_nonexistent_source_returns_error() {
        let mode = OrganizeMode { copy: false };
        let result = organize_by_date("/nonexistent/source", "/tmp/dest", &mode);
        assert_eq!(result.moved, 0);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn organize_handles_no_exif_fallback_to_mtime() {
        let source = tempfile::tempdir().unwrap();
        let dest = tempfile::tempdir().unwrap();

        // A plain file without EXIF
        let img = source.path().join("photo.jpg");
        fs::write(&img, b"fake jpeg data").unwrap();

        let mode = OrganizeMode { copy: true };
        let result = organize_by_date(
            source.path().to_str().unwrap(),
            dest.path().to_str().unwrap(),
            &mode,
        );

        // Falls back to mtime which is "now" — so it should still organize
        assert_eq!(result.skipped, 0);
    }
}
