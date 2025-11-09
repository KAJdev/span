use axum::{routing::{get, post, delete}, Router, Json};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use serde_json::json;
use crate::state::SharedState;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(super::health::get_health))
        .route("/api/v1/nodes", get(super::nodes::list_nodes))
        .route("/api/v1/nodes/:id", get(super::nodes::get_node).delete(super::nodes::remove_node_handler))
        .route("/api/v1/nodes/:id/cordon", post(super::nodes::cordon_node))
        .route("/api/v1/nodes/:id/uncordon", post(super::nodes::uncordon_node))
        .route("/api/v1/nodes/:id/drain", post(super::nodes::drain_node_handler))
        .route("/api/v1/apps", get(|| async { Json(json!([])) }))
        .route("/api/v1/cluster/info", get(super::cluster::cluster_info))
        .route("/api/v1/cluster/join", post(super::cluster::join_cluster))
        // Log streaming (WebSocket)
        .route("/api/v1/apps/:namespace/:name/logs", get(crate::events::logs::ws_app_logs))
        .route("/api/v1/builds/:id/logs", get(crate::events::logs::ws_build_logs))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
