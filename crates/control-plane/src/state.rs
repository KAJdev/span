use std::sync::Arc;
use models::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub version: &'static str,
}

pub type SharedState = Arc<AppState>;
