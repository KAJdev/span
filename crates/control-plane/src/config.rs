use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Debug, Deserialize, Clone)]
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

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut cfg = Config {
            database_url: env::var("DATABASE_URL").or_else(|_| env::var("SPAN_DATABASE_URL"))
                .unwrap_or_default(),
            http_bind: default_http_bind(),
            grpc_bind: default_grpc_bind(),
            nats_url: None,
        };

        // Load from file in priority order
        let candidates: Vec<PathBuf> = [
            env::var("SPAN_CONFIG").ok().map(Into::into),
            Some(PathBuf::from("/etc/span/config.toml")),
            dirs::config_dir().map(|p| p.join("span/config.toml")),
        ]
        .into_iter()
        .flatten()
        .collect();

        for path in candidates {
            if path.exists() {
                let s = fs::read_to_string(&path)?;
                let file_cfg: Config = toml::from_str(&s)?;
                cfg = merge(cfg, file_cfg);
                break;
            }
        }

        // Env overrides
        if let Ok(v) = env::var("SPAN_HTTP_BIND") { cfg.http_bind = v; }
        if let Ok(v) = env::var("SPAN_GRPC_BIND") { cfg.grpc_bind = v; }
        if let Ok(v) = env::var("SPAN_NATS_URL") { cfg.nats_url = Some(v); }
        if let Ok(v) = env::var("SPAN_DATABASE_URL") { cfg.database_url = v; }

        if cfg.database_url.is_empty() {
            anyhow::bail!("DATABASE_URL or SPAN_DATABASE_URL must be set or provided in config");
        }

        Ok(cfg)
    }
}

fn merge(mut base: Config, other: Config) -> Config {
    if !other.database_url.is_empty() { base.database_url = other.database_url; }
    if other.http_bind != default_http_bind() { base.http_bind = other.http_bind; }
    if other.grpc_bind != default_grpc_bind() { base.grpc_bind = other.grpc_bind; }
    if other.nats_url.is_some() { base.nats_url = other.nats_url; }
    base
}
