use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::format::format_size;

pub fn report_and_handle(
    duplicates: &HashMap<String, Vec<PathBuf>>,
    dry_run: bool,
    force: bool,
) {
    if duplicates.is_empty() {
        println!("{}", "No duplicates found!".green().bold());
        return;
    }

    let total_groups = duplicates.len();
    let total_dupes: usize = duplicates.values().map(|v| v.len() - 1).sum();
    let wasted_bytes: u64 = duplicates
        .values()
        .map(|paths| {
            let size = paths[0].metadata().map(|m| m.len()).unwrap_or(0);
            size * (paths.len() as u64 - 1)
        })
        .sum();

    println!(
        "\n{} Found {} duplicate group(s), {} extra file(s), wasting {}",
        "=>".yellow().bold(),
        total_groups.to_string().cyan(),
        total_dupes.to_string().cyan(),
        format_size(wasted_bytes).red().bold()
    );

    let mut deleted_count = 0u64;
    let mut deleted_bytes = 0u64;

    for (i, (_hash, paths)) in duplicates.iter().enumerate() {
        let size = paths[0].metadata().map(|m| m.len()).unwrap_or(0);
        println!(
            "\n{} Group {} — {} each, {} copies:",
            "##".blue().bold(),
            (i + 1).to_string().bold(),
            format_size(size).yellow(),
            paths.len()
        );

        for (j, path) in paths.iter().enumerate() {
            let label = if j == 0 {
                "[keep]".green().to_string()
            } else {
                "[dupe]".red().to_string()
            };
            println!("  {} {}", label, path.display());
        }

        if dry_run {
            continue;
        }

        let dupes = &paths[1..];

        if force {
            for dupe in dupes {
                match fs::remove_file(dupe) {
                    Ok(()) => {
                        deleted_count += 1;
                        deleted_bytes += size;
                        println!("  {} {}", "Deleted:".red(), dupe.display());
                    }
                    Err(e) => {
                        eprintln!("  Error deleting {}: {}", dupe.display(), e);
                    }
                }
            }
        } else {
            print!(
                "  Delete {} duplicate(s)? [y/N] ",
                dupes.len().to_string().bold()
            );
            io::stdout().flush().ok();

            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();

            if input.trim().eq_ignore_ascii_case("y") {
                for dupe in dupes {
                    match fs::remove_file(dupe) {
                        Ok(()) => {
                            deleted_count += 1;
                            deleted_bytes += size;
                            println!("  {} {}", "Deleted:".red(), dupe.display());
                        }
                        Err(e) => {
                            eprintln!("  Error deleting {}: {}", dupe.display(), e);
                        }
                    }
                }
            } else {
                println!("  {}", "Skipped.".dimmed());
            }
        }
    }

    if !dry_run && deleted_count > 0 {
        println!(
            "\n{} Cleaned up {} file(s), freed {}",
            "=>".green().bold(),
            deleted_count.to_string().cyan(),
            format_size(deleted_bytes).green().bold()
        );
    }
}