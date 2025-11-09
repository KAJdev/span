pub mod schema;
pub mod namespace;
pub mod node;
pub mod app;
pub mod route;
pub mod build;
pub mod secret;
pub mod bucket;

use sqlx::{Pool, Postgres};

pub type PgPool = Pool<Postgres>;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
