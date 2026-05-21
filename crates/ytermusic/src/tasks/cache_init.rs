use crate::{
    cache_utils,
    consts::{CACHE_DIR, CONFIG},
    run_service,
    structures::performance,
    DATABASE,
};
use log::{info, warn};
use std::fs;

/// Enforce the cache limit by deleting the least accessed files on startup
pub fn spawn_cache_task() {
    run_service(async move {
        let guard = performance::guard("Cache task");
        let downloads_dir = CACHE_DIR.join("downloads");

        let max_size_bytes = match cache_utils::parse_size_to_bytes(CONFIG.cache.max_size.clone()) {
            Some(size) => size,
            None => {
                warn!("Invalid cache size format, please check your config file");
                return;
            }
        };

        if max_size_bytes == 0 {
            return; // 0MB or 0GB means no cache limit
        }

        let mut current_size = cache_utils::get_cache_size(&downloads_dir);

        info!(
            "Current cache size: {}, Limit: {}",
            cache_utils::format_bytes(current_size),
            cache_utils::format_bytes(max_size_bytes)
        );

        if current_size <= max_size_bytes {
            return;
        }

        let mut files_with_date = cache_utils::get_time_sorted_files(&downloads_dir);

        while current_size > max_size_bytes {
            if let Some((file_path, _)) = files_with_date.pop() {
                let file_size = fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
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
                        let _ = fs::remove_file(&json_path); // remove json if it exists
                        info!(
                            "Deleted cached song '{}' to enforce cache limit (freed {})",
                            file_path.file_name().unwrap_or_default().display(),
                            cache_utils::format_bytes(file_size)
                        );
                        current_size -= file_size;
                    }
                }
            } else {
                break;
            }
        }
        DATABASE.fix_db();
        DATABASE.write();
        info!(
            "Cache cleanup complete. New cache size: {}",
            cache_utils::format_bytes(current_size)
        );
        drop(guard);
    });
}
