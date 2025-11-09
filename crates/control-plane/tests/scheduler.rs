use control_plane::scheduler::{filter_eligible_nodes, PlacementConstraints};
use models::schema::Node;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

#[test]
fn filter_excludes_cordoned_nodes() {
    let now = Utc::now();
    let nodes = vec![
        Node { id: Uuid::new_v4(), name: "n1".into(), wg_pubkey: None, region: None, labels: json!({}), status: "healthy".into(), heartbeat_at: None, created_at: now, cordoned: Some(false) },
        Node { id: Uuid::new_v4(), name: "n2".into(), wg_pubkey: None, region: None, labels: json!({}), status: "healthy".into(), heartbeat_at: None, created_at: now, cordoned: Some(true) },
        Node { id: Uuid::new_v4(), name: "n3".into(), wg_pubkey: None, region: None, labels: json!({}), status: "pending".into(), heartbeat_at: None, created_at: now, cordoned: Some(false) },
    ];
    let eligible = filter_eligible_nodes(&nodes, &PlacementConstraints::default());
    assert_eq!(eligible.len(), 1);
    assert_eq!(eligible[0].name, "n1");
}
