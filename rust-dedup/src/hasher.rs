use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn find_duplicates(files: &[PathBuf]) -> HashMap<String, Vec<PathBuf>> {
    // Phase 1: Group by file size (fast pre-filter)
    let mut size_groups: HashMap<u64, Vec<&PathBuf>> = HashMap::new();
    for file in files {
        if let Ok(meta) = file.metadata() {
            size_groups.entry(meta.len()).or_default().push(file);
        }
    }

    // Phase 2: Only hash files that share a size with at least one other file
    let mut hash_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let candidates: Vec<&&PathBuf> = size_groups
        .values()
        .filter(|group| group.len() > 1)
        .flatten()
        .collect();

    let total = candidates.len();
    for (i, file) in candidates.iter().enumerate() {
        print!("\r  Hashing file {}/{}", i + 1, total);
        io::stdout().flush().ok();

        match hash_file(file) {
            Ok(hash) => {
                hash_groups
                    .entry(hash)
                    .or_default()
                    .push(file.to_path_buf());
            }
            Err(e) => {
                eprintln!("\n  Warning: could not hash {}: {}", file.display(), e);
            }
        }
    }

    if total > 0 {
        println!();
    }

    hash_groups.retain(|_, paths| paths.len() > 1);
    hash_groups
}