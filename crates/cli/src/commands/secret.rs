use anyhow::Result;
use std::io::{self, Write};

pub async fn set(namespace: &str, name: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    print!("Enter secret value: ");
    io::stdout().flush()?;
    let value = rpassword::read_password()?;

    let client = reqwest::Client::new();
    let mut req = client
        .post(format!(
            "{}/api/v1/namespaces/{}/secrets",
            cp_url.trim_end_matches('/'),
            namespace
        ))
        .json(&serde_json::json!({ "name": name, "value": value }));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Secret {} created", name); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn list(namespace: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.get(format!(
        "{}/api/v1/namespaces/{}/secrets",
        cp_url.trim_end_matches('/'),
        namespace
    ));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let secrets: Vec<serde_json::Value> = req.send().await?.json().await.unwrap_or_default();

    println!("{:<32} {:<8}", "NAME", "VERSION");
    for s in secrets {
        let name = s["name"].as_str().unwrap_or("?");
        let version = s["version"].as_u64().unwrap_or(0);
        println!("{:<32} {:<8}", name, version);
    }
    Ok(())
}
