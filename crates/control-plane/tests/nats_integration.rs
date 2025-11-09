use std::{net::SocketAddr, sync::Arc, time::Duration};

use async_nats::Client;
use axum::{routing::get, Router};
use futures_util::StreamExt;
use testcontainers::{clients, core::WaitFor, GenericImage};
use tokio::net::TcpListener;

use control_plane::events::logs::LogHub;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pubsub_delivery_and_buffer() {
    common::telemetry::init_tracing();
    let docker = clients::Cli::default();
    let image = GenericImage::new("nats", "2.10").with_exposed_port(4222).with_wait_for(WaitFor::message_on_stdout("Server is ready"));
    let node = docker.run(image);

    let port = node.get_host_port_ipv4(4222);
    let url = format!("nats://127.0.0.1:{port}");
    let client = tokio::time::timeout(Duration::from_secs(20), async_nats::connect(url)).await.expect("nats connect timeout").expect("connect nats");

    let hub = Arc::new(LogHub::new());
    hub.clone().start_subscribers(client.clone()).await;

    let subject = "span.builds.build-123.logs";

    client.publish(subject.to_string(), "line 1".into()).await.unwrap();
    client.publish(subject.to_string(), "line 2".into()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    for _ in 0..50 {
        if hub.get_buffer(subject).await.len() >= 2 { break; }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let buf = hub.get_buffer(subject).await;
    assert_eq!(buf, vec!["line 1".to_string(), "line 2".to_string()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_streams_buffer_then_live() {
    common::telemetry::init_tracing();
    let docker = clients::Cli::default();
    let image = GenericImage::new("nats", "2.10").with_exposed_port(4222).with_wait_for(WaitFor::message_on_stdout("Server is ready"));
    let node = docker.run(image);

    let port = node.get_host_port_ipv4(4222);
    let url = format!("nats://127.0.0.1:{port}");
    let client = tokio::time::timeout(Duration::from_secs(20), async_nats::connect(url)).await.expect("nats connect timeout").expect("connect nats");

    let hub = Arc::new(LogHub::new());
    hub.clone().start_subscribers(client.clone()).await;

    let subject = "span.builds.build-xyz.logs".to_string();

    client.publish(subject.clone(), "before 1".into()).await.unwrap();
    client.publish(subject.clone(), "before 2".into()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    for _ in 0..50 {
        if hub.get_buffer(&subject).await.len() >= 2 { break; }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Minimal WS server using the same logic as handlers
    let hub_clone = hub.clone();
    let subject_clone = subject.clone();
    let app = Router::new().route("/ws", get(move |ws: axum::extract::ws::WebSocketUpgrade| {
        let hub = hub_clone.clone();
        let subject = subject_clone.clone();
        async move {
            ws.on_upgrade(move |mut socket| async move {
                let buf = hub.get_buffer(&subject).await;
                for line in buf { let _ = socket.send(axum::extract::ws::Message::Text(line)).await; }
                let tx = hub.get_sender(&subject).await;
                let mut rx = tx.subscribe();
                while let Ok(line) = rx.recv().await {
                    if socket.send(axum::extract::ws::Message::Text(line)).await.is_err() { break; }
                }
            })
        }
    }));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });

    // Connect client
    let (mut ws_stream, _) = tokio::time::timeout(Duration::from_secs(10), tokio_tungstenite::connect_async(format!("ws://{}/ws", addr))).await.expect("ws connect timeout").unwrap();

    let msg1 = tokio::time::timeout(Duration::from_secs(5), ws_stream.next()).await.expect("ws recv timeout").unwrap().unwrap();
    assert_eq!(msg1.to_text().unwrap(), "before 1");
    let msg2 = tokio::time::timeout(Duration::from_secs(5), ws_stream.next()).await.expect("ws recv timeout").unwrap().unwrap();
    assert_eq!(msg2.to_text().unwrap(), "before 2");

    // Publish live and expect to receive
    client.publish(subject.clone(), "live 1".into()).await.unwrap();
    let msg3 = tokio::time::timeout(Duration::from_secs(5), ws_stream.next()).await.expect("ws recv timeout").unwrap().unwrap();
    assert_eq!(msg3.to_text().unwrap(), "live 1");
}
