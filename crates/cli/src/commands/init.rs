use clap::Args;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Join an existing cluster by connecting to this node (ip or hostname)
    #[arg(long)]
    pub join: Option<String>,
    /// Force re-initialization
    #[arg(long)]
    pub force: bool,
    /// Optional install directory override (defaults to /opt/span)
    #[arg(long)]
    pub install_dir: Option<PathBuf>,
}

pub async fn run(args: InitArgs) -> anyhow::Result<()> {
    common::telemetry::init_tracing();

    let install_dir = args
        .install_dir
        .unwrap_or_else(|| PathBuf::from(std::env::var("INSTALL_DIR").unwrap_or_else(|_| "/opt/span".into())));
    let env_file = install_dir.join(".env");

    let mut cfg = load_dotenv(&env_file)?;

    if let Some(peer) = args.join {
        println!("Joining cluster via {}", peer);
        join_cluster(&peer, &mut cfg).await?;
    } else {
        println!("Bootstrapping new cluster");
        bootstrap_cluster(&mut cfg).await?;
    }

    save_dotenv(&env_file, &cfg)?;

    println!("✓ Initialization complete\n");
    println!("Start services with: systemctl start span");
    Ok(())
}

async fn bootstrap_cluster(cfg: &mut HashMap<String, String>) -> anyhow::Result<()> {
    cfg.insert("CLUSTER_MODE".into(), "standalone".into());
    cfg.insert("CLUSTER_JOIN".into(), String::new());
    cfg.insert("POSTGRES_REPLICATION_MODE".into(), "master".into());
    cfg.insert("NATS_ROUTES".into(), String::new());
    cfg.insert("MINIO_DISTRIBUTED_MODE_ENABLED".into(), "no".into());
    println!("✓ Cluster bootstrapped as first node");
    if let Ok(ip) = local_ip() { println!("  Other nodes can join using: span init --join {}", ip); }
    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
struct ClusterInfo {
    cluster_id: String,
    node_count: u32,
    peers: Vec<String>,
    jwt_secret: String,
    master_key: String,
}

async fn join_cluster(peer: &str, cfg: &mut HashMap<String, String>) -> anyhow::Result<()> {
    let url = format!("http://{}:8080/api/v1/cluster/info", peer);
    let info: ClusterInfo = reqwest::get(url).await?.json().await?;
    println!("✓ Connected to cluster: {} ({} nodes)", info.cluster_id, info.node_count);

    cfg.insert("CLUSTER_MODE".into(), "cluster".into());
    cfg.insert("CLUSTER_JOIN".into(), peer.into());
    cfg.insert("CLUSTER_PEERS".into(), info.peers.join(","));

    cfg.insert("POSTGRES_REPLICATION_MODE".into(), "replica".into());
    cfg.insert("POSTGRES_MASTER_HOST".into(), peer.into());

    let nats_routes = info
        .peers
        .iter()
        .map(|p| format!("nats://{}:6222", p))
        .collect::<Vec<_>>()
        .join(",");
    cfg.insert("NATS_ROUTES".into(), nats_routes);

    cfg.insert("MINIO_DISTRIBUTED_MODE_ENABLED".into(), "yes".into());
    cfg.insert("MINIO_DISTRIBUTED_NODES".into(), info.peers.join(","));

    cfg.insert("JWT_SECRET".into(), info.jwt_secret);
    cfg.insert("SPAN_MASTER_KEY".into(), info.master_key);

    println!("✓ Configured to join cluster");
    Ok(())
}

fn load_dotenv(path: &PathBuf) -> anyhow::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    if path.exists() {
        let s = fs::read_to_string(path)?;
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some((k, v)) = line.split_once('=') { map.insert(k.to_string(), v.to_string()); }
        }
    }
    Ok(map)
}

fn save_dotenv(path: &PathBuf, cfg: &HashMap<String, String>) -> anyhow::Result<()> {
    let mut lines = String::new();
    if path.exists() {
        let s = fs::read_to_string(path)?;
        for line in s.lines() {
            if line.trim().is_empty() || line.starts_with('#') { lines.push_str(line); lines.push('\n'); continue; }
            if let Some((k, _)) = line.split_once('=') {
                if cfg.contains_key(k) { continue; }
            }
            lines.push_str(line); lines.push('\n');
        }
    }
    for (k, v) in cfg.iter() { lines.push_str(&format!("{}={}\n", k, v)); }
    fs::write(path, lines)?;
    Ok(())
}

fn local_ip() -> anyhow::Result<String> {
    // Best-effort
    let output = std::process::Command::new("sh").arg("-c").arg("hostname -I | awk '{print $1}'").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
