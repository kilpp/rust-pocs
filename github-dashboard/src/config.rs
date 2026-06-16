use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "github-dashboard", about = "GitHub contributions dashboard in your terminal")]
pub struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    /// Override the timeline window, in days
    #[arg(short, long)]
    pub days: Option<u32>,
}

fn default_base_url() -> String {
    "https://api.github.com".to_string()
}

fn default_timeline_days() -> u32 {
    7
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_base_url")]
    pub base_url: String,

    #[serde(default)]
    pub token: Option<String>,

    #[serde(default)]
    pub users: Vec<String>,

    #[serde(default = "default_timeline_days")]
    pub timeline_days: u32,
}

impl Config {
    /// Load and validate configuration from the file referenced by `args`,
    /// applying the `GITHUB_TOKEN` fallback and the `--days` override.
    pub fn load(args: &Args) -> Result<Config, String> {
        let raw = std::fs::read_to_string(&args.config).map_err(|e| {
            format!(
                "could not read config file {}: {e}\nHint: copy config.example.toml to config.toml",
                args.config.display()
            )
        })?;

        let mut config: Config =
            toml::from_str(&raw).map_err(|e| format!("invalid config file: {e}"))?;

        // Trim a trailing slash so we can join paths uniformly.
        config.base_url = config.base_url.trim_end_matches('/').to_string();

        // Token: prefer the file, fall back to the environment.
        if config.token.as_deref().map(str::trim).unwrap_or("").is_empty() {
            config.token = std::env::var("GITHUB_TOKEN").ok();
        }

        if let Some(days) = args.days {
            config.timeline_days = days;
        }

        if config.users.is_empty() {
            return Err("no users configured: add at least one entry to `users`".to_string());
        }

        if config.token.as_deref().map(str::trim).unwrap_or("").is_empty() {
            return Err(
                "no token found: set `token` in the config file or the GITHUB_TOKEN env var"
                    .to_string(),
            );
        }

        if config.timeline_days == 0 {
            return Err("timeline_days must be greater than 0".to_string());
        }

        Ok(config)
    }

    pub fn token(&self) -> &str {
        self.token.as_deref().unwrap_or_default()
    }
}
