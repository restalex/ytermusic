// TODO : Task cache management that needs to:
//          - limit further downloads if cache limit is attained
//          - delete previous cached songs that were already played
//          - not delete the played song (optionnaly the next song)
//        TEST with shuffle functionality
//        TEST with low cache size
//        TEST with large playlists (download limiter)

use log::{info, warn};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fs, future};
use ytpapi2::YoutubeMusicVideoRef;

use crate::config::DeletingPolicy;
use crate::consts::CACHE_DIR;
use crate::consts::CONFIG;
use crate::DATABASE;

/// Parse a size string like "500MB" or "2GB" into bytes.
/// Returns None if the string is invalid or no limit specified.
pub fn parse_size_to_bytes(size_str: String) -> Option<u64> {
    let size_str = size_str.trim().to_uppercase();

    if size_str.ends_with("MB") {
        let num_str = size_str.trim_end_matches("MB").trim();
        if let Ok(num) = num_str.parse::<u64>() {
            return Some(num * 1024 * 1024);
        }
    }
    if size_str.ends_with("GB") {
        let num_str = size_str.trim_end_matches("GB").trim();
        if let Ok(num) = num_str.parse::<u64>() {
            return Some(num * 1024 * 1024 * 1024);
        }
    }
    if size_str == "" {
        return Some(0);
    }
    None
}

/// get file time in function of the deleting policy
fn get_file_time(path: &Path) -> Option<SystemTime> {
    let metadata = fs::metadata(path).ok()?;

    let file_time = match CONFIG.cache.deleting_policy {
        DeletingPolicy::Mtime => metadata.modified().ok()?,
        DeletingPolicy::Atime => metadata.accessed().ok()?,
        DeletingPolicy::Ctime => metadata.created().ok()?,
    };

    Some(file_time)
}

/// Returns a vector of (path, file_time) tuples sorted by "acces" time (newest first).
fn get_time_sorted_files(downloads_dir: &Path) -> Vec<(PathBuf, SystemTime)> {
    let mut files_with_time: Vec<(PathBuf, SystemTime)> = Vec::new();

    if !downloads_dir.exists() || !downloads_dir.is_dir() {
        return files_with_time;
    }

    for entry in fs::read_dir(downloads_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() && path.extension().unwrap() == "mp4" {
            match get_file_time(&path) {
                Some(time) => files_with_time.push((path, time)),
                None => files_with_time.push((path, SystemTime::UNIX_EPOCH)), // needs win testing
            };
        }
    }
    files_with_time.sort_by_key(|(_, time)| *time);
    files_with_time.reverse();
    files_with_time
}

/// Calculate the total size of all files in the downloads directory.
pub fn get_cache_size(downloads_dir: &Path) -> u64 {
    let mut total_size: u64 = 0;

    if !downloads_dir.exists() || !downloads_dir.is_dir() {
        return 0;
    }

    for entry in fs::read_dir(downloads_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() && path.extension().unwrap() == "mp4" {
            // json sizes are negligible compared to mp4s
            total_size += path.metadata().unwrap().len();
        }
    }

    total_size
}

/// Format bytes into a human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Enforce the cache limit by deleting the least accessed files on startup
pub fn startup_cache_limit() {
    let downloads_dir = CACHE_DIR.join("downloads");

    let max_size_bytes = match parse_size_to_bytes(CONFIG.cache.max_size.clone()) {
        Some(size) => size,
        None => {
            warn!("Invalid cache size format, please check your config file");
            return;
        }
    };

    if max_size_bytes == 0 {
        return; // 0MB or 0GB means no cache limit
    }

    let mut current_size = get_cache_size(&downloads_dir);

    info!(
        "Current cache size: {}, Limit: {}",
        format_bytes(current_size),
        format_bytes(max_size_bytes)
    );

    if current_size <= max_size_bytes {
        return;
    }

    let mut files_with_date = get_time_sorted_files(&downloads_dir);

    while current_size > max_size_bytes {
        if let Some((file_path, _)) = files_with_date.pop() {
            let file_size = fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
            let video_id = file_path
                .file_stem()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
                .unwrap();
            let json_path = file_path.with_extension("json");

            match fs::remove_file(&file_path) {
                Err(e) => {
                    warn!(
                        "Failed to delete mp4'{}': {}",
                        file_path.file_name().unwrap_or_default().display(),
                        e
                    );
                }
                Ok(_) => {
                    let video = YoutubeMusicVideoRef {
                        title: String::default(),
                        author: String::default(),
                        album: String::default(),
                        video_id,
                        duration: String::default(),
                    }; // we only need the video id
                    let _ = fs::remove_file(&json_path); // remove json if it exists
                    info!(
                        "Deleted cached song '{}' to enforce cache limit (freed {})",
                        file_path.file_name().unwrap_or_default().display(),
                        format_bytes(file_size)
                    );
                    current_size -= file_size;
                }
            }
        } else {
            break;
        }
    }
    DATABASE.fix_db();
    info!(
        "Cache cleanup complete. New cache size: {}",
        format_bytes(current_size)
    );
}

/// Initialize cache management on startup.
pub fn init_cache_management() {
    startup_cache_limit();
}
