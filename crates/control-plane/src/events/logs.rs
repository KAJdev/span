use std::{collections::{HashMap, VecDeque}, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use async_nats::Client;
use tracing::{info, warn};
use futures_util::StreamExt;

const LOG_BUFFER_CAP: usize = 1000;

#[derive(Clone, Default)]
pub struct LogHub {
    buffers: Arc<RwLock<HashMap<String, VecDeque<String>>>>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl LogHub {
    pub fn new() -> Self { Self::default() }

    pub async fn start_subscribers(self: Arc<Self>, client: Client) {
        // Establish subscriptions before returning to avoid race with early publishes
        let mut sub_apps = match client.subscribe("span.apps.*.*.logs").await {
            Ok(s) => { info!(subject = "span.apps.*.*.logs", "Subscribed to app logs"); s },
            Err(e) => { warn!(error = %e, "Failed to subscribe to app logs"); return; }
        };
        let apps = self.clone();
        tokio::spawn(async move {
            while let Some(msg) = sub_apps.next().await {
                let subject = msg.subject.clone();
                let line = match String::from_utf8(msg.payload.to_vec()) { Ok(s) => s, Err(_) => continue };
                apps.append_and_broadcast(&subject, line).await;
            }
        });

        let mut sub_builds = match client.subscribe("span.builds.*.logs").await {
            Ok(s) => { info!(subject = "span.builds.*.logs", "Subscribed to build logs"); s },
            Err(e) => { warn!(error = %e, "Failed to subscribe to build logs"); return; }
        };
        let builds = self.clone();
        tokio::spawn(async move {
            while let Some(msg) = sub_builds.next().await {
                let subject = msg.subject.clone();
                let line = match String::from_utf8(msg.payload.to_vec()) { Ok(s) => s, Err(_) => continue };
                builds.append_and_broadcast(&subject, line).await;
            }
        });
    }

    pub async fn get_buffer(&self, subject: &str) -> Vec<String> {
        let map = self.buffers.read().await;
        map.get(subject).map(|d| d.iter().cloned().collect()).unwrap_or_default()
    }

    pub async fn get_sender(&self, subject: &str) -> broadcast::Sender<String> {
        // Fast path read
        {
            let map = self.channels.read().await;
            if let Some(tx) = map.get(subject) { return tx.clone(); }
        }
        // Upgrade to write and insert
        let mut map = self.channels.write().await;
        map.entry(subject.to_string())
            .or_insert_with(|| broadcast::channel(1024).0)
            .clone()
    }

    async fn append_and_broadcast(&self, subject: &str, line: String) {
        {
            let mut map = self.buffers.write().await;
            let buf = map.entry(subject.to_string()).or_insert_with(VecDeque::new);
            if buf.len() >= LOG_BUFFER_CAP { buf.pop_front(); }
            buf.push_back(line.clone());
        }
        let tx = self.get_sender(subject).await;
        let _ = tx.send(line);
    }
}

use axum::{extract::{Path, State, ws::{WebSocketUpgrade, Message, WebSocket}}, response::IntoResponse};
use crate::state::SharedState;

pub async fn ws_app_logs(Path((namespace, name)): Path<(String, String)>, State(state): State<SharedState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    let subject = format!("span.apps.{namespace}.{name}.logs");
    ws.on_upgrade(move |socket| handle_ws(socket, state, subject))
}

pub async fn ws_build_logs(Path(build_id): Path<String>, State(state): State<SharedState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    let subject = format!("span.builds.{build_id}.logs");
    ws.on_upgrade(move |socket| handle_ws(socket, state, subject))
}

async fn handle_ws(mut socket: WebSocket, state: SharedState, subject: String) {
    // Send buffered lines first
    let buf = state.log_hub.get_buffer(&subject).await;
    for line in buf { let _ = socket.send(Message::Text(line)).await; }

    // Subscribe to live stream
    let tx = state.log_hub.get_sender(&subject).await;
    let mut rx = tx.subscribe();

    loop {
        tokio::select! {
            biased;
            // We only send server -> client; ignore client->server
            msg = rx.recv() => {
                match msg {
                    Ok(line) => { if socket.send(Message::Text(line)).await.is_err() { break; } }
                    Err(e) => { warn!(subject=%subject, error=%e, "broadcast receive error"); break; }
                }
            }
            _ = socket.recv() => { /* ignore client messages */ }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn append_and_buffer_works() {
        let hub = LogHub::new();
        let subject = "span.builds.test.logs";
        hub.append_and_broadcast(subject, "line1".into()).await;
        hub.append_and_broadcast(subject, "line2".into()).await;
        let buf = hub.get_buffer(subject).await;
        assert_eq!(buf, vec!["line1".to_string(), "line2".to_string()]);
    }

    #[tokio::test]
    async fn buffer_cap_is_enforced() {
        let hub = LogHub::new();
        let subject = "s";
        for i in 0..(LOG_BUFFER_CAP + 5) {
            hub.append_and_broadcast(subject, format!("l{i}")).await;
        }
        let buf = hub.get_buffer(subject).await;
        assert_eq!(buf.len(), LOG_BUFFER_CAP);
        assert_eq!(buf.first().unwrap(), "l5");
        assert_eq!(buf.last().unwrap(), &format!("l{}", LOG_BUFFER_CAP + 4));
    }

    #[tokio::test]
    async fn get_sender_is_cached_per_subject() {
        let hub = LogHub::new();
        let a1 = hub.get_sender("a").await;
        let a2 = hub.get_sender("a").await;
        assert!(a1.same_channel(&a2));
        let b = hub.get_sender("b").await;
        assert!(!a1.same_channel(&b));
    }
}
