use tonic::{Request, Response, Status};
use proto::agent::{
    agent_service_server::AgentService,
    NodeInfo, NodeCredentials, NodeStatus, NodeId, DesiredState, Container,
};
use crate::state::SharedState;
use sqlx::types::Json;
use uuid::Uuid;
#[cfg(feature = "grpc")]
use tonic::transport::server::TlsConnectInfo;

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
        sqlx::query!(
            "INSERT INTO nodes (id, name, region, labels, status) VALUES ($1, $2, $3, $4, 'registered')",
            node_id, info.name, info.region, Json(info.labels)
        )
        .execute(&self.state.db)
        .await
        .map_err(|e| Status::internal(format!("db error: {}", e)))?;

        let (cert_pem, key_pem) = crypto::generate_node_cert(&node_id.to_string(), &self.state.ca)
            .map_err(|e| Status::internal(format!("cert error: {}", e)))?;

        Ok(Response::new(NodeCredentials { node_id: node_id.to_string(), cert: cert_pem.into_bytes(), key: key_pem.into_bytes() }))
    }

    async fn heartbeat(&self, request: Request<NodeStatus>) -> Result<Response<prost_types::Empty>, Status> {
        let body = request.get_ref();
        let mut node_id = body.node_id.clone();
        let status = &body.status;

        if let Some(info) = request.extensions().get::<TlsConnectInfo>() {
            if let Some(first) = info.peer_certs().and_then(|v| v.first()).cloned() {
                if let Ok((_rem, parsed)) = x509_parser::parse_x509_certificate(first.as_ref()) {
                    if let Some(san) = parsed.subject_alternative_name() {
                        for gn in san.value.general_names.iter() {
                            if let x509_parser::extensions::GeneralName::DNSName(dns) = gn {
                                node_id = dns.to_string();
                                break;
                            }
                        }
                    }
                }
            }
        }

        let node_uuid = Uuid::parse_str(&node_id).map_err(|_| Status::unauthenticated("invalid node id"))?;

        sqlx::query!(
            "UPDATE nodes SET status = $1, heartbeat_at = NOW() WHERE id = $2",
            status,
            node_uuid
        )
        .execute(&self.state.db)
        .await
        .map_err(|e| Status::internal(format!("db error: {}", e)))?;

        if let Some(nc) = &self.state.nats {
            let subject = format!("span.nodes.{}.heartbeat", node_id);
            let _ = nc.publish(subject, Vec::new().into()).await;
        }

        Ok(Response::new(prost_types::Empty{}))
    }

    async fn get_desired_state(&self, _request: Request<NodeId>) -> Result<Response<DesiredState>, Status> {
        let state = DesiredState { containers: vec![ Container { id: "example".into(), image: "ghcr.io/example:latest".into(), env: Default::default() } ] };
        Ok(Response::new(state))
    }
}
