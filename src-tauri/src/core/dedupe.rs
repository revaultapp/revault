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

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn hash_image(path: &str) -> Result<ImageHash, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let hasher = HasherConfig::new().to_hasher();
    Ok(hasher.hash_image(&img))
}

fn collect_images(paths: &[String]) -> Vec<String> {
    let mut images = Vec::new();
    for path_str in paths {
        let p = Path::new(path_str);
        if p.is_file() && is_image(p) {
            images.push(path_str.clone());
        } else if p.is_dir() {
            if let Ok(entries) = fs::read_dir(p) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if is_image(&entry_path) {
                        if let Some(s) = entry_path.to_str() {
                            images.push(s.to_string());
                        }
                    }
                }
            }
        }
    }
    images
}

pub fn find_duplicates(
    paths: &[String],
) -> Result<Vec<DuplicateGroup>, Box<dyn std::error::Error>> {
    let image_paths = collect_images(paths);

    let mut hashes: Vec<(String, ImageHash)> = Vec::with_capacity(image_paths.len());
    for path in &image_paths {
        match hash_image(path) {
            Ok(h) => hashes.push((path.clone(), h)),
            Err(_) => continue,
        }
    }

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for i in 0..hashes.len() {
        if used.contains(&i) {
            continue;
        }
        let (path_i, hash_i) = &hashes[i];

        let mut group_files: Vec<(String, u64, u64)> = vec![];
        group_files.push((
            path_i.clone(),
            fs::metadata(path_i).map(|m| m.len()).unwrap_or(0),
            fs::metadata(path_i)
                .and_then(|m| m.modified())
                .map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })
                .unwrap_or(0),
        ));
        used.insert(i);

        for (j, _) in hashes.iter().enumerate().skip(i + 1) {
            if used.contains(&j) {
                continue;
            }
            let (_, hash_j) = &hashes[j];
            let dist = hash_i.dist(hash_j);

            if dist <= 5 {
                let (path_j, _) = &hashes[j];
                used.insert(j);
                group_files.push((
                    path_j.clone(),
                    fs::metadata(path_j).map(|m| m.len()).unwrap_or(0),
                    fs::metadata(path_j)
                        .and_then(|m| m.modified())
                        .map(|t| {
                            t.duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        })
                        .unwrap_or(0),
                ));
            }
        }

        if group_files.len() > 1 {
            groups.push(DuplicateGroup {
                hash: hash_i.to_base64(),
                distance: 0,
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
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn duplicate_grouping() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        fs::write(&a, b"\x89PNG\r\n\x1a\n").unwrap();
        fs::write(&b, b"\x89PNG\r\n\x1a\n").unwrap();

        let groups = find_duplicates(&[dir.path().to_str().unwrap().to_string()]).unwrap();
        assert!(!groups.is_empty());
    }

    #[test]
    fn no_duplicates_in_empty() {
        let result = find_duplicates(&[]);
        assert!(result.is_ok());
    }
}
