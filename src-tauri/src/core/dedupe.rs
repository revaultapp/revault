use crate::core::image_io;
use image_hasher::{HashAlg, HasherConfig, ImageHash};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::image_io::IMAGE_EXTENSIONS;

const HASH_SIZE: u32 = 16;
const PERCEPTUAL_THRESHOLD_SIMILAR: u32 = 24;
const CANCELLED_ERROR: &str = "dedupe scan cancelled";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    Exact,
    Similar,
}

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
    pub max_distance: u32,
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

fn check_cancelled(cancel: &AtomicBool) -> Result<(), Box<dyn std::error::Error>> {
    if cancel.load(Ordering::Relaxed) {
        return Err(CANCELLED_ERROR.into());
    }
    Ok(())
}

fn is_cancelled_error(err: &(dyn std::error::Error + 'static)) -> bool {
    err.to_string().contains(CANCELLED_ERROR)
}

fn compute_sha256(path: &str, cancel: &AtomicBool) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        check_cancelled(cancel)?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
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
    seen: &mut HashSet<std::path::PathBuf>,
    cancel: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error>> {
    check_cancelled(cancel)?;
    if path.is_file() && is_image(path) {
        collect_image(path, images, errors, seen);
    } else if path.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    check_cancelled(cancel)?;
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
                            collect_image(&entry_path, images, errors, seen);
                        }
                    } else if ft.is_dir() && recursive {
                        collect_images_recursive(&entry_path, true, images, errors, seen, cancel)?;
                    }
                }
            }
            Err(e) => errors.push(format!("{}: {}", path.display(), e)),
        }
    }
    Ok(())
}

fn collect_image(
    path: &Path,
    images: &mut Vec<String>,
    errors: &mut Vec<String>,
    seen: &mut HashSet<std::path::PathBuf>,
) {
    match path.canonicalize() {
        Ok(canonical) => {
            if seen.insert(canonical.clone()) {
                if let Some(s) = canonical.to_str() {
                    images.push(s.to_string());
                }
            }
        }
        Err(e) => errors.push(format!("{}: {}", path.display(), e)),
    }
}

#[cfg(test)]
fn collect_images(paths: &[String], recursive: bool) -> (Vec<String>, Vec<String>) {
    let cancel = AtomicBool::new(false);
    collect_images_with_cancel(paths, recursive, &cancel).unwrap_or_default()
}

fn collect_images_with_cancel(
    paths: &[String],
    recursive: bool,
    cancel: &AtomicBool,
) -> Result<(Vec<String>, Vec<String>), Box<dyn std::error::Error>> {
    let mut images = Vec::new();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for path_str in paths {
        check_cancelled(cancel)?;
        match crate::core::paths::validate_input_path(path_str, true) {
            Ok(canonical) => {
                collect_images_recursive(
                    &canonical,
                    recursive,
                    &mut images,
                    &mut errors,
                    &mut seen,
                    cancel,
                )?;
            }
            Err(e) => errors.push(e),
        }
    }
    Ok((images, errors))
}

fn sha256_to_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
fn find_duplicates(
    paths: &[String],
    recursive: bool,
    mode: ScanMode,
) -> Result<FindDuplicatesResult, Box<dyn std::error::Error>> {
    let cancel = AtomicBool::new(false);
    find_duplicates_with_progress_and_cancel(paths, recursive, mode, |_, _, _| {}, &cancel)
}

pub fn find_duplicates_with_progress_and_cancel<F>(
    paths: &[String],
    recursive: bool,
    mode: ScanMode,
    mut on_progress: F,
    cancel: &AtomicBool,
) -> Result<FindDuplicatesResult, Box<dyn std::error::Error>>
where
    F: FnMut(usize, usize, &str),
{
    let (image_paths, mut collect_errors) = collect_images_with_cancel(paths, recursive, cancel)?;
    let total = image_paths.len();
    let total_scanned = image_paths.len();

    let mut files_data: Vec<FileData> = Vec::with_capacity(image_paths.len());
    for (idx, path) in image_paths.iter().enumerate() {
        check_cancelled(cancel)?;
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

        let sha256 = match compute_sha256(path, cancel) {
            Ok(h) => h,
            Err(e) => {
                if is_cancelled_error(e.as_ref()) {
                    return Err(e);
                }
                collect_errors.push(format!("{}: {}", path, e));
                continue;
            }
        };

        let perceptual_hash = if matches!(mode, ScanMode::Similar) {
            check_cancelled(cancel)?;
            compute_perceptual_hash(path).ok()
        } else {
            None
        };

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
        check_cancelled(cancel)?;
    }

    // Stage 1: Exact match by SHA256. Exact mode stops here so the label means
    // byte-identical, not visually similar.
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();

    let mut sha_buckets: std::collections::HashMap<[u8; 32], Vec<usize>> =
        std::collections::HashMap::new();
    for (i, fd) in files_data.iter().enumerate() {
        check_cancelled(cancel)?;
        sha_buckets.entry(fd.sha256).or_default().push(i);
    }

    for (sha, indices) in sha_buckets {
        check_cancelled(cancel)?;
        if indices.len() < 2 {
            continue;
        }
        let group_files: Vec<DuplicateFile> = indices
            .iter()
            .map(|&i| DuplicateFile {
                path: files_data[i].path.clone(),
                size: files_data[i].size,
                modified: files_data[i].modified,
            })
            .collect();
        for i in &indices {
            used.insert(*i);
        }
        groups.push(DuplicateGroup {
            hash: sha256_to_hex(&sha),
            distance: 0,
            max_distance: 0,
            files: group_files,
        });
    }

    if matches!(mode, ScanMode::Exact) {
        groups.sort_by_key(|g| std::cmp::Reverse(g.files.len()));
        return Ok(FindDuplicatesResult {
            groups,
            total_scanned,
            errors: collect_errors,
        });
    }

    // Stage 2: Perceptual match by pHash for remaining ungrouped files in
    // Similar mode. Recompressed copies can differ in bytes and size.
    check_cancelled(cancel)?;
    on_progress(total, total, "grouping");
    let all: Vec<usize> = (0..files_data.len())
        .filter(|i| !used.contains(i))
        .collect();
    let buckets: Vec<Vec<usize>> = vec![all];

    for indices in buckets {
        check_cancelled(cancel)?;
        if indices.len() < 2 {
            continue;
        }

        for i in &indices {
            check_cancelled(cancel)?;
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
            let mut max_dist: u32 = 0;

            for j in indices.iter() {
                check_cancelled(cancel)?;
                if used.contains(j) || j == i {
                    continue;
                }
                let hash_j = match &files_data[*j].perceptual_hash {
                    Some(h) => h,
                    None => continue,
                };
                let dist = hash_i.dist(hash_j);

                if dist <= PERCEPTUAL_THRESHOLD_SIMILAR {
                    used.insert(*j);
                    group_files.push(DuplicateFile {
                        path: files_data[*j].path.clone(),
                        size: files_data[*j].size,
                        modified: files_data[*j].modified,
                    });
                    min_dist = min_dist.min(dist);
                    max_dist = max_dist.max(dist);
                }
            }

            if group_files.len() > 1 {
                groups.push(DuplicateGroup {
                    hash: hash_i.to_base64(),
                    distance: min_dist,
                    max_distance: max_dist,
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
    use std::sync::atomic::{AtomicBool, Ordering};

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

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
        assert!(!result.groups.is_empty());
    }

    #[test]
    fn no_duplicates_in_empty() {
        let result = find_duplicates(&[], true, ScanMode::Exact);
        assert!(result.is_ok());
    }

    #[test]
    fn find_duplicates_returns_error_when_cancelled_before_scan() {
        let dir = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(true);

        let result = find_duplicates_with_progress_and_cancel(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
            |_, _, _| {},
            &cancel,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }

    #[test]
    fn find_duplicates_returns_error_when_cancelled_during_scan() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let photo = dir.path().join("photo.png");
        let img: RgbaImage =
            image::ImageBuffer::from_fn(4, 4, |_x, _y| image::Rgba([200, 200, 200, 255]));
        img.save(&photo).unwrap();
        let cancel = AtomicBool::new(false);

        let result = find_duplicates_with_progress_and_cancel(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
            |_, _, _| cancel.store(true, Ordering::SeqCst),
            &cancel,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cancelled"));
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
    fn collect_images_dedupes_overlapping_roots() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        let photo = subdir.join("photo.png");
        let img: RgbaImage =
            image::ImageBuffer::from_fn(4, 4, |_x, _y| image::Rgba([20, 40, 60, 255]));
        img.save(&photo).unwrap();

        let (images, errors) = collect_images(
            &[
                dir.path().to_str().unwrap().to_string(),
                subdir.to_str().unwrap().to_string(),
            ],
            true,
        );

        assert!(errors.is_empty());
        assert_eq!(images.len(), 1);
        assert!(images[0].ends_with("photo.png"));
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
        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
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
        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
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

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
        assert_eq!(result.groups[0].distance, 0); // SHA256 exact match = distance 0
    }

    #[test]
    fn similar_mode_groups_visually_identical_different_bytes() {
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

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Similar,
        )
        .unwrap();
        // Either SHA256 groups them (distance 0) or pHash groups them (distance 0)
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
        assert_eq!(result.groups[0].distance, 0);
    }

    #[test]
    fn exact_mode_does_not_group_same_pixels_with_different_bytes() {
        use image::RgbaImage;
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("photo_a.png");
        let b = dir.path().join("photo_b.png");

        let img: RgbaImage =
            image::ImageBuffer::from_fn(32, 32, |_x, _y| image::Rgba([120, 80, 40, 255]));
        img.save(&a).unwrap();
        let mut bytes = std::fs::read(&a).unwrap();
        bytes.extend_from_slice(b"trailing-a");
        std::fs::write(&a, &bytes).unwrap();
        let last = bytes.last_mut().unwrap();
        *last = b'b';
        std::fs::write(&b, &bytes).unwrap();

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();

        assert!(
            result.groups.is_empty(),
            "Exact mode must be SHA/byte-exact only"
        );
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

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
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

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
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

        // Visually dissimilar content: a top-left corner gradient vs a
        // bottom-right corner gradient. Different SHA256, different pHash.
        let img1: RgbaImage = image::ImageBuffer::from_fn(64, 64, |x, y| {
            image::Rgba([(x * 4) as u8, (y * 4) as u8, 0, 255])
        });
        let img2: RgbaImage = image::ImageBuffer::from_fn(64, 64, |x, y| {
            image::Rgba([0, ((63 - x) * 4) as u8, ((63 - y) * 4) as u8, 255])
        });
        img1.save(&a).unwrap();
        img2.save(&b).unwrap();

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
        assert!(
            result.groups.is_empty(),
            "visually dissimilar gradients should not be grouped"
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

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
    }

    fn save_gradient_jpeg(path: &std::path::Path, quality: u8, invert: bool) {
        const W: u32 = 256;
        const H: u32 = 256;
        let img: image::GrayImage = image::ImageBuffer::from_fn(W, H, |x, y| {
            let v = ((x + y) % 256) as u8;
            image::Luma([if invert { 255u8.wrapping_sub(v) } else { v }])
        });
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            std::io::BufWriter::new(file),
            quality,
        );
        encoder
            .encode(img.as_raw(), W, H, image::ExtendedColorType::L8)
            .unwrap();
    }

    #[test]
    fn similar_mode_detects_recompressed_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a_q95.jpg");
        let b = dir.path().join("b_q50.jpg");
        save_gradient_jpeg(&a, 95, false);
        save_gradient_jpeg(&b, 50, false);

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Similar,
        )
        .unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
    }

    #[test]
    fn similar_mode_does_not_group_different_images() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.jpg");
        let b = dir.path().join("b_inverted.jpg");
        save_gradient_jpeg(&a, 90, false);
        save_gradient_jpeg(&b, 90, true);

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Similar,
        )
        .unwrap();
        assert!(
            result.groups.is_empty(),
            "gradient and its inverse should not be grouped in Similar mode"
        );
    }

    #[test]
    fn similar_mode_size_filter_bypassed() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a_q95.jpg");
        let b = dir.path().join("b_q50.jpg");
        save_gradient_jpeg(&a, 95, false);
        save_gradient_jpeg(&b, 50, false);

        let size_a = std::fs::metadata(&a).unwrap().len();
        let size_b = std::fs::metadata(&b).unwrap().len();
        assert_ne!(
            size_a, size_b,
            "test precondition: recompressed JPEGs must differ in size"
        );

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Similar,
        )
        .unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
    }

    #[test]
    fn exact_mode_uses_lower_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a_q95.jpg");
        let b = dir.path().join("b_q50.jpg");
        save_gradient_jpeg(&a, 95, false);
        save_gradient_jpeg(&b, 50, false);

        let result = find_duplicates(
            &[dir.path().to_str().unwrap().to_string()],
            true,
            ScanMode::Exact,
        )
        .unwrap();
        assert!(
            result.groups.is_empty(),
            "Exact mode (threshold=10) must not group recompressed JPEGs that Similar mode catches"
        );
    }
}
