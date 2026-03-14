use serde::Serialize;
use std::path::{Path, PathBuf};
use std::{fs, io};

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "tif", "bmp", "gif",
];

#[derive(Serialize)]
pub struct ImageInfo {
    pub path: String,
    pub relative_path: String,
    pub size: u64,
    pub extension: String,
}

#[derive(Serialize)]
pub struct ScanResult {
    pub images: Vec<ImageInfo>,
    pub total_size: u64,
    pub skipped: u32,
}

fn collect_files(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)?.flatten() {
        let ft = entry.file_type()?;
        if ft.is_file() {
            out.push(entry.path());
        } else if ft.is_dir() && recursive {
            collect_files(&entry.path(), true, out)?;
        }
    }
    Ok(())
}

pub fn scan_folder(root: &str, recursive: bool) -> Result<ScanResult, Box<dyn std::error::Error>> {
    let root_path = Path::new(root).canonicalize()?;
    let mut files = Vec::new();
    collect_files(&root_path, recursive, &mut files)?;

    let mut images = Vec::new();
    let mut total_size = 0u64;
    let mut skipped = 0u32;

    for path in &files {
        let ext = match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        {
            Some(e) => e,
            None => {
                skipped += 1;
                continue;
            }
        };

        if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            skipped += 1;
            continue;
        }

        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let relative = path
            .strip_prefix(&root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        total_size += size;
        images.push(ImageInfo {
            path: path.to_string_lossy().to_string(),
            relative_path: relative,
            size,
            extension: ext,
        });
    }

    Ok(ScanResult {
        images,
        total_size,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_finds_image_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("photo.jpg"), b"fake jpeg").unwrap();
        fs::write(dir.path().join("screenshot.png"), b"fake png").unwrap();

        let result = scan_folder(dir.path().to_str().unwrap(), false).unwrap();
        assert_eq!(result.images.len(), 2);
        assert!(result.total_size > 0);
    }

    #[test]
    fn scan_skips_non_image_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("photo.jpg"), b"fake jpeg").unwrap();
        fs::write(dir.path().join("readme.txt"), b"text file").unwrap();
        fs::write(dir.path().join("data.csv"), b"csv file").unwrap();

        let result = scan_folder(dir.path().to_str().unwrap(), false).unwrap();
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.skipped, 2);
    }

    #[test]
    fn scan_non_recursive_skips_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("top.png"), b"top level").unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.png"), b"nested").unwrap();

        let result = scan_folder(dir.path().to_str().unwrap(), false).unwrap();
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].relative_path, "top.png");

        let recursive = scan_folder(dir.path().to_str().unwrap(), true).unwrap();
        assert_eq!(recursive.images.len(), 2);
    }

    #[test]
    fn scan_invalid_path_returns_error() {
        let result = scan_folder("/nonexistent/path/that/does/not/exist", false);
        assert!(result.is_err());
    }
}
