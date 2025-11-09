pub mod api;
pub mod grpc;
pub mod scheduler;
pub mod state;
pub mod config;

use std::{net::SocketAddr, sync::Arc};
use axum::Router;
use models::{create_pool, run_migrations};
use proto::agent::agent_service_server::AgentServiceServer;
use tokio::net::TcpListener;
use tonic::transport::Server;
use tracing::info;

use crate::{api::routes::router, grpc::agent_service::AgentSvc, state::{AppState, SharedState}};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn start() -> anyhow::Result<()> {
    common::telemetry::init_tracing();
    info!(version = VERSION, "Control Plane starting...");

    let cfg = config::Config::load()?;

    let pool = create_pool(&cfg.database_url).await?;
    run_migrations(&pool).await?;
    info!("Connected to database and ran migrations");

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret".into());
    let cluster_id = std::env::var("SPAN_CLUSTER_ID").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

    let state: SharedState = Arc::new(AppState { db: pool, version: VERSION, cluster_id, jwt_secret });

    let http_addr: SocketAddr = cfg.http_bind.parse()?;
    let grpc_addr: SocketAddr = cfg.grpc_bind.parse()?;

    let http = run_http(http_addr, state.clone());
    let grpc = run_grpc(grpc_addr);
    let shutdown = shutdown_signal();

    tokio::select! {
        res = http => { res?; },
        res = grpc => { res?; },
        _ = shutdown => { info!("Shutdown signal received"); }
    }

    Ok(())
}

pub async fn run_http(addr: SocketAddr, state: SharedState) -> anyhow::Result<()> {
    let app: Router = router(state);
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "HTTP API listening");
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

pub async fn run_grpc(addr: SocketAddr) -> anyhow::Result<()> {
    let svc = AgentSvc::default();
    tracing::info!(%addr, "gRPC API listening");
    Server::builder()
        .add_service(AgentServiceServer::new(svc))
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;
    Ok(())
}

pub async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        let mut stream = signal::unix::signal(signal::unix::SignalKind::terminate()).expect("failed to install signal handler");
        stream.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {}, }
}
