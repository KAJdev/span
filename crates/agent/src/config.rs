use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, fs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub control_plane_url: String,
    pub node_name: String,
    pub region: Option<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_cert_path: Option<PathBuf>,
}

impl AgentConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&data)?;
        Ok(cfg)
    }
}
