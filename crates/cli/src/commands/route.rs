use anyhow::Result;

pub async fn list(namespace: Option<&str>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let url = match namespace {
        Some(ns) => format!("{}/api/v1/namespaces/{}/routes", cp_url.trim_end_matches('/'), ns),
        None => format!("{}/api/v1/routes", cp_url.trim_end_matches('/')),
    };
    let client = reqwest::Client::new();
    let mut req = client.get(url);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let routes: Vec<serde_json::Value> = req.send().await?.json().await.unwrap_or_default();
    println!("{:<24} {:<16} {:<30}", "NAME", "NAMESPACE", "HOST");
    for r in routes {
        let name = r["name"].as_str().unwrap_or("?");
        let ns = r["namespace"].as_str().unwrap_or("default");
        let host = r["host"].as_str().unwrap_or("-");
        println!("{:<24} {:<16} {:<30}", name, ns, host);
    }
    Ok(())
}

pub async fn apply(file: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let content = std::fs::read_to_string(file)?;
    let doc: serde_json::Value = if file.ends_with(".yaml") || file.ends_with(".yml") { serde_yaml::from_str(&content)? } else { serde_json::from_str(&content)? };
    let kind = doc["kind"].as_str().unwrap_or("");
    if kind.to_lowercase() != "route" { return Err(anyhow::anyhow!("Invalid kind: expected Route")); }
    let namespace = doc["metadata"]["namespace"].as_str().unwrap_or("default");

    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/v1/namespaces/{}/routes", cp_url.trim_end_matches('/'), namespace)).json(&doc);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Route applied"); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn delete(namespace: &str, name: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client.delete(format!("{}/api/v1/namespaces/{}/routes/{}", cp_url.trim_end_matches('/'), namespace, name));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Route {}/{} deleted", namespace, name); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}
