mod config;
mod heartbeat;
mod wireguard;

use config::AgentConfig;
use dirs::home_dir;
use proto::agent::{agent_service_client::AgentServiceClient, NodeInfo, NodeId};
use std::{fs, path::PathBuf};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    common::telemetry::init_tracing();
    let default_cfg = home_dir().unwrap_or_default().join(".config/span-agent/config.toml");
    let cfg_path = std::env::args().nth(2).unwrap_or(default_cfg.to_string_lossy().to_string());
    info!(%cfg_path, "Starting span-agent");
    let cfg_path = PathBuf::from(cfg_path);

    let cfg = if cfg_path.exists() { AgentConfig::load(&cfg_path)? } else { prompt_minimal_config()? };

    // Ensure certs exist; if missing, register
    let have_creds = cfg.cert_path.exists() && cfg.key_path.exists();

    let ca_pem = cfg.ca_cert_path.as_ref().and_then(|p| fs::read(p).ok());

    // Ensure WireGuard keys exist, capture pubkey
    let base_dir = cfg.cert_path.parent().unwrap().to_path_buf();
    let wg_pubkey = wireguard::ensure_wg_keys(&base_dir)?;

    if !have_creds {
        let mut client = heartbeat::make_client_with_identity(&cfg.control_plane_url, ca_pem.clone(), None, None).await?;
        let req = NodeInfo { name: cfg.node_name.clone(), region: cfg.region.clone().unwrap_or_default(), labels: cfg.labels.clone(), wg_pubkey: wg_pubkey.clone() };
        let resp = client.register_node(req).await?.into_inner();
        fs::create_dir_all(&base_dir).ok();
        fs::write(&cfg.cert_path, &resp.cert)?;
        fs::write(&cfg.key_path, &resp.key)?;
        fs::write(base_dir.join("node_id"), resp.node_id.as_bytes()).ok();
        fs::write(base_dir.join("wg_ip"), resp.wg_ip.as_bytes()).ok();
        info!(node_id = %resp.node_id, wg_ip = %resp.wg_ip, "Registered and saved mTLS credentials");
    }

    // Now connect with identity and start heartbeat
    let cert = fs::read(&cfg.cert_path)?;
    let key = fs::read(&cfg.key_path)?;
    let mut client = heartbeat::make_client_with_identity(&cfg.control_plane_url, ca_pem.clone(), Some(cert), Some(key)).await?;

    let node_id_path = cfg.cert_path.parent().unwrap().join("node_id");
    let node_id = fs::read_to_string(node_id_path).unwrap_or_else(|_| "unknown".into());

    // Spawn WireGuard refresh loop
    let base_clone = base_dir.clone();
    let node_id_clone = node_id.clone();
    tokio::spawn(async move {
        wireguard::refresh_wireguard_loop(client.clone(), node_id_clone, base_clone).await
    });

    heartbeat::run_heartbeat(client, node_id).await;
    Ok(())
}

fn prompt_minimal_config() -> anyhow::Result<AgentConfig> {
    let base = home_dir().unwrap_or_default().join(".config/span-agent");
    fs::create_dir_all(&base).ok();
    let cfg = AgentConfig {
        control_plane_url: std::env::var("SPAN_CP_URL").unwrap_or_else(|_| "https://127.0.0.1:50051".into()),
        node_name: hostname::get().unwrap_or_default().to_string_lossy().to_string(),
        region: None,
        labels: Default::default(),
        cert_path: base.join("node.crt"),
        key_path: base.join("node.key"),
        ca_cert_path: Some(base.join("ca.crt")),
    };
    let cfg_text = toml::to_string_pretty(&cfg)?;
    fs::write(base.join("config.toml"), cfg_text)?;
    Ok(cfg)
}
