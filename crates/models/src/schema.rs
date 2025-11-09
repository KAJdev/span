use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Namespace {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub name: String,
    pub wg_pubkey: Option<String>,
    pub region: Option<String>,
    pub labels: serde_json::Value,
    pub status: String,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct App {
    pub id: Uuid,
    pub namespace_id: Uuid,
    pub name: String,
    pub spec: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Release {
    pub id: Uuid,
    pub app_id: Uuid,
    pub version: i32,
    pub image_ref: String,
    pub build_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Route {
    pub id: Uuid,
    pub namespace_id: Uuid,
    pub host: String,
    pub path_prefix: String,
    pub backend_ref: String,
    pub tls_policy: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Build {
    pub id: Uuid,
    pub repo_url: String,
    pub commit: String,
    pub status: String,
    pub logs_ptr: Option<String>,
    pub image_ref: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Secret {
    pub id: Uuid,
    pub namespace_id: Uuid,
    pub name: String,
    pub version: i32,
    pub encrypted_value: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Bucket {
    pub id: Uuid,
    pub namespace_id: Uuid,
    pub name: String,
    pub policy: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Object {
    pub id: Uuid,
    pub bucket_id: Uuid,
    pub key: String,
    pub sha256: String,
    pub size: i64,
    pub content_type: Option<String>,
    pub created_at: DateTime<Utc>,
}
