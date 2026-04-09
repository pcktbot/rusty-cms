use crate::catalog::{ApiCatalog, BranchSummary, SiteSummary};
use crate::config::AppConfig;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use cms_core::health::HealthStatus;
use cms_core::site::SiteSnapshotRef;
use cms_core::widget::{WidgetCommand, WidgetDefinitionRef};
use cms_registry::importer::{ImportedWidgetPackage, WidgetImportError, WidgetSourceImporter};
use cms_render::RenderEngine;
use cms_workflows::{
    AgentRuntime, WorkflowArtifactContract, WorkflowKind, WorkflowRequest, WorkflowRuntimeMatrix,
    WorkflowSafetyPolicy,
};
use serde::{Deserialize, Serialize};
use std::{process::Stdio, sync::Arc};
use tokio::{io::AsyncWriteExt, process::Command};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub renderer: RenderEngine,
    pub workflows: WorkflowRuntimeMatrix,
    pub catalog: Arc<ApiCatalog>,
}

#[derive(Debug, Clone, Serialize)]
struct RuntimeInfoResponse {
    database_configured: bool,
    temporal_ui_url: String,
    temporal_grpc_endpoint: String,
}

#[derive(Debug, Clone, Serialize)]
struct BranchHeadResponse {
    site_id: Uuid,
    branch_name: String,
    snapshot_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WidgetCommandEnvelope {
    pub branch: String,
    pub base_snapshot_id: Uuid,
    pub command: WidgetCommand,
}

#[derive(Debug, Clone, Serialize)]
pub struct WidgetCommandReceipt {
    pub accepted: bool,
    pub site_id: Uuid,
    pub page_id: Uuid,
    pub branch: String,
    pub previous_snapshot_id: Uuid,
    pub new_snapshot_id: Uuid,
    pub command_type: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSubmissionReceipt {
    pub admitted: bool,
    pub workflow_id: Uuid,
    pub temporal_queue: String,
    pub temporal_ui_url: String,
    pub temporal_grpc_endpoint: String,
    pub site_id: Uuid,
    pub branch_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowTriggerReceipt {
    pub admitted: bool,
    pub started: bool,
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub temporal_queue: String,
    pub temporal_namespace: String,
    pub temporal_ui_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemporalStartResult {
    workflow_id: String,
    run_id: Option<String>,
    task_queue: String,
    namespace: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportLocalWidgetSourceRequest {
    pub path: String,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/runtime", get(runtime_info))
        .route("/api/sites", get(sites))
        .route("/api/sites/{site_id}/branches", get(site_branches))
        .route(
            "/api/sites/{site_id}/branches/{branch_name}/head",
            get(branch_head),
        )
        .route(
            "/api/sites/{site_id}/pages/{page_id}/widget-commands",
            post(submit_widget_command),
        )
        .route(
            "/api/sites/{site_id}/workflow-requests",
            post(submit_workflow_request),
        )
        .route(
            "/api/sites/{site_id}/workflow-requests/trigger",
            post(trigger_workflow_request),
        )
        .route("/api/widget-definitions", get(widget_definitions))
        .route(
            "/api/widget-definitions/{slug}",
            get(widget_definition_detail),
        )
        .route(
            "/api/widget-definitions/{slug}/versions",
            get(widget_definition_versions),
        )
        .route(
            "/api/widget-sources/import-local",
            post(import_local_widget_source),
        )
        .route("/api/workflows/definitions", get(workflow_definitions))
        .route("/api/demo/workflow-request", get(demo_workflow_request))
        .route("/api/demo/widget-command", get(demo_widget_command))
        .route("/preview/demo", get(preview_demo))
        .route("/viewer", get(viewer))
        .with_state(state)
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<HealthStatus> {
    Json(HealthStatus::ok("cms-api"))
}

async fn runtime_info(State(state): State<AppState>) -> Json<RuntimeInfoResponse> {
    Json(RuntimeInfoResponse {
        database_configured: state.config.database_url.is_some(),
        temporal_ui_url: state.config.temporal_ui_url.clone(),
        temporal_grpc_endpoint: state.config.temporal_grpc_endpoint.clone(),
    })
}

async fn sites(State(state): State<AppState>) -> Json<Vec<SiteSummary>> {
    Json(state.catalog.sites().to_vec())
}

async fn site_branches(
    Path(site_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BranchSummary>>, StatusCode> {
    if state.catalog.sites().iter().any(|site| site.id == site_id) {
        Ok(Json(state.catalog.branches_for_site(site_id).to_vec()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn branch_head(
    Path((site_id, branch_name)): Path<(Uuid, String)>,
    State(state): State<AppState>,
) -> Result<Json<BranchHeadResponse>, StatusCode> {
    let branch = state
        .catalog
        .branch_head(site_id, &branch_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(BranchHeadResponse {
        site_id,
        branch_name,
        snapshot_id: branch.head_snapshot_id,
    }))
}

async fn workflow_definitions(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    Json(
        state
            .workflows
            .definitions()
            .iter()
            .map(|definition| serde_json::json!(definition))
            .collect(),
    )
}

async fn submit_workflow_request(
    Path(site_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<WorkflowRequest>,
) -> Result<(StatusCode, Json<WorkflowSubmissionReceipt>), (StatusCode, Json<serde_json::Value>)> {
    if request.site_id != site_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "site_id in path does not match workflow request body"
            })),
        ));
    }

    let definition = state.workflows.admit(&request).map_err(|error| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": error.to_string()
            })),
        )
    })?;

    Ok((
        StatusCode::ACCEPTED,
        Json(WorkflowSubmissionReceipt {
            admitted: true,
            workflow_id: request.id,
            temporal_queue: definition.temporal_queue.clone(),
            temporal_ui_url: state.config.temporal_ui_url.clone(),
            temporal_grpc_endpoint: state.config.temporal_grpc_endpoint.clone(),
            site_id,
            branch_name: request.branch_name,
        }),
    ))
}

async fn trigger_workflow_request(
    Path(site_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<WorkflowRequest>,
) -> Result<(StatusCode, Json<WorkflowTriggerReceipt>), (StatusCode, Json<serde_json::Value>)> {
    if request.site_id != site_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "site_id in path does not match workflow request body"
            })),
        ));
    }

    let definition = state.workflows.admit(&request).map_err(|error| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": error.to_string()
            })),
        )
    })?;

    let result = start_temporal_workflow(&state.config, &request)
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": error
                })),
            )
        })?;

    Ok((
        StatusCode::ACCEPTED,
        Json(WorkflowTriggerReceipt {
            admitted: true,
            started: true,
            workflow_id: result.workflow_id,
            run_id: result.run_id,
            temporal_queue: definition.temporal_queue.clone(),
            temporal_namespace: result.namespace,
            temporal_ui_url: state.config.temporal_ui_url.clone(),
        }),
    ))
}

async fn widget_definitions(
    State(state): State<AppState>,
) -> Json<Vec<cms_core::widget::WidgetDefinition>> {
    Json(state.catalog.widget_definitions().to_vec())
}

async fn widget_definition_detail(
    Path(slug): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<cms_core::widget::WidgetDefinition>, StatusCode> {
    state
        .catalog
        .widget_definition(&slug)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn widget_definition_versions(
    Path(slug): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<cms_core::widget::WidgetDefinitionVersion>>, StatusCode> {
    if state.catalog.widget_definition(&slug).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(state.catalog.widget_versions(&slug).to_vec()))
}

async fn import_local_widget_source(
    Json(request): Json<ImportLocalWidgetSourceRequest>,
) -> Result<Json<ImportedWidgetPackage>, (StatusCode, Json<serde_json::Value>)> {
    WidgetSourceImporter::default()
        .import_from_path(&request.path)
        .map(Json)
        .map_err(import_error_response)
}

async fn submit_widget_command(
    Path((site_id, page_id)): Path<(Uuid, Uuid)>,
    State(_state): State<AppState>,
    Json(request): Json<WidgetCommandEnvelope>,
) -> Json<WidgetCommandReceipt> {
    Json(WidgetCommandReceipt {
        accepted: true,
        site_id,
        page_id,
        branch: request.branch,
        previous_snapshot_id: request.base_snapshot_id,
        new_snapshot_id: Uuid::new_v4(),
        command_type: command_type(&request.command),
    })
}

async fn demo_workflow_request(State(state): State<AppState>) -> Json<WorkflowRequest> {
    Json(sample_workflow_request(&state))
}

async fn demo_widget_command() -> Json<WidgetCommand> {
    Json(sample_widget_command())
}

async fn preview_demo(State(state): State<AppState>) -> Html<String> {
    let snapshot = SiteSnapshotRef::new(
        Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        "draft",
        Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
    );

    Html(
        state
            .renderer
            .render_preview_document(&snapshot)
            .unwrap_or_else(|error| format!("<pre>preview render failed: {error}</pre>")),
    )
}

async fn viewer(State(state): State<AppState>) -> Html<String> {
    let workflow_cards = state
        .workflows
        .definitions()
        .iter()
        .map(|definition| {
            format!(
                r#"<article class="workflow-card">
<div class="workflow-kind">{:?}</div>
<h3>{}</h3>
<p>Temporal queue: <code>{}</code></p>
<p>Allowed runtimes: <code>{}</code></p>
</article>"#,
                definition.kind,
                definition.name,
                definition.temporal_queue,
                definition
                    .accepted_runtimes
                    .iter()
                    .map(|runtime| format!("{runtime:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("");

    Html(format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>CMS Viewer</title>
    <style>
      :root {{
        --bg: #efe7d9;
        --panel: rgba(255, 252, 246, 0.92);
        --line: #d5c4ad;
        --ink: #221c14;
        --muted: #73675d;
        --accent: #8f3f24;
      }}
      * {{ box-sizing: border-box; }}
      body {{
        margin: 0;
        color: var(--ink);
        background:
          radial-gradient(circle at top right, rgba(143, 63, 36, 0.18), transparent 26%),
          linear-gradient(180deg, #f3ecdf, #e4d5bf);
        font-family: "Iowan Old Style", "Palatino Linotype", serif;
      }}
      .layout {{
        min-height: 100vh;
        display: grid;
        grid-template-columns: minmax(340px, 430px) minmax(0, 1fr);
      }}
      .sidebar {{
        padding: 28px 24px;
        border-right: 1px solid rgba(86, 60, 37, 0.12);
        backdrop-filter: blur(8px);
      }}
      .sidebar-panel {{
        background: var(--panel);
        border: 1px solid var(--line);
        border-radius: 22px;
        padding: 24px;
        box-shadow: 0 22px 60px rgba(49, 33, 19, 0.08);
      }}
      .eyebrow {{
        margin: 0 0 8px;
        color: var(--accent);
        font-size: 0.8rem;
        letter-spacing: 0.16em;
        text-transform: uppercase;
      }}
      h1 {{
        margin: 0 0 12px;
        font-size: 2.4rem;
        line-height: 0.95;
      }}
      p {{
        color: var(--muted);
        line-height: 1.6;
      }}
      .actions {{
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
        margin: 20px 0 22px;
      }}
      .btn {{
        display: inline-flex;
        align-items: center;
        justify-content: center;
        min-height: 42px;
        padding: 0 16px;
        border-radius: 999px;
        border: 1px solid var(--line);
        background: #fffaf2;
        color: var(--ink);
        font: inherit;
        text-decoration: none;
      }}
      .btn.primary {{
        background: var(--accent);
        color: #fff8f4;
        border-color: var(--accent);
      }}
      .workflow-list {{
        display: grid;
        gap: 12px;
        margin-top: 18px;
      }}
      .workflow-card {{
        padding: 16px;
        border: 1px solid var(--line);
        border-radius: 16px;
        background: rgba(255, 253, 247, 0.72);
      }}
      .workflow-card h3 {{
        margin: 4px 0 8px;
      }}
      .workflow-card p {{
        margin: 0 0 8px;
        font-size: 0.94rem;
      }}
      .workflow-kind {{
        font-size: 0.78rem;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: var(--accent);
      }}
      code {{
        font-family: "SFMono-Regular", ui-monospace, monospace;
        font-size: 0.88em;
      }}
      .frame-shell {{
        padding: 28px;
      }}
      .frame {{
        width: 100%;
        min-height: calc(100vh - 56px);
        border: 1px solid var(--line);
        border-radius: 24px;
        background: white;
        box-shadow: 0 24px 70px rgba(49, 33, 19, 0.14);
      }}
      @media (max-width: 980px) {{
        .layout {{
          grid-template-columns: 1fr;
        }}
        .frame {{
          min-height: 65vh;
        }}
      }}
    </style>
  </head>
  <body>
    <div class="layout">
      <aside class="sidebar">
        <section class="sidebar-panel">
          <div class="eyebrow">Temporary Viewer</div>
          <h1>Server render surface</h1>
          <p>
            This is the lightweight preview shell before the Bun and Svelte authoring
            UI exists. It now sits alongside early API routes for sites, branches,
            widget commands, workflow submission, and the widget registry.
          </p>
          <div class="actions">
            <a class="btn primary" href="/preview/demo" target="preview-frame">Load preview</a>
            <a class="btn" href="/api/sites" target="_blank">View sites</a>
            <a class="btn" href="/api/widget-definitions" target="_blank">Widget registry</a>
            <a class="btn" href="/api/runtime" target="_blank">Runtime config</a>
          </div>
          <div class="workflow-list">{workflow_cards}</div>
        </section>
      </aside>
      <main class="frame-shell">
        <iframe
          class="frame"
          title="preview"
          name="preview-frame"
          src="/preview/demo"
        ></iframe>
      </main>
    </div>
  </body>
</html>"#
    ))
}

fn sample_workflow_request(state: &AppState) -> WorkflowRequest {
    WorkflowRequest {
        id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        kind: WorkflowKind::AiContentOperation,
        site_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
        branch_name: "draft".to_owned(),
        requested_runtime: AgentRuntime::Python,
        temporal_queue: state
            .workflows
            .definition_for_kind(WorkflowKind::AiContentOperation)
            .expect("workflow definition should exist")
            .temporal_queue
            .clone(),
        input_payload: serde_json::json!({
            "instruction": "refresh the homepage hero and CTA copy",
            "component_id": "hero.v1"
        }),
        artifact_contract: WorkflowArtifactContract {
            output_schema: "schemas/ai-content-operation-output.json".to_owned(),
            creates_snapshot: true,
            mutates_branch_head: false,
        },
        safety_policy: WorkflowSafetyPolicy {
            requires_human_approval: true,
            max_sites_touched: 1,
            allow_publish_side_effects: false,
        },
    }
}

fn import_error_response(error: WidgetImportError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match error {
        WidgetImportError::PathMissing(_) | WidgetImportError::RequiredFileMissing(_) => {
            StatusCode::NOT_FOUND
        }
        WidgetImportError::ReadFailure { .. } | WidgetImportError::ParseFailure { .. } => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
    };

    (
        status,
        Json(serde_json::json!({
            "error": error.to_string()
        })),
    )
}

fn sample_widget_command() -> WidgetCommand {
    WidgetCommand::InsertWidget {
        region: "main".to_owned(),
        position: 0,
        definition: WidgetDefinitionRef {
            definition_id: Uuid::parse_str("66666666-6666-6666-6666-666666666666").unwrap(),
            version_id: Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap(),
            slug: "hero-banner".to_owned(),
            version: "3.4.1".to_owned(),
        },
        settings: serde_json::json!({
            "headline": "A better place to live",
            "cta_text": "Schedule a tour",
            "image_asset_id": "asset_123"
        }),
    }
}

fn command_type(command: &WidgetCommand) -> &'static str {
    match command {
        WidgetCommand::InsertWidget { .. } => "insert_widget",
        WidgetCommand::UpdateWidgetSettings { .. } => "update_widget_settings",
        WidgetCommand::MoveWidget { .. } => "move_widget",
        WidgetCommand::ReplaceWidget { .. } => "replace_widget",
        WidgetCommand::RemoveWidget { .. } => "remove_widget",
    }
}

async fn start_temporal_workflow(
    config: &AppConfig,
    request: &WorkflowRequest,
) -> Result<TemporalStartResult, String> {
    let mut child = Command::new(&config.temporal_runner_python)
        .arg(&config.temporal_runner_start_script)
        .env("TEMPORAL_GRPC_ENDPOINT", &config.temporal_grpc_endpoint)
        .env("TEMPORAL_NAMESPACE", &config.temporal_namespace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn temporal start script: {error}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::to_vec(request)
            .map_err(|error| format!("failed to serialize workflow request: {error}"))?;
        stdin.write_all(&payload).await.map_err(|error| {
            format!("failed to write workflow request to temporal script: {error}")
        })?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|error| format!("failed waiting for temporal start script: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("temporal start script failed: {}", stderr.trim()));
    }

    serde_json::from_slice::<TemporalStartResult>(&output.stdout)
        .map_err(|error| format!("failed to decode temporal start output: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn state() -> AppState {
        AppState {
            config: AppConfig::from_env(),
            renderer: RenderEngine,
            workflows: WorkflowRuntimeMatrix::default(),
            catalog: Arc::new(ApiCatalog::default()),
        }
    }

    #[tokio::test]
    async fn sites_route_returns_seed_data() {
        let response = build_router(state())
            .oneshot(
                Request::builder()
                    .uri("/api/sites")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn workflow_request_route_accepts_valid_payload() {
        let request = sample_workflow_request(&state());
        let response = build_router(state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/sites/{}/workflow-requests", request.site_id))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn widget_definition_versions_route_returns_not_found_for_unknown_slug() {
        let response = build_router(state())
            .oneshot(
                Request::builder()
                    .uri("/api/widget-definitions/does-not-exist/versions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn trigger_workflow_request_returns_bad_gateway_without_runner() {
        let mut test_state = state();
        test_state.config.temporal_runner_start_script = "/does/not/exist.py".to_owned();
        let request = sample_workflow_request(&test_state);

        let response = build_router(test_state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/sites/{}/workflow-requests/trigger",
                        request.site_id
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn import_local_widget_source_reads_fixture_repo() {
        let fixture_path = format!(
            "{}/../../crates/cms-registry/tests/fixtures/simple-widget",
            env!("CARGO_MANIFEST_DIR")
        );

        let response = build_router(state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/widget-sources/import-local")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({ "path": fixture_path })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["widget_slug"], "simple-hero");
        assert_eq!(value["runtime"], "Svelte");
    }
}
