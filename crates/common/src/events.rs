use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SpanEvent {
    NodeHeartbeat { node_id: String, status: String },
    BuildStarted { build_id: String, repo: String },
    BuildLog { build_id: String, line: String },
    BuildCompleted { build_id: String, status: String },
    DeploymentStarted { app_id: String, release_id: String },
    ContainerLog { container_id: String, line: String },
}

pub struct EventPublisher {
    pub client: async_nats::Client,
}

impl EventPublisher {
    pub async fn publish(&self, event: SpanEvent) -> anyhow::Result<()> {
        let subject = Self::subject_for_event(&event);
        match event {
            SpanEvent::BuildLog { line, .. } | SpanEvent::ContainerLog { line, .. } => {
                self.client.publish(subject, line.into()).await?;
            }
            other => {
                let payload = serde_json::to_vec(&other)?;
                self.client.publish(subject, payload.into()).await?;
            }
        }
        Ok(())
    }

    pub fn subject_for_event(event: &SpanEvent) -> String {
        match event {
            SpanEvent::NodeHeartbeat { node_id, .. } => format!("span.nodes.{node_id}.heartbeat"),
            SpanEvent::BuildStarted { build_id, .. } => format!("span.builds.{build_id}.status"),
            SpanEvent::BuildLog { build_id, .. } => format!("span.builds.{build_id}.logs"),
            SpanEvent::BuildCompleted { build_id, .. } => format!("span.builds.{build_id}.status"),
            SpanEvent::DeploymentStarted { release_id, .. } => format!("span.deploys.{release_id}.status"),
            // Using container_id directly for container logs; app-based logs use span.apps.{ns}.{name}.logs
            SpanEvent::ContainerLog { container_id, .. } => format!("span.containers.{container_id}.logs"),
        }
    }
}
