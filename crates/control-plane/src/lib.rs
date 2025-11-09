pub mod api;
#[cfg(feature = "grpc")]
pub mod grpc;
pub mod scheduler;
pub mod nodes;
pub mod state;
pub mod config;
pub mod events;

use std::{net::SocketAddr, sync::Arc};
use axum::Router;
use models::{create_pool, run_migrations};
#[cfg(feature = "grpc")]
use proto::agent::agent_service_server::AgentServiceServer;
use tokio::net::TcpListener;
#[cfg(feature = "grpc")]
use tonic::transport::Server;
use tracing::{info, warn};

use crate::{api::routes::router, state::{AppState, SharedState}};
#[cfg(feature = "grpc")]
use crate::grpc::agent_service::AgentSvc;
#[cfg(feature = "grpc")]
use tonic::transport::{ServerTlsConfig, Identity, Certificate as TlsCertificate};

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

    // NATS (optional for MVP)
    let (nats, log_hub) = if let Some(url) = cfg.nats_url.clone() {
        match async_nats::connect(url.clone()).await {
            Ok(client) => {
                info!(%url, "Connected to NATS");
                let hub = Arc::new(events::logs::LogHub::new());
                hub.clone().start_subscribers(client.clone()).await;
                (Some(client), hub)
            }
            Err(e) => {
                warn!(error=%e, "Failed to connect to NATS; continuing without event bus");
                (None, Arc::new(events::logs::LogHub::new()))
            }
        }
    } else {
        warn!("NATS URL not configured; event bus disabled");
        (None, Arc::new(events::logs::LogHub::new()))
    };

    #[cfg(feature = "grpc")]
    let ca_material = crypto::load_or_init_ca(None)?;
    #[cfg(feature = "grpc")]
    let state: SharedState = Arc::new(AppState { db: pool, version: VERSION, cluster_id, jwt_secret, nats, log_hub, ca_pem: ca_material.ca_cert_pem.clone(), ca: Arc::new(ca_material.ca) });
    #[cfg(not(feature = "grpc"))]
    let state: SharedState = Arc::new(AppState { db: pool, version: VERSION, cluster_id, jwt_secret, nats, log_hub });

    let http_addr: SocketAddr = cfg.http_bind.parse()?;
    let grpc_addr: SocketAddr = cfg.grpc_bind.parse()?;

    let http = run_http(http_addr, state.clone());
    #[cfg(feature = "grpc")]
    let grpc = run_grpc(grpc_addr, state.clone());
    // Start node health monitor
    #[cfg(feature = "grpc")]
    let monitor = monitor_node_health(state.clone());
    let shutdown = shutdown_signal();

    #[cfg(feature = "grpc")]
    tokio::select! {
        res = http => { res?; },
        res = grpc => { res?; },
        _ = monitor => { info!("Health monitor exited"); },
        _ = shutdown => { info!("Shutdown signal received"); }
    }

    #[cfg(not(feature = "grpc"))]
    tokio::select! {
        res = http => { res?; },
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

#[cfg(feature = "grpc")]
pub async fn run_grpc(addr: SocketAddr, state: SharedState) -> anyhow::Result<()> {
    // Build server identity signed by CA for TLS
    let (server_cert_pem, server_key_pem) = crypto::generate_node_cert("control-plane", &state.ca)?;
    let identity = Identity::from_pem(server_cert_pem.clone(), server_key_pem.clone());
    let _client_ca = TlsCertificate::from_pem(state.ca_pem.clone());

    // Use optional client auth: allow both anonymous and authenticated clients.
    let tls = ServerTlsConfig::new().identity(identity);

    let svc = AgentSvc::new(state.clone());
    tracing::info!(%addr, "gRPC API listening (TLS enabled)");
    Server::builder()
        .tls_config(tls)?
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

#[cfg(feature = "grpc")]
pub async fn monitor_node_health(state: SharedState) {
    use std::time::Duration;
    loop {
        let _ = sqlx::query!(
            "UPDATE nodes SET status = 'unreachable' WHERE heartbeat_at < NOW() - INTERVAL '2 minutes' AND status != 'unreachable'"
        )
        .execute(&state.db)
        .await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
