use axum::{extract::State, Json};
use serde::Serialize;
use crate::state::SharedState;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
    pub version: &'static str,
}

pub async fn get_health(State(state): State<SharedState>) -> Json<Health> {
    Json(Health { status: "ok", version: state.version })
}
