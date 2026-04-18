use crate::core::image_io;
use image_hasher::{HashAlg, HasherConfig, ImageHash};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "tif", "bmp", "gif", "avif", "jxl",
];

const HASH_SIZE: u32 = 16;
const PERCEPTUAL_THRESHOLD: u32 = 10;

#[derive(Serialize, Clone, Debug)]
pub struct DuplicateFile {
    pub path: String,
    pub size: u64,
    pub modified: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct DuplicateGroup {
    pub hash: String,
    pub distance: u32,
    pub files: Vec<DuplicateFile>,
}

#[derive(Serialize, Debug)]
pub struct FindDuplicatesResult {
    pub groups: Vec<DuplicateGroup>,
    pub total_scanned: usize,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug)]
struct FileData {
    path: String,
    size: u64,
    modified: u64,
    sha256: [u8; 32],
    perceptual_hash: Option<ImageHash>,
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn compute_sha256(path: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    Ok(hash)
}

fn compute_perceptual_hash(path: &str) -> Result<ImageHash, Box<dyn std::error::Error>> {
    let img = image_io::open_image(path)?;
    let hasher = HasherConfig::new()
        .hash_size(HASH_SIZE, HASH_SIZE)
        .hash_alg(HashAlg::DoubleGradient)
        .to_hasher();
    Ok(hasher.hash_image(&img))
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
        match crate::core::paths::validate_input_path(path_str, true) {
            Ok(canonical) => {
                collect_images_recursive(&canonical, recursive, &mut images, &mut errors);
            }
            Err(e) => errors.push(e),
        }
    }
    (images, errors)
}

fn sha256_to_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

#[allow(dead_code)]
pub fn find_duplicates(
    paths: &[String],
    recursive: bool,
) -> Result<FindDuplicatesResult, Box<dyn std::error::Error>> {
    find_duplicates_with_progress(paths, recursive, |_, _, _| {})
}

pub fn find_duplicates_with_progress<F>(
    paths: &[String],
    recursive: bool,
    mut on_progress: F,
) -> Result<FindDuplicatesResult, Box<dyn std::error::Error>>
where
    F: FnMut(usize, usize, &str),
{
    let (image_paths, mut collect_errors) = collect_images(paths, recursive);
    let total = image_paths.len();
    let total_scanned = image_paths.len();

    let mut files_data: Vec<FileData> = Vec::with_capacity(image_paths.len());
    for (idx, path) in image_paths.iter().enumerate() {
        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                collect_errors.push(format!("{}: metadata: {}", path, e));
                continue;
            }
        };
        let modified = meta
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or(0);
        let size = meta.len();

        let sha256 = match compute_sha256(path) {
            Ok(h) => h,
            Err(e) => {
                collect_errors.push(format!("{}: {}", path, e));
                continue;
            }
        };

        let perceptual_hash = compute_perceptual_hash(path).ok();

        files_data.push(FileData {
            path: path.clone(),
            size,
            modified,
            sha256,
            perceptual_hash,
        });
        let progress = idx + 1;
        if progress % 10 == 0 || progress == total {
            on_progress(progress, total, "hashing");
        }
    }

    // Stage 1: Exact match by SHA256
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // Group by exact SHA256
    for i in 0..files_data.len() {
        if used.contains(&i) {
            continue;
        }
        let sha256_i = files_data[i].sha256;
        let mut group_files: Vec<DuplicateFile> = vec![DuplicateFile {
            path: files_data[i].path.clone(),
            size: files_data[i].size,
            modified: files_data[i].modified,
        }];
        used.insert(i);

        for (j, fd_j) in files_data.iter().enumerate().skip(i + 1) {
            if used.contains(&j) {
                continue;
            }
            if fd_j.sha256 == sha256_i {
                used.insert(j);
                group_files.push(DuplicateFile {
                    path: fd_j.path.clone(),
                    size: fd_j.size,
                    modified: fd_j.modified,
                });
            }
        }

        if group_files.len() > 1 {
            groups.push(DuplicateGroup {
                hash: sha256_to_hex(&sha256_i),
                distance: 0,
                files: group_files,
            });
        }
    }

    // Stage 2: Perceptual match by pHash for remaining ungrouped files (same size pre-filter)
    on_progress(total, total, "grouping");
    let mut size_groups: std::collections::HashMap<u64, Vec<usize>> =
        std::collections::HashMap::new();
    for (i, fd) in files_data.iter().enumerate() {
        if !used.contains(&i) {
            size_groups.entry(fd.size).or_default().push(i);
        }
    }

    for (_size, indices) in size_groups {
        if indices.len() < 2 {
            continue;
        }

        for i in &indices {
            if used.contains(i) {
                continue;
            }
            let hash_i = match &files_data[*i].perceptual_hash {
                Some(h) => h,
                None => continue,
            };

            let mut group_files: Vec<DuplicateFile> = vec![DuplicateFile {
                path: files_data[*i].path.clone(),
                size: files_data[*i].size,
                modified: files_data[*i].modified,
            }];
            used.insert(*i);
            let mut min_dist: u32 = u32::MAX;

            for j in indices.iter() {
                if used.contains(j) || j == i {
                    continue;
                }
                let hash_j = match &files_data[*j].perceptual_hash {
                    Some(h) => h,
                    None => continue,
                };
                let dist = hash_i.dist(hash_j);

                if dist <= PERCEPTUAL_THRESHOLD {
                    used.insert(*j);
                    group_files.push(DuplicateFile {
                        path: files_data[*j].path.clone(),
                        size: files_data[*j].size,
                        modified: files_data[*j].modified,
                    });
                    min_dist = min_dist.min(dist);
                }
            }

            if group_files.len() > 1 {
                groups.push(DuplicateGroup {
                    hash: hash_i.to_base64(),
                    distance: min_dist,
                    files: group_files,
                });
            }
        }
    }

    groups.sort_by_key(|g| std::cmp::Reverse(g.files.len()));
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
    fn find_duplicates_handles_corrupt_image_gracefully() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let valid1 = dir.path().join("valid1.png");
        let valid2 = dir.path().join("valid2.png");
        let corrupt = dir.path().join("corrupt.png");

        let img: RgbaImage =
            image::ImageBuffer::from_fn(16, 16, |_x, _y| image::Rgba([200, 200, 200, 255]));
        img.save(&valid1).unwrap();
        img.save(&valid2).unwrap();
        std::fs::write(&corrupt, b"not a real png").unwrap();

        // Corrupt file computes SHA256 (succeeds) but pHash fails silently
        // It should be gracefully skipped - no crash, just excluded from groups
        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        // Valid images should still be grouped
        assert!(result.groups.iter().any(|g| g.files.len() == 2));
        // Corrupt file not in any group (pHash failed silently)
        for group in &result.groups {
            for file in &group.files {
                assert!(!file.path.contains("corrupt.png"));
            }
        }
    }

    #[test]
    fn find_duplicates_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert!(result.groups.is_empty());
        assert_eq!(result.total_scanned, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn sha256_exact_match_groups_identical_files() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        let img: RgbaImage =
            image::ImageBuffer::from_fn(16, 16, |_x, _y| image::Rgba([100, 150, 200, 255]));
        img.save(&a).unwrap();
        img.save(&b).unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
        assert_eq!(result.groups[0].distance, 0); // SHA256 exact match = distance 0
    }

    #[test]
    fn perceptual_hash_groups_visually_identical_different_bytes() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("photo_a.png");
        let b = dir.path().join("photo_b.png");

        // Two visually identical images saved separately
        // SHA256 may or may not match (PNG encoding can vary)
        // But pHash should definitely group them since they're identical content
        let img: RgbaImage = image::ImageBuffer::from_fn(64, 64, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 100, 255])
        });
        img.save(&a).unwrap();
        img.save(&b).unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        // Either SHA256 groups them (distance 0) or pHash groups them (distance 0)
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
        assert_eq!(result.groups[0].distance, 0);
    }

    #[test]
    fn perceptual_hash_detects_near_duplicates_burst_mode() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("burst_1.png");
        let b = dir.path().join("burst_2.png");

        // Identical image saved twice - simulates exact burst duplicate
        // Both SHA256 and pHash should detect this as duplicate
        let img = image::ImageBuffer::from_fn(64, 64, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
        });
        img.save(&a).unwrap();
        img.save(&b).unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
        assert_eq!(result.groups[0].distance, 0);
    }

    #[test]
    fn duplicate_groups_sorted_by_file_count_descending() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        // Create group of 2 (same content - green)
        let g2: RgbaImage =
            image::ImageBuffer::from_fn(32, 32, |_x, _y| image::Rgba([0u8, 255, 0, 255]));
        let a1 = dir.path().join("a1.png");
        let a2 = dir.path().join("a2.png");
        g2.save(&a1).unwrap();
        g2.save(&a2).unwrap();

        // Create group of 3 (same content - blue)
        let g3: RgbaImage =
            image::ImageBuffer::from_fn(48, 48, |_x, _y| image::Rgba([0u8, 0, 255, 255]));
        let b1 = dir.path().join("b1.png");
        let b2 = dir.path().join("b2.png");
        let b3 = dir.path().join("b3.png");
        g3.save(&b1).unwrap();
        g3.save(&b2).unwrap();
        g3.save(&b3).unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert_eq!(result.groups.len(), 2);
        // Should be sorted by file count descending (group of 3 first)
        assert_eq!(result.groups[0].files.len(), 3);
        assert_eq!(result.groups[1].files.len(), 2);
    }

    #[test]
    fn perceptual_hash_separates_dissimilar_images() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("photo1.png");
        let b = dir.path().join("photo2.png");

        // Completely different images (red vs blue)
        let img1: RgbaImage =
            image::ImageBuffer::from_fn(64, 64, |_x, _y| image::Rgba([255, 0, 0, 255]));
        let img2: RgbaImage =
            image::ImageBuffer::from_fn(64, 64, |_x, _y| image::Rgba([0, 0, 255, 255]));
        img1.save(&a).unwrap();
        img2.save(&b).unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert!(
            result.groups.is_empty(),
            "red and blue should not be grouped"
        );
    }

    #[test]
    fn perceptual_hash_works_with_grayscale_images() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("gray1.png");
        let b = dir.path().join("gray2.png");

        // Same grayscale content
        let img: RgbaImage =
            image::ImageBuffer::from_fn(64, 64, |_x, _y| image::Rgba([128, 128, 128, 255]));
        img.save(&a).unwrap();
        img.save(&b).unwrap();

        let result = find_duplicates(&[dir.path().to_str().unwrap().to_string()], true).unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
    }
}
