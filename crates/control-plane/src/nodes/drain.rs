use anyhow::Result;
use models::PgPool;
use sqlx::Row;
use uuid::Uuid;

use crate::scheduler::schedule_app;

pub async fn drain_node(node_id: Uuid, db: PgPool) -> Result<()> {
    sqlx::query("UPDATE nodes SET cordoned = TRUE WHERE id = $1")
        .bind(node_id)
        .execute(&db)
        .await?;
    println!("Node {} cordoned", node_id);

    let deployments = sqlx::query("SELECT id, app_id, container_id FROM container_deployments WHERE node_id = $1")
        .bind(node_id)
        .fetch_all(&db)
        .await?;
    println!("Found {} deployments to evict", deployments.len());

    for row in deployments {
        let deployment_id: Uuid = row.get("id");
        let app_id: Option<Uuid> = row.get("app_id");
        let container_id: String = row.get("container_id");

        println!("Evicting container {} from node {}", container_id, node_id);

        sqlx::query("DELETE FROM container_deployments WHERE id = $1")
            .bind(deployment_id)
            .execute(&db)
            .await?;

        if let Some(app_id) = app_id {
            let _ = schedule_app(app_id, db.clone()).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    println!("Node {} drained successfully", node_id);
    Ok(())
}
