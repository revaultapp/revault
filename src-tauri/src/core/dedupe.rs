use image_hasher::{HasherConfig, ImageHash};
use serde::Serialize;
use std::fs;
use std::path::Path;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "tif", "bmp", "gif", "avif", "jxl",
];

#[derive(Serialize, Clone)]
pub struct DuplicateFile {
    pub path: String,
    pub size: u64,
    pub modified: u64,
}

#[derive(Serialize, Clone)]
pub struct DuplicateGroup {
    pub hash: String,
    pub distance: u32,
    pub files: Vec<DuplicateFile>,
}

#[derive(Serialize)]
pub struct FindDuplicatesResult {
    pub groups: Vec<DuplicateGroup>,
    pub total_scanned: usize,
    pub errors: Vec<String>,
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn hash_image(path: &str) -> Result<ImageHash, Box<dyn std::error::Error>> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    let mut reader = image::ImageReader::open(path)?;
    reader.limits(limits);
    let img = reader.decode()?;
    // Perceptual hash only needs a small thumbnail — resize before hashing to avoid OOM
    let thumb = img.thumbnail(64, 64);
    let hasher = HasherConfig::new().to_hasher();
    Ok(hasher.hash_image(&thumb))
}

fn collect_images_recursive(
    path: &Path,
    recursive: bool,
    images: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if path.is_file() && is_image(path) {
        if let Some(s) = path.to_str() {
            images.push(s.to_string());
        }
    } else if path.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    let ft = match entry.file_type() {
                        Ok(ft) => ft,
                        Err(e) => {
                            errors.push(format!("{}: {}", entry_path.display(), e));
                            continue;
                        }
                    };
                    if ft.is_file() {
                        if is_image(&entry_path) {
                            if let Some(s) = entry_path.to_str() {
                                images.push(s.to_string());
                            }
                        }
                    } else if ft.is_dir() && recursive {
                        collect_images_recursive(&entry_path, true, images, errors);
                    }
                }
            }
            Err(e) => errors.push(format!("{}: {}", path.display(), e)),
        }
    }
}

fn collect_images(paths: &[String], recursive: bool) -> (Vec<String>, Vec<String>) {
    let mut images = Vec::new();
    let mut errors = Vec::new();
    for path_str in paths {
        collect_images_recursive(Path::new(path_str), recursive, &mut images, &mut errors);
    }
    (images, errors)
}

pub fn find_duplicates(
    paths: &[String],
    recursive: bool,
) -> Result<FindDuplicatesResult, Box<dyn std::error::Error>> {
    let (image_paths, mut collect_errors) = collect_images(paths, recursive);
    let total_scanned = image_paths.len();

    let mut hashes: Vec<(String, ImageHash)> = Vec::with_capacity(image_paths.len());
    for path in &image_paths {
        match hash_image(path) {
            Ok(h) => hashes.push((path.clone(), h)),
            Err(e) => collect_errors.push(format!("{}: {}", path, e)),
        }
    }

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for i in 0..hashes.len() {
        if used.contains(&i) {
            continue;
        }
        let (path_i, hash_i) = &hashes[i];

        let meta_i = match fs::metadata(path_i) {
            Ok(m) => m,
            Err(e) => {
                collect_errors.push(format!("{}: metadata: {}", path_i, e));
                continue;
            }
        };
        let modified_i = meta_i
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or(0);
        let size_i = meta_i.len();

        let mut group_files: Vec<(String, u64, u64)> = vec![(path_i.clone(), size_i, modified_i)];
        used.insert(i);

        let mut min_dist: u32 = u32::MAX;

        for (j, _) in hashes.iter().enumerate().skip(i + 1) {
            if used.contains(&j) {
                continue;
            }
            let (_, hash_j) = &hashes[j];
            let dist = hash_i.dist(hash_j);

            if dist <= 5 {
                let (path_j, _) = &hashes[j];
                let meta_j = match fs::metadata(path_j) {
                    Ok(m) => m,
                    Err(e) => {
                        collect_errors.push(format!("{}: metadata: {}", path_j, e));
                        continue;
                    }
                };
                let modified_j = meta_j
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    })
                    .unwrap_or(0);
                let size_j = meta_j.len();
                used.insert(j);
                group_files.push((path_j.clone(), size_j, modified_j));
                min_dist = min_dist.min(dist);
            }
        }

        if group_files.len() > 1 {
            groups.push(DuplicateGroup {
                hash: hash_i.to_base64(),
                distance: min_dist,
                files: group_files
                    .into_iter()
                    .map(|(path, size, modified)| DuplicateFile {
                        path,
                        size,
                        modified,
                    })
                    .collect(),
            });
        }
    }

    groups.sort_by(|a, b| b.files.len().cmp(&a.files.len()));
    Ok(FindDuplicatesResult {
        groups,
        total_scanned,
        errors: collect_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_grouping() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        let img: RgbaImage =
            image::ImageBuffer::from_fn(4, 4, |_x, _y| image::Rgba([200, 200, 200, 255]));
        img.save(&a).unwrap();
        img.save(&b).unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert!(!result.groups.is_empty());
    }

    #[test]
    fn no_duplicates_in_empty() {
        let result = find_duplicates(&[], true);
        assert!(result.is_ok());
    }

    #[test]
    fn collect_images_recursive_finds_nested_files() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let photo1 = dir.path().join("photo1.png");
        let subdir = dir.path().join("subdir");
        let photo2 = subdir.join("photo2.png");
        let deeper = dir.path().join("deeper");
        let nested = deeper.join("nested");
        let photo3 = nested.join("photo3.png");

        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::create_dir_all(&nested).unwrap();

        let img: RgbaImage =
            image::ImageBuffer::from_fn(4, 4, |_x, _y| image::Rgba([200, 200, 200, 255]));
        img.save(&photo1).unwrap();
        img.save(&photo2).unwrap();
        img.save(&photo3).unwrap();

        let (images, _errors) = collect_images(&[dir.path().to_str().unwrap().to_string()], true);
        assert_eq!(images.len(), 3);
        assert!(images.iter().any(|p| p.contains("photo1.png")));
        assert!(images.iter().any(|p| p.contains("photo2.png")));
        assert!(images.iter().any(|p| p.contains("photo3.png")));
    }

    #[test]
    fn find_duplicates_with_corrupt_image() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let valid1 = dir.path().join("valid1.png");
        let valid2 = dir.path().join("valid2.png");
        let corrupt = dir.path().join("corrupt.png");

        let img: RgbaImage =
            image::ImageBuffer::from_fn(4, 4, |_x, _y| image::Rgba([200, 200, 200, 255]));
        img.save(&valid1).unwrap();
        img.save(&valid2).unwrap();
        std::fs::write(&corrupt, b"not a real png").unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.contains("corrupt.png")));
    }

    #[test]
    fn find_duplicates_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert!(result.groups.is_empty());
        assert_eq!(result.total_scanned, 0);
        assert!(result.errors.is_empty());
    }
}
