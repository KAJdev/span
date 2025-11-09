use tonic::{Request, Response, Status};
use proto::agent::{
    agent_service_server::AgentService,
    NodeInfo, NodeCredentials, NodeStatus, NodeId, DesiredState, Container,
};

#[derive(Default, Clone)]
pub struct AgentSvc;

#[tonic::async_trait]
impl AgentService for AgentSvc {
    async fn register_node(&self, _request: Request<NodeInfo>) -> Result<Response<NodeCredentials>, Status> {
        let creds = NodeCredentials { node_id: "mock".into(), cert: vec![], key: vec![] };
        Ok(Response::new(creds))
    }

    async fn heartbeat(&self, _request: Request<NodeStatus>) -> Result<Response<prost_types::Empty>, Status> {
        Ok(Response::new(prost_types::Empty{}))
    }

    async fn get_desired_state(&self, _request: Request<NodeId>) -> Result<Response<DesiredState>, Status> {
        let state = DesiredState { containers: vec![ Container { id: "example".into(), image: "ghcr.io/example:latest".into(), env: Default::default() } ] };
        Ok(Response::new(state))
    }
}
