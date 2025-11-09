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

pub async fn join(join_token: Option<&str>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/v1/cluster/join", cp_url.trim_end_matches('/')))
        .json(&serde_json::json!({
            "node_id": uuid::Uuid::new_v4().to_string(),
            "node_name": "node",
            "ip_address": local_ip().unwrap_or_else(|_| "127.0.0.1".to_string()),
            "token": join_token,
        }));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Join request sent"); } else { eprintln!("Error: {}", resp.status()); }
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
