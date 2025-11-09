use models::PgPool;
use models::schema::Node;
use sqlx::Row;
use uuid::Uuid;

#[derive(Default)]
pub struct PlacementConstraints;

pub fn filter_eligible_nodes(all_nodes: &[Node], _constraints: &PlacementConstraints) -> Vec<Node> {
    all_nodes
        .iter()
        .filter(|n| n.status == "healthy" && n.cordoned.unwrap_or(false) == false)
        .cloned()
        .collect()
}

pub async fn schedule_app(app_id: Uuid, db: PgPool) -> anyhow::Result<()> {
    let rows = sqlx::query("SELECT id, name, status, COALESCE(cordoned, FALSE) as cordoned FROM nodes WHERE status = 'healthy' AND COALESCE(cordoned, FALSE) = FALSE ORDER BY created_at ASC")
        .fetch_all(&db)
        .await?;
    let target = rows.first().ok_or_else(|| anyhow::anyhow!("No eligible nodes"))?;
    let node_id: Uuid = target.get("id");

    let container_id = format!("app-{}-{}", app_id, uuid::Uuid::new_v4());
    sqlx::query("INSERT INTO container_deployments(app_id, node_id, container_id) VALUES ($1, $2, $3)")
        .bind(app_id)
        .bind(node_id)
        .bind(container_id)
        .execute(&db)
        .await?;

    println!("Scheduled app {} on node {}", app_id, node_id);
    Ok(())
}

