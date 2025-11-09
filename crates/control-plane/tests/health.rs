use axum::{http::StatusCode, body::Body};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let pool = sqlx::PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
    let state = std::sync::Arc::new(control_plane::state::AppState { db: pool, version: control_plane::VERSION });
    let app = control_plane::api::routes::router(state);

    let resp = app.oneshot(axum::http::Request::builder().uri("/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
