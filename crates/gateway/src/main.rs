use axum::{routing::get, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    common::telemetry::init_tracing();
    let bind = std::env::var("BIND_HTTP").unwrap_or_else(|_| "0.0.0.0:80".into());
    let addr: SocketAddr = bind.parse().expect("invalid BIND_HTTP");

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(|| async { "span-gateway" }));

    tracing::info!(%addr, "Gateway listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
