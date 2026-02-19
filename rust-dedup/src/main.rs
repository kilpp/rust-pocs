mod cli;
mod format;
mod hasher;
mod reporter;
mod scanner;

use clap::Parser;
use colored::Colorize;

use cli::Args;
use format::format_size;
use hasher::find_duplicates;
use reporter::report_and_handle;
use scanner::scan_files;

fn main() {
    let args = Args::parse();

    println!(
        "{} Scanning {} ...",
        "=>".blue().bold(),
        args.path.bold()
    );

    let files = scan_files(&args.path, args.min_size);
    println!(
        "  Found {} file(s) (min size: {})",
        files.len().to_string().cyan(),
        format_size(args.min_size)
    );

    println!("{} Looking for duplicates...", "=>".blue().bold());
    let duplicates = find_duplicates(&files);

    report_and_handle(&duplicates, args.dry_run, args.force);
}