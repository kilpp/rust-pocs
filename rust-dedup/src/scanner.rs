use std::path::PathBuf;
use walkdir::WalkDir;

pub fn scan_files(root: &str, min_size: u64) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Ok(meta) = path.metadata() {
                if meta.len() >= min_size {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    files
}