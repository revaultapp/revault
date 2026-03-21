use exif::{In, Reader};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameRequest {
    pub original_path: String,
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub original_path: String,
    pub new_path: String,
    pub success: bool,
    pub error: Option<String>,
}

impl RenameResult {
    pub fn ok(original: &str, new: &str) -> Self {
        Self {
            original_path: original.to_string(),
            new_path: new.to_string(),
            success: true,
            error: None,
        }
    }

    pub fn err(original: &str, msg: String) -> Self {
        Self {
            original_path: original.to_string(),
            new_path: String::new(),
            success: false,
            error: Some(msg),
        }
    }
}

fn read_exif_date(path: &str) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let exif = Reader::new()
        .read_from_container(&mut BufReader::new(&file))
        .ok()?;
    let field = exif.get_field(exif::Tag::DateTimeOriginal, In::PRIMARY)?;
    let s = field.display_value().to_string();
    let s = s.trim().trim_matches('"');
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn parse_exif_datetime(s: &str) -> Option<(String, String, String)> {
    // Format: "YYYY:MM:DD HH:MM:SS"
    let parts: Vec<&str> = s.split([' ', ':']).collect();
    if parts.len() >= 3 {
        Some((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ))
    } else {
        None
    }
}

fn get_date_tokens(path: &str) -> Option<(String, String, String)> {
    if let Some(exif_dt) = read_exif_date(path) {
        if let Some((y, m, d)) = parse_exif_datetime(&exif_dt) {
            return Some((y, m, d));
        }
    }
    // Fallback to file mtime
    let metadata = fs::metadata(path).ok()?;
    let mtime = metadata.modified().ok()?;
    let datetime = mtime.duration_since(std::time::UNIX_EPOCH).ok()?;
    let secs = datetime.as_secs();
    let (year, month, day) = civil_from_secs(secs);
    Some((
        year.to_string(),
        format!("{:02}", month),
        format!("{:02}", day),
    ))
}

/// Convert seconds since UNIX_EPOCH to (year, month, day).
fn civil_from_secs(secs: u64) -> (i32, u8, u8) {
    let days = secs / 86400;
    let mut year = 1970;
    let mut remaining_days = days as i64;

    // Advance year until we run out of days
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Days per month, adjusted for leap year
    let month_days = if is_leap_year(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &d in month_days {
        if remaining_days < d as i64 {
            break;
        }
        remaining_days -= d as i64;
        month += 1;
    }

    let day = remaining_days + 1;
    (year, month as u8, day as u8)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn apply_template(template: &str, stem: &str, ext: &str, counter: u32, path: &str) -> String {
    let mut result = template.to_string();

    result = result.replace("{name}", stem);
    result = result.replace("{ext}", ext);

    // {counter} — default 3 digits
    result = result.replace("{counter}", &format!("{:03}", counter));
    // {counter:N} for N in [2,4,5]
    for digits in [2, 4, 5] {
        let placeholder = format!("{{counter:{}}}", digits);
        result = result.replace(
            &placeholder,
            &format!("{:0width$}", counter, width = digits),
        );
    }

    if let Some((year, month, day)) = get_date_tokens(path) {
        result = result.replace("{date}", &format!("{}-{:02}-{:02}", year, month, day));
        result = result.replace("{year}", &year);
        result = result.replace("{month}", &format!("{:02}", month));
        result = result.replace("{day}", &format!("{:02}", day));
        // {datetime} uses mtime hours/min/sec (not from EXIF, just from file)
        let metadata = fs::metadata(path).ok();
        let mtime = metadata.and_then(|m| m.modified().ok());
        if let Some(st) = mtime {
            let dur = st.duration_since(std::time::UNIX_EPOCH).ok().unwrap();
            let total_secs = dur.as_secs();
            let hour = (total_secs / 3600) % 24;
            let min = (total_secs / 60) % 60;
            let sec = total_secs % 60;
            result = result.replace(
                "{datetime}",
                &format!(
                    "{}-{:02}-{:02}_{:02}-{:02}-{:02}",
                    year, month, day, hour, min, sec
                ),
            );
        } else {
            result = result.replace("{datetime}", &format!("{:03}", counter));
        }
    } else {
        result = result.replace("{date}", &format!("{:03}", counter));
        result = result.replace("{datetime}", &format!("{:03}", counter));
        result = result.replace("{year}", &format!("{:03}", counter));
        result = result.replace("{month}", &format!("{:03}", counter));
        result = result.replace("{day}", &format!("{:03}", counter));
    }

    result
}

pub fn rename_batch(requests: &[RenameRequest]) -> Vec<RenameResult> {
    requests
        .iter()
        .map(|req| {
            let input_path = Path::new(&req.original_path);

            let stem = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");

            let ext = input_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            let parent = match input_path.parent() {
                Some(p) => p,
                None => return RenameResult::err(&req.original_path, "no parent directory".into()),
            };

            let mut counter = 1u32;
            loop {
                let new_name =
                    apply_template(&req.template, stem, ext, counter, &req.original_path);
                let new_path = parent.join(&new_name);
                let new_path_str = new_path.to_string_lossy().into_owned();

                if new_path_str == req.original_path {
                    return RenameResult::ok(&req.original_path, &new_path_str);
                }

                if !new_path.exists() {
                    match fs::rename(&req.original_path, &new_path) {
                        Ok(_) => return RenameResult::ok(&req.original_path, &new_path_str),
                        Err(e) => return RenameResult::err(&req.original_path, e.to_string()),
                    }
                }
                counter += 1;
                if counter > 99999 {
                    return RenameResult::err(
                        &req.original_path,
                        "could not find unique name after 99999 attempts".into(),
                    );
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rename_counter_token() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        fs::write(&input, b"fake").unwrap();

        let requests = &[RenameRequest {
            original_path: input.to_string_lossy().to_string(),
            template: "{name}_{counter}.{ext}".to_string(),
        }];
        let results = rename_batch(requests);
        assert!(results[0].success);
        assert_eq!(
            results[0].new_path,
            dir.path().join("photo_001.jpg").to_string_lossy()
        );
    }

    #[test]
    fn rename_counter_custom_digits() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        fs::write(&input, b"fake").unwrap();

        let requests = &[RenameRequest {
            original_path: input.to_string_lossy().to_string(),
            template: "{name}_{counter:5}.{ext}".to_string(),
        }];
        let results = rename_batch(requests);
        assert!(results[0].success);
        assert_eq!(
            results[0].new_path,
            dir.path().join("photo_00001.jpg").to_string_lossy()
        );
    }

    #[test]
    fn rename_preserves_extension() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpeg");
        fs::write(&input, b"fake").unwrap();

        let requests = &[RenameRequest {
            original_path: input.to_string_lossy().to_string(),
            template: "IMG_{counter}.{ext}".to_string(),
        }];
        let results = rename_batch(requests);
        assert!(results[0].success);
        assert!(results[0].new_path.ends_with(".jpeg"));
    }

    #[test]
    fn rename_handles_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("photo.jpg");
        fs::write(&input, b"fake").unwrap();
        let existing = dir.path().join("photo_001.jpg");
        fs::write(&existing, b"existing").unwrap();

        let requests = &[RenameRequest {
            original_path: input.to_string_lossy().to_string(),
            template: "{name}_{counter}.{ext}".to_string(),
        }];
        let results = rename_batch(requests);
        assert!(results[0].success);
        assert_eq!(
            results[0].new_path,
            dir.path().join("photo_002.jpg").to_string_lossy()
        );
    }

    #[test]
    fn rename_invalid_path() {
        let requests = &[RenameRequest {
            original_path: "/nonexistent/photo.jpg".to_string(),
            template: "{name}_new.{ext}".to_string(),
        }];
        let results = rename_batch(requests);
        assert!(!results[0].success);
        assert!(results[0].error.is_some());
    }
}
