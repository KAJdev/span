use std::time::Duration;

fn main() {
    common::telemetry::init_tracing();
    println!("span-agent stub");
}

#[allow(dead_code)]
pub async fn stop_container_gracefully(container_id: &str, timeout: Duration) -> anyhow::Result<()> {
    send_signal(container_id, Signal::SigTerm).await?;
    for _ in 0..timeout.as_secs() {
        let status = get_container_status(container_id).await?;
        if status.state == "exited" {
            return Ok(())
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    send_signal(container_id, Signal::SigKill).await?;
    Ok(())
}

#[derive(Clone, Copy)]
enum Signal { SigTerm, SigKill }

async fn send_signal(_container_id: &str, _sig: Signal) -> anyhow::Result<()> { Ok(()) }

struct ContainerStatus { state: String }

async fn get_container_status(_container_id: &str) -> anyhow::Result<ContainerStatus> {
    Ok(ContainerStatus { state: "exited".to_string() })
}
