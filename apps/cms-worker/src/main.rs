use anyhow::{Context, bail};
use cms_core::health::HealthStatus;
use cms_db::{migrate, pool};
use cms_pubsub::{MemoryPubSub, PubSub};
use cms_render::RenderEngine;
use cms_workflows::WorkflowRuntimeMatrix;
use std::env;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let _ = dotenvy::dotenv();
    let command = env::args().nth(1);
    if let Some(command) = command.as_deref() {
        match command {
            "migrate" => return run_migrations().await,
            other => bail!("unknown cms-worker command: {other}"),
        }
    }

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

async fn run_migrations() -> anyhow::Result<()> {
    let database_url = env::var("DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .context("DATABASE_URL must be set to run migrations")?;

    let pool = pool::connect(&database_url)
        .await
        .context("failed to connect to postgres for migrations")?;
    migrate::run_migrations(&pool)
        .await
        .context("failed to apply database migrations")?;

    info!("database migrations applied");
    Ok(())
}
