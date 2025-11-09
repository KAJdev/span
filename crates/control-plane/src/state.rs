use std::sync::Arc;
use models::PgPool;
use crate::events::logs::LogHub;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub version: &'static str,
    pub cluster_id: String,
    pub jwt_secret: String,
    pub nats: Option<async_nats::Client>,
    pub log_hub: Arc<LogHub>,
    #[cfg(feature = "grpc")]
    pub ca_pem: String,
    #[cfg(feature = "grpc")]
    pub ca: Arc<rcgen::Certificate>,
}

pub type SharedState = Arc<AppState>;
