use proto::agent::{agent_service_client::AgentServiceClient, NodeStatus};
use sysinfo::System;
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig, Certificate as TlsCertificate, Identity};

pub async fn run_heartbeat(mut client: AgentServiceClient<Channel>, node_id: String) {
    let mut sys = System::new_all();
    loop {
        sys.refresh_all();
        let cpu = (sys.global_cpu_info().cpu_usage() as u64).to_string();
        let mem = sys.used_memory().to_string();
        let total = sys.total_memory().to_string();
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("cpu".to_string(), cpu);
        metadata.insert("mem_used".to_string(), mem);
        metadata.insert("mem_total".to_string(), total);
        let status = NodeStatus { node_id: node_id.clone(), status: "healthy".into(), metadata };
        let _ = client.heartbeat(status).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

pub async fn make_client_with_identity(cp_url: &str, ca_pem: Option<Vec<u8>>, cert_pem: Option<Vec<u8>>, key_pem: Option<Vec<u8>>) -> anyhow::Result<AgentServiceClient<Channel>> {
    let mut tls = ClientTlsConfig::new();
    if let Some(ca) = ca_pem { tls = tls.ca_certificate(TlsCertificate::from_pem(ca)); }
    if let (Some(c), Some(k)) = (cert_pem, key_pem) {
        let ident = Identity::from_pem(c, k);
        tls = tls.identity(ident);
    }
    let channel = tonic::transport::Channel::from_shared(cp_url.to_string())?
        .tls_config(tls)?
        .connect()
        .await?;
    Ok(AgentServiceClient::new(channel))
}
