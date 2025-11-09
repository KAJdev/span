use tonic::{Request, Response, Status};
use proto::agent::{
    agent_service_server::AgentService,
    NodeInfo, NodeCredentials, NodeStatus, NodeId, DesiredState, Container, WireGuardConfig, Peer,
};
use crate::state::SharedState;
use uuid::Uuid;


#[derive(Clone)]
pub struct AgentSvc {
    pub state: SharedState,
}

impl AgentSvc {
    pub fn new(state: SharedState) -> Self { Self { state } }
}

#[tonic::async_trait]
impl AgentService for AgentSvc {
    async fn register_node(&self, request: Request<NodeInfo>) -> Result<Response<NodeCredentials>, Status> {
        let info = request.into_inner();
        let node_id = Uuid::new_v4();

        let last_ip: Option<String> = sqlx::query_scalar!(
            "SELECT MAX(wg_ip)::text FROM nodes WHERE wg_ip IS NOT NULL"
        )
        .fetch_optional(&self.state.db)
        .await
        .map_err(|e| Status::internal(format!("db error: {}", e)))?
        .flatten();

        let next_ip = if let Some(ip_txt) = last_ip {
            let ip: std::net::Ipv4Addr = ip_txt.parse().map_err(|_| Status::internal("invalid ip in db"))?;
            let octets = ip.octets();
            let num = (octets[0] as u32) << 24 | (octets[1] as u32) << 16 | (octets[2] as u32) << 8 | (octets[3] as u32);
            let next = num + 1;
            std::net::Ipv4Addr::new(((next >> 24) & 0xFF) as u8, ((next >> 16) & 0xFF) as u8, ((next >> 8) & 0xFF) as u8, (next & 0xFF) as u8)
        } else {
            // Start from 10.99.0.1
            std::net::Ipv4Addr::new(10, 99, 0, 1)
        };

        let wg_ip_txt = next_ip.to_string();

        let labels_json = serde_json::to_value(&info.labels).unwrap_or_else(|_| serde_json::json!({}));
        let wg_ip_net: sqlx::types::ipnetwork::IpNetwork = sqlx::types::ipnetwork::IpNetwork::new(std::net::IpAddr::V4(next_ip), 32).unwrap();
        sqlx::query!(
            "INSERT INTO nodes (id, name, region, labels, status, wg_pubkey, wg_ip) VALUES ($1, $2, $3, $4, 'registered', $5, $6)",
            node_id, info.name, info.region, labels_json, info.wg_pubkey, wg_ip_net
        )
        .execute(&self.state.db)
        .await
        .map_err(|e| Status::internal(format!("db error: {}", e)))?;

        let (cert_pem, key_pem) = crypto::generate_node_cert(&node_id.to_string(), &self.state.ca)
            .map_err(|e| Status::internal(format!("cert error: {}", e)))?;

        Ok(Response::new(NodeCredentials { node_id: node_id.to_string(), cert: cert_pem.into_bytes(), key: key_pem.into_bytes(), wg_ip: wg_ip_txt }))
    }

    async fn heartbeat(&self, request: Request<NodeStatus>) -> Result<Response<()>, Status> {
        let body = request.get_ref();
        let node_id = body.node_id.clone();
        let status = &body.status;

        let node_uuid = Uuid::parse_str(&node_id).map_err(|_| Status::unauthenticated("invalid node id"))?;

        sqlx::query!(
            "UPDATE nodes SET status = $1, heartbeat_at = NOW() WHERE id = $2",
            status,
            node_uuid
        )
        .execute(&self.state.db)
        .await
        .map_err(|e| Status::internal(format!("db error: {}", e)))?;

        if let Some(ep) = body.metadata.get("public_endpoint") {
            let _ = sqlx::query!(
                "UPDATE nodes SET public_endpoint = $1 WHERE id = $2",
                ep,
                node_uuid
            )
            .execute(&self.state.db)
            .await;
        }

        if let Some(nc) = &self.state.nats {
            let subject = format!("span.nodes.{}.heartbeat", node_id);
            let _ = nc.publish(subject, Vec::new().into()).await;
        }

        Ok(Response::new(()))
    }

    async fn get_desired_state(&self, _request: Request<NodeId>) -> Result<Response<DesiredState>, Status> {
        let state = DesiredState { containers: vec![ Container { id: "example".into(), image: "ghcr.io/example:latest".into(), env: Default::default() } ] };
        Ok(Response::new(state))
    }

    async fn get_wire_guard_config(&self, request: Request<NodeId>) -> Result<Response<WireGuardConfig>, Status> {
        let node_id = Uuid::parse_str(&request.get_ref().id).map_err(|_| Status::invalid_argument("invalid node id"))?;
        let row = sqlx::query!("SELECT wg_ip::text FROM nodes WHERE id = $1", node_id)
            .fetch_optional(&self.state.db)
            .await
            .map_err(|e| Status::internal(format!("db error: {}", e)))?;
        let wg_ip = row.and_then(|r| r.wg_ip).ok_or_else(|| Status::not_found("node wg_ip not set"))?;
        let address = format!("{}/16", wg_ip);

        let peers_rows = sqlx::query!(
            r#"SELECT wg_pubkey, wg_ip::text as wg_ip, public_endpoint FROM nodes 
                WHERE id != $1 AND status = 'healthy' AND wg_pubkey IS NOT NULL AND wg_ip IS NOT NULL"#,
            node_id
        )
        .fetch_all(&self.state.db)
        .await
        .map_err(|e| Status::internal(format!("db error: {}", e)))?;

        let mut peers = Vec::new();
        for r in peers_rows {
            let pubkey = r.wg_pubkey.unwrap_or_default();
            let peer_ip = r.wg_ip.unwrap_or_default();
            let endpoint = r.public_endpoint.unwrap_or_default();
            peers.push(Peer { public_key: pubkey, endpoint, allowed_ips: vec![format!("{}/32", peer_ip)] });
        }

        let cfg = WireGuardConfig { private_key: String::new(), address, peers };
        Ok(Response::new(cfg))
    }
}
