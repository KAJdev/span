use std::sync::Arc;
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde_json::json;
use uuid::Uuid;
use sqlx::Row;

use crate::{state::AppState, nodes::{drain::drain_node, remove::remove_node}};

pub async fn list_nodes(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, StatusCode> {
    let rows = sqlx::query("SELECT id, name, status, COALESCE(cordoned, FALSE) as cordoned FROM nodes ORDER BY created_at ASC")
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let nodes: Vec<serde_json::Value> = rows.into_iter().map(|row| {
        let id: Uuid = row.get("id");
        let name: String = row.get("name");
        let status: String = row.get("status");
        let cordoned: bool = row.get::<bool, _>("cordoned");
        json!({
            "id": id,
            "name": name,
            "status": status,
            "cordoned": cordoned,
        })
    }).collect();
    Ok(Json(json!(nodes)))
}

pub async fn get_node(Path(node_id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, StatusCode> {
    let row = sqlx::query("SELECT id, name, status, labels, heartbeat_at, created_at, COALESCE(cordoned, FALSE) as cordoned FROM nodes WHERE id = $1")
        .bind(node_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let id: Uuid = row.get("id");
    let name: String = row.get("name");
    let status: String = row.get("status");
    let labels: serde_json::Value = row.get("labels");
    let heartbeat_at: Option<chrono::DateTime<chrono::Utc>> = row.get("heartbeat_at");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let cordoned: bool = row.get::<bool, _>("cordoned");
    Ok(Json(json!({
        "id": id,
        "name": name,
        "status": status,
        "labels": labels,
        "heartbeat_at": heartbeat_at,
        "created_at": created_at,
        "cordoned": cordoned,
    })))
}

pub async fn cordon_node(Path(node_id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    sqlx::query("UPDATE nodes SET cordoned = TRUE WHERE id = $1")
        .bind(node_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    println!("Node {} cordoned", node_id);
    Ok(StatusCode::OK)
}

pub async fn uncordon_node(Path(node_id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    sqlx::query("UPDATE nodes SET cordoned = FALSE WHERE id = $1")
        .bind(node_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    println!("Node {} uncordoned", node_id);
    Ok(StatusCode::OK)
}

pub async fn drain_node_handler(Path(node_id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    let db = state.db.clone();
    tokio::spawn(async move {
        let _ = drain_node(node_id, db).await;
    });
    Ok(StatusCode::ACCEPTED)
}

pub async fn remove_node_handler(Path(node_id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    remove_node(node_id, state.db.clone()).await.map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(StatusCode::NO_CONTENT)
}
