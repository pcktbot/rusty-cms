use anyhow::Context;
use cms_api::{
    catalog::ApiCatalog,
    config::AppConfig,
    routes::{AppState, build_router},
};
use cms_db::{pool, repositories::PgRepository};
use cms_pubsub::{MemoryPubSub, PubSub};
use cms_render::RenderEngine;
use cms_workflows::WorkflowRuntimeMatrix;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env().context("failed to load app configuration")?;
    let repository = match config.database_url.as_deref() {
        Some(database_url) => {
            let pool = pool::connect(database_url)
                .await
                .context("failed to connect to postgres")?;
            Some(PgRepository::new(pool))
        }
        None => None,
    };
    let pubsub = MemoryPubSub::default();
    let workflows = WorkflowRuntimeMatrix::default();
    let state = AppState {
        config: config.clone(),
        renderer: RenderEngine,
        workflows: workflows.clone(),
        catalog: Arc::new(ApiCatalog::default()),
        migrations: Arc::new(RwLock::new(HashMap::new())),
        repository: repository.clone(),
    };

    info!(
        supported_runtimes = ?workflows.supported_runtimes(),
        pubsub = pubsub.backend_name(),
        database_configured = config.database_url.is_some(),
        database_connected = repository.is_some(),
        database_required = config.require_database,
        temporal_ui_url = %config.temporal_ui_url,
        temporal_grpc_endpoint = %config.temporal_grpc_endpoint,
        "starting cms api"
    );

    let app = build_router(state);
    let addr = SocketAddr::from((config.bind_host, config.bind_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
