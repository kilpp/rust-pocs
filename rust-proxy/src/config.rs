use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen: SocketAddr,
    pub strategy: Strategy,
    pub backends: Vec<BackendConfig>,
}

#[derive(Debug, Deserialize)]
pub struct BackendConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    RoundRobin,
    LeastConnections,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file '{}': {}", path, e))?;
        let config: Config = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file '{}': {}", path, e))?;
        if config.backends.is_empty() {
            return Err(anyhow::anyhow!("Config must have at least one backend"));
        }
        Ok(config)
    }
}
