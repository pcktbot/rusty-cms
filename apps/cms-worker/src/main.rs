use cms_core::health::HealthStatus;
use cms_pubsub::{MemoryPubSub, PubSub};
use cms_render::RenderEngine;
use cms_workflows::WorkflowRuntimeMatrix;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let pubsub = MemoryPubSub::default();
    let workflows = WorkflowRuntimeMatrix::default();
    let renderer = RenderEngine::default();
    let health = HealthStatus::ok("cms-worker");

    info!(
        status = %health.status,
        renderer = renderer.name(),
        pubsub = pubsub.backend_name(),
        supported_runtimes = ?workflows.supported_runtimes(),
        "worker booted"
    );

    Ok(())
}
