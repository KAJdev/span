use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Deserialize, Serialize)]
struct AppSpec {
    #[serde(rename = "apiVersion")]
    api_version: Option<String>,
    kind: String,
    metadata: Metadata,
    #[serde(default)]
    spec: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
struct Metadata {
    name: String,
    #[serde(default = "default_namespace")] 
    namespace: String,
}

fn default_namespace() -> String { "default".into() }

fn client_with_token(token: Option<&str>) -> reqwest::Client {
    let builder = reqwest::Client::builder();
    let client = builder.build().expect("failed to build client");
    if token.is_some() { client } else { client }
}

pub async fn apply(file: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let content = fs::read_to_string(file)?;
    let app_spec: AppSpec = if file.ends_with(".yaml") || file.ends_with(".yml") {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    if app_spec.kind.to_lowercase() != "app" {
        return Err(anyhow::anyhow!("Invalid kind: expected App"));
    }

    let url = format!(
        "{}/api/v1/namespaces/{}/apps",
        cp_url.trim_end_matches('/'),
        app_spec.metadata.namespace
    );

    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .json(&serde_json::json!({
            "name": app_spec.metadata.name,
            "spec": app_spec.spec,
        }));

    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().await?;
    if resp.status().is_success() {
        println!("✓ App applied successfully");
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        eprintln!("Error: {} - {}", status, text);
    }
    Ok(())
}

pub async fn list(namespace: Option<&str>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let url = match namespace {
        Some(ns) => format!(
            "{}/api/v1/namespaces/{}/apps",
            cp_url.trim_end_matches('/'),
            ns
        ),
        None => format!("{}/api/v1/apps", cp_url.trim_end_matches('/')),
    };

    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let apps: Vec<serde_json::Value> = req.send().await?.json().await.unwrap_or_default();

    println!("{:<24} {:<16} {:<10}", "NAME", "NAMESPACE", "REPLICAS");
    for app in apps {
        let name = app["name"].as_str().unwrap_or("?");
        let namespace = app["namespace"].as_str().unwrap_or("default");
        let replicas = app["spec"]["run"]["replicas"].as_u64().unwrap_or_else(|| app["spec"]["replicas"].as_u64().unwrap_or(0));
        println!("{:<24} {:<16} {:<10}", name, namespace, replicas);
    }
    Ok(())
}

pub async fn get(namespace: &str, name: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let url = format!(
        "{}/api/v1/namespaces/{}/apps/{}",
        cp_url.trim_end_matches('/'),
        namespace,
        name
    );

    let client = reqwest::Client::new();
    let mut req = client.get(&url);
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

pub async fn delete(namespace: &str, name: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let url = format!(
        "{}/api/v1/namespaces/{}/apps/{}",
        cp_url.trim_end_matches('/'),
        namespace,
        name
    );

    let client = reqwest::Client::new();
    let mut req = client.delete(&url);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() {
        println!("✓ Deleted {}/{}", namespace, name);
    } else {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        eprintln!("Error: {} - {}", status, text);
    }
    Ok(())
}

pub async fn deploy(namespace: &str, name: &str, version: Option<u32>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let mut url = format!(
        "{}/api/v1/namespaces/{}/apps/{}/deploy",
        cp_url.trim_end_matches('/'),
        namespace,
        name
    );
    if let Some(v) = version { url.push_str(&format!("?version={}", v)); }

    let client = reqwest::Client::new();
    let mut req = client.post(&url);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Deployment triggered"); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn rollback(namespace: &str, name: &str, cp_url: &str, token: Option<&str>) -> Result<()> {
    let url = format!(
        "{}/api/v1/namespaces/{}/apps/{}/rollback",
        cp_url.trim_end_matches('/'),
        namespace,
        name
    );
    let client = reqwest::Client::new();
    let mut req = client.post(&url);
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Rolled back {}/{}", namespace, name); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}

pub async fn scale(namespace: &str, name: &str, replicas: u32, cp_url: &str, token: Option<&str>) -> Result<()> {
    let url = format!(
        "{}/api/v1/namespaces/{}/apps/{}/scale",
        cp_url.trim_end_matches('/'),
        namespace,
        name
    );
    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&serde_json::json!({"replicas": replicas}));
    if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
    let resp = req.send().await?;
    if resp.status().is_success() { println!("✓ Scaled to {}", replicas); } else { eprintln!("Error: {}", resp.status()); }
    Ok(())
}
