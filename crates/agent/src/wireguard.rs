use proto::agent::{agent_service_client::AgentServiceClient, NodeId, WireGuardConfig};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tonic::transport::Channel;
use std::{fs, path::PathBuf};

pub fn ensure_wg_keys(base_dir: &PathBuf) -> anyhow::Result<String> {
    let priv_path = base_dir.join("wg.key");
    let pub_path = base_dir.join("wg.pub");
    if !priv_path.exists() || !pub_path.exists() {
        let private_key = wireguard_keys::Privkey::generate();
        let public_key = private_key.pubkey();
        fs::write(&priv_path, private_key.to_base64())?;
        fs::write(&pub_path, public_key.to_base64())?;
    }
    let pubkey = fs::read_to_string(pub_path)?;
    Ok(pubkey.trim().to_string())
}

pub async fn configure_wireguard(cfg: &WireGuardConfig, private_key: &str) -> anyhow::Result<()> {
    // Create interface if missing
    let _ = Command::new("ip").args(["link", "add", "dev", "wg0", "type", "wireguard"]).output().await;

    // Set private key via stdin
    let mut child = Command::new("wg")
        .args(["set", "wg0", "private-key", "/dev/stdin"]) 
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(private_key.as_bytes()).await?;
    }
    let _ = child.wait().await;

    // Address
    let _ = Command::new("ip").args(["addr", "flush", "dev", "wg0"]).output().await;
    let _ = Command::new("ip").args(["addr", "add", &cfg.address, "dev", "wg0"]).output().await;

    // Clear peers
    let _ = Command::new("wg").args(["set", "wg0", "remove", "all-peers"]).output().await;

    for p in &cfg.peers {
        let allowed = if p.allowed_ips.is_empty() { String::new() } else { p.allowed_ips.join(",") };
        let _ = Command::new("wg").args(["set", "wg0", "peer", &p.public_key, "allowed-ips", &allowed]).output().await;
        if !p.endpoint.is_empty() {
            let _ = Command::new("wg").args(["set", "wg0", "peer", &p.public_key, "endpoint", &p.endpoint]).output().await;
        }
    }

    let _ = Command::new("ip").args(["link", "set", "wg0", "up"]).output().await;
    Ok(())
}

pub async fn refresh_wireguard_loop(mut client: AgentServiceClient<Channel>, node_id: String, base_dir: PathBuf) {
    let priv_key = match fs::read_to_string(base_dir.join("wg.key")) { Ok(v) => v, Err(_) => String::new() };
    loop {
        if let Ok(resp) = client.get_wire_guard_config(NodeId { id: node_id.clone() }).await {
            let cfg = resp.into_inner();
            let key_to_use = if !cfg.private_key.is_empty() { cfg.private_key.clone() } else { priv_key.clone() };
            let _ = configure_wireguard(&cfg, &key_to_use).await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

pub async fn test_mesh_connectivity(peer_ips: &[String]) -> Vec<String> {
    let mut reachable = Vec::new();
    for ip in peer_ips {
        let out = Command::new("ping").args(["-c1", "-W1", ip]).output().await;
        if let Ok(o) = out { if o.status.success() { reachable.push(ip.clone()); } }
    }
    reachable
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generates_keys() {
        let dir = tempdir().unwrap();
        let pubkey = ensure_wg_keys(&dir.path().to_path_buf()).unwrap();
        assert!(pubkey.len() > 40);
        assert!(dir.path().join("wg.key").exists());
        assert!(dir.path().join("wg.pub").exists());
    }
}
