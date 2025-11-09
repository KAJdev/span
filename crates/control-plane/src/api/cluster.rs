use std::sync::Arc;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::state::AppState;

#[derive(Serialize)]
pub struct ClusterInfo {
    pub cluster_id: String,
    pub node_count: u32,
    pub peers: Vec<String>,
    pub jwt_secret: String,
    pub master_key: String,
}

pub async fn cluster_info(State(state): State<Arc<AppState>>) -> Json<ClusterInfo> {
    let nodes = sqlx::query("SELECT id, name, status FROM nodes WHERE status = 'healthy'")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let peers: Vec<String> = nodes.iter().map(|_n| String::new()).collect();

    Json(ClusterInfo {
        cluster_id: state.cluster_id.clone(),
        node_count: nodes.len() as u32,
        peers,
        jwt_secret: state.jwt_secret.clone(),
        master_key: std::env::var("SPAN_MASTER_KEY").unwrap_or_default(),
    })
}

#[derive(Deserialize)]
pub struct JoinRequest {
    pub node_id: String,
    pub node_name: String,
    pub ip_address: String,
}

#[derive(Serialize)]
pub struct JoinResponse {
    pub cluster_id: String,
    pub wg_config: String,
}

pub async fn join_cluster(State(state): State<Arc<AppState>>, Json(_req): Json<JoinRequest>) -> Json<JoinResponse> {
    let wg_config = "# wireguard config placeholder".to_string();
    Json(JoinResponse { cluster_id: state.cluster_id.clone(), wg_config })
}
