use axum::{extract::State, Json};
use serde::Serialize;
use crate::state::SharedState;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

pub async fn get_health(State(state): State<SharedState>) -> Json<Health> {
    Json(Health { status: "ok", version: state.version })
}
