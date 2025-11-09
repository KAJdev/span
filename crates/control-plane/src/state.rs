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
}

pub type SharedState = Arc<AppState>;
