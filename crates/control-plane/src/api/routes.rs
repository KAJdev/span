use axum::{routing::get, Router, Json};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use serde_json::json;
use crate::state::SharedState;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(super::health::get_health))
        .route("/api/v1/nodes", get(|| async { Json(json!([])) }))
        .route("/api/v1/apps", get(|| async { Json(json!([])) }))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
