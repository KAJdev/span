pub fn run() -> anyhow::Result<()> {
    println!("node command stub");
    Ok(())
}

pub async fn status() -> anyhow::Result<()> {
    let cp = std::env::var("CONTROL_PLANE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let url = format!("{}/health", cp.trim_end_matches('/'));
    let resp = reqwest::get(url).await;
    match resp {
        Ok(r) => {
            if r.status().is_success() {
                let text = r.text().await.unwrap_or_default();
                println!("✓ Control Plane healthy: {}", text);
            } else {
                println!("Control Plane not healthy: {}", r.status());
            }
        }
        Err(e) => println!("Failed to reach Control Plane: {}", e),
    }
    Ok(())
}
