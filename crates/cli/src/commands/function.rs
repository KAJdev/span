use anyhow::Result;

pub async fn list(namespace: Option<&str>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let url = match namespace {
        Some(ns) => format!("{}/api/v1/namespaces/{}/functions", cp_url.trim_end_matches('/'), ns),
        None => format!("{}/api/v1/functions", cp_url.trim_end_matches('/')),
    };
    let client = reqwest::Client::new();
    let mut req = client.get(url);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let funcs: Vec<serde_json::Value> = req.send().await?.json().await.unwrap_or_default();
    println!("{:<24} {:<16}", "NAME", "NAMESPACE");
    for f in funcs {
        let name = f["name"].as_str().unwrap_or("?");
        let ns = f["namespace"].as_str().unwrap_or("default");
        println!("{:<24} {:<16}", name, ns);
    }
    Ok(())
}

pub async fn invoke(namespace: &str, name: &str, data: Option<&str>, file: Option<&str>, path: Option<&str>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let mut url = format!("{}/api/v1/functions/{}/{}/invoke", cp_url.trim_end_matches('/'), namespace, name);
    if let Some(p) = path { if !p.is_empty() { if !p.starts_with('/') { url.push('/'); } url.push_str(p.trim_start_matches('/')); } }

    let client = reqwest::Client::new();
    let mut req = client.post(url);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }

    let body: Option<serde_json::Value> = if let Some(d) = data { Some(serde_json::from_str(d).unwrap_or(serde_json::json!({"data": d}))) } else if let Some(f) = file { let s = std::fs::read_to_string(f)?; Some(serde_json::from_str(&s).unwrap_or(serde_json::json!({"data": s}))) } else { None };
    if let Some(b) = body { req = req.json(&b); }

    let resp = req.send().await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() { println!("{}", text); } else { eprintln!("Error: {} - {}", status, text); }
    Ok(())
}
