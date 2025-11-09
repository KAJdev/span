use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    #[serde(default = "default_http_bind")] 
    pub http_bind: String,
    #[serde(default = "default_grpc_bind")] 
    pub grpc_bind: String,
    pub nats_url: Option<String>,
}

fn default_http_bind() -> String { "0.0.0.0:8080".into() }
fn default_grpc_bind() -> String { "0.0.0.0:50051".into() }

pub fn default_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("span/config.toml")
}

pub fn write_config(cfg: &Config, path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = path.unwrap_or_else(default_path);
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    let text = toml::to_string_pretty(cfg)?;
    fs::write(&path, text)?;
    Ok(path)
}
