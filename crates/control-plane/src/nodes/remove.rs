use anyhow::{anyhow, Result};
use models::PgPool;
use sqlx::Row;
use uuid::Uuid;

pub async fn remove_node(node_id: Uuid, db: PgPool) -> Result<()> {
    let node_exists = sqlx::query("SELECT cordoned FROM nodes WHERE id = $1")
        .bind(node_id)
        .fetch_optional(&db)
        .await?;

    let Some(row) = node_exists else { return Err(anyhow!("Node not found")); };
    let cordoned: Option<bool> = row.get("cordoned");
    if cordoned.unwrap_or(false) == false {
        return Err(anyhow!("Node must be cordoned and drained before removal"));
    }

    let deployment_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM container_deployments WHERE node_id = $1")
        .bind(node_id)
        .fetch_one(&db)
        .await?;
    if deployment_count > 0 {
        return Err(anyhow!("Node still has active deployments. Drain first."));
    }

    sqlx::query("DELETE FROM service_endpoints WHERE node_id = $1")
        .bind(node_id)
        .execute(&db)
        .await?;

    sqlx::query("DELETE FROM nodes WHERE id = $1")
        .bind(node_id)
        .execute(&db)
        .await?;

    println!("Node {} removed", node_id);
    Ok(())
}
