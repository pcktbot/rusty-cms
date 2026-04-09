use axum::{Json, Router, routing::get};
use cms_core::health::HealthStatus;
use cms_pubsub::{MemoryPubSub, PubSub};
use cms_workflows::WorkflowRuntimeMatrix;
use std::net::SocketAddr;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let pubsub = MemoryPubSub::default();
    let workflows = WorkflowRuntimeMatrix::default();

    info!(
        supported_runtimes = ?workflows.supported_runtimes(),
        pubsub = pubsub.backend_name(),
        "starting cms api"
    );

    let app = Router::new()
        .route("/health", get(health))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<HealthStatus> {
    Json(HealthStatus::ok("cms-api"))
}
