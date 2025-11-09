use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::Router;
use futures_util::StreamExt;
use testcontainers::{clients, core::WaitFor, GenericImage};
use tokio::net::TcpListener;

use control_plane::{api::routes::router, state::AppState};
use models::{create_pool, run_migrations};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_health_and_cluster_info() {
    common::telemetry::init_tracing();
    // Start Postgres
    let docker = clients::Cli::default();
    let image = GenericImage::new("postgres", "16")
        .with_env_var("POSTGRES_PASSWORD", "pass")
        .with_env_var("POSTGRES_DB", "span_test")
        .with_exposed_port(5432)
        .with_wait_for(WaitFor::message_on_stdout("database system is ready to accept connections"));
    let node = docker.run(image);
    let port = node.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:pass@127.0.0.1:{port}/span_test");

    // DB setup
    let pool = create_pool(&db_url).await.expect("create pool");
    run_migrations(&pool).await.expect("migrations");

    // App state
    std::env::set_var("SPAN_MASTER_KEY", "mk123");
    let state = Arc::new(AppState {
        db: pool,
        version: control_plane::VERSION,
        cluster_id: "cluster-abc".into(),
        jwt_secret: "jwt-xyz".into(),
        nats: None,
        log_hub: Arc::new(control_plane::events::logs::LogHub::new()),
    });

    // Server
    let app: Router = router(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });

    // HTTP client
    let base = format!("http://{addr}");
    let hc: serde_json::Value = reqwest::get(format!("{base}/health")).await.unwrap().json().await.unwrap();
    assert_eq!(hc["status"], "ok");
    assert_eq!(hc["version"], control_plane::VERSION);

    let ci: serde_json::Value = reqwest::get(format!("{base}/api/v1/cluster/info")).await.unwrap().json().await.unwrap();
    assert_eq!(ci["cluster_id"], "cluster-abc");
    assert_eq!(ci["node_count"], 0);
    assert_eq!(ci["jwt_secret"], "jwt-xyz");
    assert_eq!(ci["master_key"], "mk123");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_websocket_log_streaming_with_nats() {
    common::telemetry::init_tracing();
    // NATS
    let docker = clients::Cli::default();
    let image = GenericImage::new("nats", "2.10")
        .with_exposed_port(4222)
        .with_wait_for(WaitFor::message_on_stdout("Server is ready"));
    let node = docker.run(image);
    let port = node.get_host_port_ipv4(4222);
    let url = format!("nats://127.0.0.1:{port}");
    let client = tokio::time::timeout(Duration::from_secs(20), async_nats::connect(url)).await.expect("nats connect timeout").expect("connect nats");

    // Postgres (for state completeness, though not used here)
    let pg = GenericImage::new("postgres", "16")
        .with_env_var("POSTGRES_PASSWORD", "pass")
        .with_env_var("POSTGRES_DB", "span_test")
        .with_exposed_port(5432)
        .with_wait_for(WaitFor::message_on_stdout("database system is ready to accept connections"));
    let pg_node = docker.run(pg);
    let pg_port = pg_node.get_host_port_ipv4(5432);
    let db_url = format!("postgres://postgres:pass@127.0.0.1:{pg_port}/span_test");
    let pool = create_pool(&db_url).await.expect("create pool");
    run_migrations(&pool).await.expect("migrations");

    // Log hub subscribers
    let hub = Arc::new(control_plane::events::logs::LogHub::new());
    hub.clone().start_subscribers(client.clone()).await;

    // State and server
    let state = Arc::new(AppState {
        db: pool,
        version: control_plane::VERSION,
        cluster_id: "cluster-xyz".into(),
        jwt_secret: "jwt-123".into(),
        nats: Some(client.clone()),
        log_hub: hub.clone(),
    });
    let app: Router = router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });

    // Publish some logs before connecting
    let subject = "span.builds.e2e-build.logs".to_string();
    client.publish(subject.clone(), "before 1".into()).await.unwrap();
    client.publish(subject.clone(), "before 2".into()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    for _ in 0..50 {
        if hub.get_buffer(&subject).await.len() >= 2 { break; }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Connect WS to full API route
    let (mut ws_stream, _) = tokio::time::timeout(Duration::from_secs(10), tokio_tungstenite::connect_async(format!("ws://{}/api/v1/builds/e2e-build/logs", addr))).await.expect("ws connect timeout").unwrap();

    // Expect buffered
    let msg1 = tokio::time::timeout(Duration::from_secs(5), ws_stream.next()).await.expect("ws recv timeout").unwrap().unwrap();
    assert_eq!(msg1.to_text().unwrap(), "before 1");
    let msg2 = tokio::time::timeout(Duration::from_secs(5), ws_stream.next()).await.expect("ws recv timeout").unwrap().unwrap();
    assert_eq!(msg2.to_text().unwrap(), "before 2");

    // Then live
    client.publish(subject.clone(), "live 1".into()).await.unwrap();
    let msg3 = tokio::time::timeout(Duration::from_secs(5), ws_stream.next()).await.expect("ws recv timeout").unwrap().unwrap();
    assert_eq!(msg3.to_text().unwrap(), "live 1");
}
