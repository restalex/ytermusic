// TODO : Task cache management that needs to:
//          - limit further downloads if cache limit is attained
//          - delete previous cached songs that were already played
//          - not delete the played song (optionnaly the next song)
//        TEST with shuffle functionality
//        TEST with low cache size
//        TEST with large playlists (download limiter)

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::DeletingPolicy;
use crate::consts::CONFIG;

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
pub fn get_file_time(path: &Path) -> Option<SystemTime> {
    let metadata = fs::metadata(path).ok()?;

    let file_time = match CONFIG.cache.deleting_policy {
        DeletingPolicy::Mtime => metadata.modified().ok()?,
        DeletingPolicy::Atime => metadata.accessed().ok()?,
        DeletingPolicy::Ctime => metadata.created().ok()?,
    };

    Some(file_time)
}

/// Returns a vector of (path, file_time) tuples sorted by "acces" time (newest first).
pub fn get_time_sorted_files(downloads_dir: &Path) -> Vec<(PathBuf, SystemTime)> {
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
pub fn format_bytes(bytes: u64) -> String {
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
