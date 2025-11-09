use anyhow::Result;

pub async fn create(namespace: &str, name: &str, public: bool, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client
        .post(format!("{}/api/v1/namespaces/{}/buckets", cp_url.trim_end_matches('/'), namespace))
        .json(&serde_json::json!({
            "name": name,
            "policy": {"public": public}
        }));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Bucket {} created", name); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn upload(bucket: &str, key: &str, file: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut req = client
        .post(format!(
            "{}/api/v1/buckets/{}/presigned-upload",
            cp_url.trim_end_matches('/'),
            bucket
        ))
        .json(&serde_json::json!({ "key": key, "expires_in_seconds": 3600 }));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp: serde_json::Value = req.send().await?.json().await?;
    let upload_url = resp["url"].as_str().ok_or_else(|| anyhow::anyhow!("Missing upload URL"))?;

    let data = std::fs::read(file)?;
    let put = reqwest::Client::new().put(upload_url).body(data).send().await?;
    if put.status().is_success() {
        println!("✓ Uploaded {} to {}/{}", file, bucket, key);
    } else {
        eprintln!("Upload error: {}", put.status());
    }
    Ok(())
}

pub async fn list_objects(bucket: &str, prefix: Option<&str>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let mut url = format!("{}/api/v1/buckets/{}/objects", cp_url.trim_end_matches('/'), bucket);
    if let Some(p) = prefix { url.push_str(&format!("?prefix={}", p)); }

    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let objects: Vec<serde_json::Value> = req.send().await?.json().await.unwrap_or_default();

    println!("{:<48} {:<12} {:<25}", "KEY", "SIZE", "LAST MODIFIED");
    for o in objects {
        let key = o["key"].as_str().unwrap_or("?");
        let size = o["size"].as_u64().unwrap_or(0);
        let last = o["last_modified"].as_str().unwrap_or("?");
        println!("{:<48} {:<12} {:<25}", key, format_size(size), last);
    }
    Ok(())
}

pub async fn download(bucket: &str, key: &str, file: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    // Get presigned URL
    let client = reqwest::Client::new();
    let mut req = client
        .post(format!(
            "{}/api/v1/buckets/{}/presigned-download",
            cp_url.trim_end_matches('/'),
            bucket
        ))
        .json(&serde_json::json!({ "key": key, "expires_in_seconds": 3600 }));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp: serde_json::Value = req.send().await?.json().await?;
    let url = resp["url"].as_str().ok_or_else(|| anyhow::anyhow!("Missing download URL"))?;

    let bytes = reqwest::get(url).await?.bytes().await?;
    std::fs::write(file, &bytes)?;
    println!("✓ Downloaded {}/{} to {}", bucket, key, file);
    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.2} KB", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}
