use clap::Parser;

#[derive(Parser)]
#[command(name = "rust-dedup", about = "Find and remove duplicate files")]
pub struct Args {
    /// Directory to scan for duplicates
    #[arg(default_value = ".")]
    pub path: String,

    /// Minimum file size in bytes to consider (skip tiny files)
    #[arg(short, long, default_value = "1")]
    pub min_size: u64,

    /// Delete duplicates without asking (keeps the first found copy)
    #[arg(short, long, default_value = "false")]
    pub force: bool,

    /// Only show duplicates, don't offer to delete
    #[arg(short, long, default_value = "false")]
    pub dry_run: bool,
}
