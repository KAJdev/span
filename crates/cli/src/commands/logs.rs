use anyhow::Result;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn stream_logs(app: &str, follow: bool, _tail: Option<usize>, cp_url: &str, token: Option<&str>) -> Result<()> {
    let parts: Vec<_> = app.split('/').collect();
    if parts.len() != 2 { return Err(anyhow::anyhow!("Invalid app format. Use: namespace/name")); }
    let (namespace, name) = (parts[0], parts[1]);

    let ws_base = if cp_url.starts_with("https://") { cp_url.replacen("https", "wss", 1) } else { cp_url.replacen("http", "ws", 1) };
    let mut ws_url = format!("{}/api/v1/apps/{}/{}/logs", ws_base.trim_end_matches('/'), namespace, name);
    if follow { ws_url.push_str("?follow=true"); }

    let (mut ws_stream, _) = connect_async(&ws_url).await?;

    // If token exists, try to send as first message (if server expects subprotocol/headers this may not work; using query param would be better if supported)
    if let Some(t) = token { let _ = ws_stream.send(Message::Text(format!("AUTH {}", t))).await; }

    println!("Connected to logs for {}/{}...", namespace, name);

    let (_, mut read) = ws_stream.split();
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(t) => println!("{}", t),
            Message::Binary(b) => println!("{}", String::from_utf8_lossy(&b)),
            Message::Close(_) => { println!("Connection closed"); break; },
            _ => {}
        }
        if !follow { break; }
    }
    Ok(())
}
