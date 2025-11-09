use anyhow::Result;

pub async fn list(cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/v1/nodes", cp_url.trim_end_matches('/')));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let nodes: Vec<serde_json::Value> = req.send().await?.json().await.unwrap_or_default();
    println!("{:<20} {:<16} {:<10} {:<10}", "ID", "NAME", "STATUS", "CORDONED");
    for n in nodes {
        let id = n["id"].as_str().unwrap_or("?");
        let name = n["name"].as_str().unwrap_or("?");
        let status = n["status"].as_str().unwrap_or("?");
        let cordoned = n["cordoned"].as_bool().unwrap_or(false);
        println!("{:<20} {:<16} {:<10} {:<10}", id, name, status, cordoned);
    }
    Ok(())
}

pub async fn get(id: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/v1/nodes/{}", cp_url.trim_end_matches('/'), id));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await?;
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        eprintln!("Error: {}", resp.status());
    }
    Ok(())
}

pub async fn join(join_token: Option<&str>, _cp_url: &str, _token: Option<&str>) -> Result<()> {
    use dirs::home_dir;
    use std::fs;
    use std::path::PathBuf;
    use proto::agent::{agent_service_client::AgentServiceClient, NodeInfo};
    use tonic::transport::{ClientTlsConfig, Identity, Certificate as TlsCertificate, Channel};

    #[derive(serde::Deserialize)]
    struct JoinToken { control_plane_url: String, ca_cert: String }

    let base = home_dir().unwrap_or_default().join(".config/span-agent");
    fs::create_dir_all(&base)?;

    let token_str = join_token.ok_or_else(|| anyhow::anyhow!("--token is required"))?;
    let token_json = String::from_utf8(base64::decode(token_str)?)?;
    let jt: JoinToken = serde_json::from_str(&token_json)?;

    let ca_path = base.join("ca.crt");
    fs::write(&ca_path, jt.ca_cert.as_bytes())?;

    let cfg = serde_json::json!({
        "control_plane_url": jt.control_plane_url,
        "node_name": hostname::get().unwrap_or_default().to_string_lossy().to_string(),
        "region": null,
        "labels": {},
        "cert_path": base.join("node.crt"),
        "key_path": base.join("node.key"),
        "ca_cert_path": ca_path,
    });
    let cfg_toml = toml::to_string_pretty(&cfg)?;
    fs::write(base.join("config.toml"), cfg_toml)?;

    let ca_pem = fs::read(&ca_path)?;
    let tls = ClientTlsConfig::new().ca_certificate(TlsCertificate::from_pem(ca_pem));
    let channel = Channel::from_shared(jt.control_plane_url.clone())?.tls_config(tls)?.connect().await?;
    let mut client = AgentServiceClient::new(channel);

    // Generate WireGuard keys and persist so the agent reuses them
    let priv_path = base.join("wg.key");
    let pub_path = base.join("wg.pub");
    let wg_pubkey = if priv_path.exists() && pub_path.exists() {
        std::fs::read_to_string(&pub_path).unwrap_or_default().trim().to_string()
    } else {
        let private_key = wireguard_keys::Privkey::generate();
        let public_key = private_key.pubkey();
        std::fs::write(&priv_path, private_key.to_base64())?;
        std::fs::write(&pub_path, public_key.to_base64())?;
        public_key.to_base64()
    };

    let req = NodeInfo { name: hostname::get().unwrap_or_default().to_string_lossy().to_string(), region: String::new(), labels: Default::default(), wg_pubkey };
    let creds = client.register_node(req).await?.into_inner();
    fs::write(base.join("node.crt"), &creds.cert)?;
    fs::write(base.join("node.key"), &creds.key)?;
    fs::write(base.join("node_id"), creds.node_id.as_bytes())?;

    println!("Node registered successfully!");
    println!("Run: span-agent --config {}", base.join("config.toml").display());
    Ok(())
}

pub async fn cordon(id: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/v1/nodes/{}/cordon", cp_url.trim_end_matches('/'), id));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Cordoned {}", id); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn drain(id: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/v1/nodes/{}/drain", cp_url.trim_end_matches('/'), id));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Draining {}", id); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn uncordon(id: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/v1/nodes/{}/uncordon", cp_url.trim_end_matches('/'), id));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Uncordoned {}", id); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn remove(id: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.delete(format!("{}/api/v1/nodes/{}", cp_url.trim_end_matches('/'), id));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Removed node {}", id); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

fn local_ip() -> anyhow::Result<String> {
    let output = std::process::Command::new("sh").arg("-c").arg("hostname -I | awk '{print $1}'").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
