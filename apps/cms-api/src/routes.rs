use crate::catalog::{ApiCatalog, BranchSummary, SiteSummary};
use crate::config::AppConfig;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use cms_core::draft::{
    DraftChangeKind, DraftChangeSetSourceKind, DraftChangeSetStatus, DraftChangeStatus,
    DraftPreviewRef, DraftResourceKind, ImportedPageDraft,
};
use cms_core::health::HealthStatus;
use cms_core::page::{PageDocument, PageSeo};
use cms_core::site::{SiteKind, SiteSnapshotRef};
use cms_core::widget::{
    HtmlSupportMode, WidgetCommand, WidgetDefinition, WidgetDefinitionRef, WidgetDefinitionVersion,
    WidgetRuntime, WidgetSourceKind,
};
use cms_db::{
    models::{
        AccountRow, BranchRow, DraftChangeRow, DraftChangeSetRow, DraftPageDocumentRow,
        MigrationJobRow, MigrationPageArtifactRow, MigrationPageRow, OutboxEventRow, SiteRow,
        WorkflowRequestRow,
    },
    repositories::PgRepository,
};
use cms_registry::importer::{ImportedWidgetPackage, WidgetImportError, WidgetSourceImporter};
use cms_render::RenderEngine;
use cms_workflows::{
    AgentRuntime, WorkflowArtifactContract, WorkflowKind, WorkflowRequest, WorkflowRuntimeMatrix,
    WorkflowSafetyPolicy,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Stdio, sync::Arc};
use time::OffsetDateTime;
use tokio::{io::AsyncWriteExt, process::Command, sync::RwLock};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::error;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub renderer: RenderEngine,
    pub workflows: WorkflowRuntimeMatrix,
    pub catalog: Arc<ApiCatalog>,
    pub migrations: Arc<RwLock<HashMap<Uuid, MigrationJobRecord>>>,
    pub repository: Option<PgRepository>,
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
    snapshot_id: Option<Uuid>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemporalWorkflowResult {
    workflow_id: String,
    namespace: String,
    result: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportLocalWidgetSourceRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Queued,
    Running,
    ReviewReady,
    Approved,
    Imported,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationCrawlScope {
    HomepageOnly,
    Subpath,
    Site,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationOptions {
    pub crawl_scope: MigrationCrawlScope,
    pub follow_subdomains: bool,
    pub max_pages: u32,
    pub respect_robots: bool,
    pub include_assets: bool,
    pub detect_registered_widgets: bool,
    pub use_legacy_api_enrichment: bool,
    pub screenshot_compare_after_import: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMigrationRequest {
    pub target_site_id: Option<Uuid>,
    pub create_site: Option<CreateSiteTarget>,
    pub homepage_url: String,
    pub client_id: Uuid,
    pub location_id: Uuid,
    pub legacy_api_profile: Option<String>,
    pub options: MigrationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSiteTarget {
    pub account_id: Option<Uuid>,
    pub account_name: Option<String>,
    pub account_slug: Option<String>,
    pub name: String,
    pub slug: String,
    pub primary_host: String,
    pub site_kind: SiteKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPageSummary {
    pub id: Uuid,
    pub path: String,
    pub title_guess: String,
    pub widget_matches: Vec<String>,
    pub unknown_regions: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPageDetail {
    pub id: Uuid,
    pub path: String,
    pub title_guess: String,
    pub widget_matches: Vec<String>,
    pub unknown_regions: u32,
    pub confidence: f32,
    pub warnings: Vec<String>,
    pub extraction_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPageArtifactDetail {
    pub page_id: Uuid,
    pub source_url: String,
    pub http_status: Option<u16>,
    pub final_url: Option<String>,
    pub artifact: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationJobRecord {
    pub id: Uuid,
    pub site_id: Uuid,
    pub workflow_request_id: Uuid,
    pub workflow_id: String,
    pub branch_name: String,
    pub homepage_url: String,
    pub client_id: Uuid,
    pub location_id: Uuid,
    pub legacy_api_profile: Option<String>,
    pub status: MigrationStatus,
    pub options: MigrationOptions,
    pub pages: Vec<MigrationPageDetail>,
    pub warnings: Vec<String>,
    pub created_at: OffsetDateTime,
    pub approved_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationJobSummary {
    pub id: Uuid,
    pub site_id: Uuid,
    pub workflow_request_id: Uuid,
    pub workflow_id: String,
    pub branch_name: String,
    pub homepage_url: String,
    pub client_id: Uuid,
    pub location_id: Uuid,
    pub legacy_api_profile: Option<String>,
    pub status: MigrationStatus,
    pub options: MigrationOptions,
    pub page_count: usize,
    pub warnings: Vec<String>,
    pub created_at: OffsetDateTime,
    pub approved_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateMigrationReceipt {
    pub accepted: bool,
    pub migration_id: Uuid,
    pub workflow_request_id: Uuid,
    pub workflow_id: String,
    pub site_id: Uuid,
    pub branch_name: String,
    pub temporal_queue: String,
    pub temporal_namespace: String,
    pub temporal_ui_url: String,
    pub status: MigrationStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationApprovalReceipt {
    pub approved: bool,
    pub migration_id: Uuid,
    pub status: MigrationStatus,
    pub approved_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationSyncReceipt {
    pub synced: bool,
    pub migration_id: Uuid,
    pub workflow_id: String,
    pub status: MigrationStatus,
    pub page_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowResultSyncRequest {
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationExtractionTriggerReceipt {
    pub admitted: bool,
    pub migration_id: Uuid,
    pub workflow_id: String,
    pub run_id: Option<String>,
    pub temporal_queue: String,
    pub temporal_namespace: String,
    pub temporal_ui_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportMigrationDraftReceipt {
    pub imported: bool,
    pub migration_id: Uuid,
    pub draft_change_set_id: Uuid,
    pub branch_name: String,
    pub base_snapshot_id: Option<Uuid>,
    pub change_count: usize,
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftChangeSetSummary {
    pub id: Uuid,
    pub site_id: Uuid,
    pub branch_name: String,
    pub base_snapshot_id: Option<Uuid>,
    pub source_kind: DraftChangeSetSourceKind,
    pub status: DraftChangeSetStatus,
    pub name: String,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub change_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftChangeSummary {
    pub id: Uuid,
    pub change_set_id: Uuid,
    pub site_id: Uuid,
    pub migration_job_id: Option<Uuid>,
    pub migration_page_id: Option<Uuid>,
    pub change_kind: DraftChangeKind,
    pub resource_kind: DraftResourceKind,
    pub resource_key: String,
    pub status: DraftChangeStatus,
    pub title: String,
    pub payload: serde_json::Value,
    pub preview_url: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/runtime", get(runtime_info))
        .route("/api/sites", get(sites))
        .route("/api/migrations", post(create_migration_without_site))
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
        .route("/api/sites/{site_id}/migrations", post(create_migration))
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
        .route("/api/migrations/{migration_id}", get(migration_detail))
        .route("/api/migrations/{migration_id}/pages", get(migration_pages))
        .route(
            "/api/migrations/{migration_id}/pages/{page_id}",
            get(migration_page_detail),
        )
        .route(
            "/api/migrations/{migration_id}/pages/{page_id}/artifact",
            get(migration_page_artifact_detail),
        )
        .route(
            "/api/migrations/{migration_id}/sync-discovery",
            post(sync_migration_discovery),
        )
        .route(
            "/api/migrations/{migration_id}/extract-page-documents",
            post(trigger_migration_page_extraction),
        )
        .route(
            "/api/migrations/{migration_id}/sync-extraction",
            post(sync_migration_page_extraction),
        )
        .route(
            "/api/migrations/{migration_id}/import-draft",
            post(import_migration_to_draft),
        )
        .route(
            "/api/migrations/{migration_id}/approve",
            post(approve_migration),
        )
        .route(
            "/api/draft-change-sets/{change_set_id}",
            get(draft_change_set_detail),
        )
        .route(
            "/api/draft-change-sets/{change_set_id}/changes",
            get(draft_change_set_changes),
        )
        .route(
            "/preview/draft-change-sets/{change_set_id}/changes/{change_id}",
            get(draft_change_preview),
        )
        .route("/api/workflows/definitions", get(workflow_definitions))
        .route("/api/demo/workflow-request", get(demo_workflow_request))
        .route("/api/demo/widget-command", get(demo_widget_command))
        .route("/preview/demo", get(preview_demo))
        .route("/migration-console", get(migration_console))
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

async fn sites(State(state): State<AppState>) -> Result<Json<Vec<SiteSummary>>, StatusCode> {
    if let Some(repository) = state.repository.as_ref() {
        let rows = repository.list_sites().await.map_err(internal_db_error)?;
        let sites = rows
            .into_iter()
            .map(site_summary_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(internal_mapping_error)?;
        Ok(Json(sites))
    } else {
        Ok(Json(state.catalog.sites().to_vec()))
    }
}

async fn site_branches(
    Path(site_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BranchSummary>>, StatusCode> {
    if let Some(repository) = state.repository.as_ref() {
        if !repository
            .site_exists(site_id)
            .await
            .map_err(internal_db_error)?
        {
            return Err(StatusCode::NOT_FOUND);
        }

        let rows = repository
            .list_branches_for_site(site_id)
            .await
            .map_err(internal_db_error)?;
        let branches = rows
            .into_iter()
            .map(branch_summary_from_row)
            .collect::<Vec<_>>();
        Ok(Json(branches))
    } else if state.catalog.sites().iter().any(|site| site.id == site_id) {
        Ok(Json(state.catalog.branches_for_site(site_id).to_vec()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn branch_head(
    Path((site_id, branch_name)): Path<(Uuid, String)>,
    State(state): State<AppState>,
) -> Result<Json<BranchHeadResponse>, StatusCode> {
    let branch = if let Some(repository) = state.repository.as_ref() {
        repository
            .branch_head(site_id, &branch_name)
            .await
            .map_err(internal_db_error)?
            .map(branch_summary_from_row)
            .ok_or(StatusCode::NOT_FOUND)?
    } else {
        state
            .catalog
            .branch_head(site_id, &branch_name)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)?
    };

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

    ensure_site_exists(&state, site_id).await?;
    upsert_workflow_request(&state, &request).await?;
    emit_workflow_event(&state, &request, "workflow.requested", None).await?;

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

    ensure_site_exists(&state, site_id).await?;
    upsert_workflow_request(&state, &request).await?;

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
    emit_workflow_event(
        &state,
        &request,
        "workflow.started",
        Some(serde_json::json!({
            "workflow_id": result.workflow_id,
            "run_id": result.run_id,
            "namespace": result.namespace,
            "task_queue": result.task_queue,
        })),
    )
    .await?;

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

async fn create_migration(
    Path(site_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreateMigrationRequest>,
) -> Result<(StatusCode, Json<CreateMigrationReceipt>), (StatusCode, Json<serde_json::Value>)> {
    let site_id = resolve_migration_site(&state, Some(site_id), &request).await?;
    create_migration_inner(state, site_id, request).await
}

async fn create_migration_without_site(
    State(state): State<AppState>,
    Json(request): Json<CreateMigrationRequest>,
) -> Result<(StatusCode, Json<CreateMigrationReceipt>), (StatusCode, Json<serde_json::Value>)> {
    let site_id = resolve_migration_site(&state, None, &request).await?;
    create_migration_inner(state, site_id, request).await
}

async fn create_migration_inner(
    state: AppState,
    site_id: Uuid,
    request: CreateMigrationRequest,
) -> Result<(StatusCode, Json<CreateMigrationReceipt>), (StatusCode, Json<serde_json::Value>)> {
    let workflow_request = migration_workflow_request(&state, site_id, &request)?;
    let definition = state.workflows.admit(&workflow_request).map_err(|error| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": error.to_string()
            })),
        )
    })?;

    upsert_workflow_request(&state, &workflow_request).await?;
    emit_workflow_event(&state, &workflow_request, "workflow.requested", None).await?;

    let result = start_temporal_workflow(&state.config, &workflow_request)
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": error
                })),
            )
        })?;

    emit_workflow_event(
        &state,
        &workflow_request,
        "workflow.started",
        Some(serde_json::json!({
            "workflow_id": result.workflow_id,
            "run_id": result.run_id,
            "namespace": result.namespace,
            "task_queue": result.task_queue,
        })),
    )
    .await?;

    let record =
        migration_record_from_request(site_id, &request, &workflow_request, &result.workflow_id);
    store_migration_record(&state, &record).await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(CreateMigrationReceipt {
            accepted: true,
            migration_id: record.id,
            workflow_request_id: workflow_request.id,
            workflow_id: result.workflow_id,
            site_id,
            branch_name: workflow_request.branch_name,
            temporal_queue: definition.temporal_queue.clone(),
            temporal_namespace: result.namespace,
            temporal_ui_url: state.config.temporal_ui_url.clone(),
            status: record.status,
        }),
    ))
}

async fn migration_detail(
    Path(migration_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<MigrationJobSummary>, StatusCode> {
    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(migration_summary(&record)))
}

async fn migration_pages(
    Path(migration_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<MigrationPageSummary>>, StatusCode> {
    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(
        record
            .pages
            .iter()
            .map(migration_page_summary)
            .collect::<Vec<_>>(),
    ))
}

async fn migration_page_detail(
    Path((migration_id, page_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> Result<Json<MigrationPageDetail>, StatusCode> {
    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let page = record
        .pages
        .iter()
        .find(|page| page.id == page_id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(page))
}

async fn migration_page_artifact_detail(
    Path((migration_id, page_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> Result<Json<MigrationPageArtifactDetail>, StatusCode> {
    let repository = state
        .repository
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !record.pages.iter().any(|page| page.id == page_id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let row = repository
        .migration_page_artifact(page_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    migration_page_artifact_detail_from_row(row)
        .map(Json)
        .map_err(internal_mapping_error)
}

async fn sync_migration_discovery(
    Path(migration_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<MigrationSyncReceipt>, (StatusCode, Json<serde_json::Value>)> {
    let repository = state.repository.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "migration discovery sync requires a configured database"
        })),
    ))?;
    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(json_db_error)?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("migration {migration_id} was not found")
            })),
        ))?;

    let workflow = load_temporal_workflow_result(&state.config, &record.workflow_id)
        .await
        .map_err(|error| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": error
                })),
            )
        })?;

    let synced_record = persist_migration_workflow_result(repository, &record, &workflow.result)
        .await
        .map_err(|error| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": error
                })),
            )
        })?;

    let synced_record = load_migration_record(&state, synced_record.id)
        .await
        .map_err(json_db_error)?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "synced migration record could not be reloaded"
            })),
        ))?;

    Ok(Json(MigrationSyncReceipt {
        synced: true,
        migration_id: synced_record.id,
        workflow_id: synced_record.workflow_id,
        status: synced_record.status,
        page_count: synced_record.pages.len(),
        warnings: synced_record.warnings,
    }))
}

async fn trigger_migration_page_extraction(
    Path(migration_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<MigrationExtractionTriggerReceipt>, (StatusCode, Json<serde_json::Value>)> {
    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(json_db_error)?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("migration {migration_id} was not found")
            })),
        ))?;

    let workflow_request = migration_page_extraction_workflow_request(&state, &record)?;
    let definition = state
        .workflows
        .admit(&workflow_request)
        .map_err(|error| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": error.to_string()
                })),
            )
        })?
        .clone();

    let result = start_temporal_workflow(&state.config, &workflow_request)
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": error
                })),
            )
        })?;

    Ok(Json(MigrationExtractionTriggerReceipt {
        admitted: true,
        migration_id,
        workflow_id: result.workflow_id,
        run_id: result.run_id,
        temporal_queue: definition.temporal_queue,
        temporal_namespace: result.namespace,
        temporal_ui_url: state.config.temporal_ui_url.clone(),
    }))
}

async fn sync_migration_page_extraction(
    Path(migration_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<WorkflowResultSyncRequest>,
) -> Result<Json<MigrationSyncReceipt>, (StatusCode, Json<serde_json::Value>)> {
    let repository = state.repository.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "migration extraction sync requires a configured database"
        })),
    ))?;
    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(json_db_error)?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("migration {migration_id} was not found")
            })),
        ))?;

    let workflow = load_temporal_workflow_result(&state.config, &request.workflow_id)
        .await
        .map_err(|error| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": error
                })),
            )
        })?;

    let synced_record = persist_migration_extraction_result(repository, &record, &workflow.result)
        .await
        .map_err(|error| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": error
                })),
            )
        })?;

    let synced_record = load_migration_record(&state, synced_record.id)
        .await
        .map_err(json_db_error)?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "synced migration record could not be reloaded"
            })),
        ))?;

    Ok(Json(MigrationSyncReceipt {
        synced: true,
        migration_id: synced_record.id,
        workflow_id: request.workflow_id,
        status: synced_record.status,
        page_count: synced_record.pages.len(),
        warnings: synced_record.warnings,
    }))
}

async fn approve_migration(
    Path(migration_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<MigrationApprovalReceipt>, StatusCode> {
    let approved_at = OffsetDateTime::now_utc();
    let status = if let Some(repository) = state.repository.as_ref() {
        repository
            .approve_migration_job(migration_id, approved_at)
            .await
            .map_err(internal_db_error)?
            .map(|row| parse_migration_status(&row.status))
            .transpose()
            .map_err(internal_mapping_error)?
            .ok_or(StatusCode::NOT_FOUND)?
    } else {
        let mut migrations = state.migrations.write().await;
        let record = migrations
            .get_mut(&migration_id)
            .ok_or(StatusCode::NOT_FOUND)?;
        record.status = MigrationStatus::Approved;
        record.approved_at = Some(approved_at);
        record.status.clone()
    };

    Ok(Json(MigrationApprovalReceipt {
        approved: true,
        migration_id,
        status,
        approved_at,
    }))
}

async fn import_migration_to_draft(
    Path(migration_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ImportMigrationDraftReceipt>, (StatusCode, Json<serde_json::Value>)> {
    let repository = state.repository.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "draft import requires a configured database"
        })),
    ))?;

    let record = load_migration_record(&state, migration_id)
        .await
        .map_err(json_db_error)?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("migration {migration_id} was not found")
            })),
        ))?;

    if !matches!(
        record.status,
        MigrationStatus::ReviewReady | MigrationStatus::Approved | MigrationStatus::Imported
    ) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "migration must be review_ready or approved before draft import"
            })),
        ));
    }

    let branch = resolve_or_create_migration_branch(repository, &record)
        .await
        .map_err(json_db_error)?;
    let now = OffsetDateTime::now_utc();
    let change_set_id = Uuid::new_v4();
    let name = format!("Migration import: {}", record.homepage_url);
    let description = Some(format!(
        "Imported {} discovered pages from migration {}",
        record.pages.len(),
        record.id
    ));

    let change_set_row = DraftChangeSetRow {
        id: change_set_id,
        site_id: record.site_id,
        branch_id: branch.id,
        base_snapshot_id: branch.head_snapshot_id,
        source_kind: draft_change_set_source_kind_name(DraftChangeSetSourceKind::MigrationImport)
            .to_owned(),
        status: draft_change_set_status_name(DraftChangeSetStatus::PreviewReady).to_owned(),
        name,
        description,
        metadata: sqlx::types::Json(serde_json::json!({
            "migration_job_id": record.id,
            "workflow_request_id": record.workflow_request_id,
            "homepage_url": record.homepage_url,
            "client_id": record.client_id,
            "location_id": record.location_id,
            "legacy_api_profile": record.legacy_api_profile,
        })),
        created_by: "migration-import".to_owned(),
        created_at: now,
        updated_at: now,
    };
    repository
        .insert_draft_change_set(&change_set_row)
        .await
        .map_err(json_db_error)?;

    let mut first_preview_url = None;
    for page in &record.pages {
        let artifact = repository
            .migration_page_artifact(page.id)
            .await
            .map_err(json_db_error)?;
        let imported_page = imported_page_draft_from_migration_page(&record, page, artifact);
        let change_row = DraftChangeRow {
            id: Uuid::new_v4(),
            change_set_id,
            site_id: record.site_id,
            page_id: None,
            migration_job_id: Some(record.id),
            migration_page_id: Some(page.id),
            change_kind: draft_change_kind_name(DraftChangeKind::UpsertPageDocument).to_owned(),
            resource_kind: draft_resource_kind_name(DraftResourceKind::Page).to_owned(),
            resource_key: page.path.clone(),
            status: draft_change_status_name(DraftChangeStatus::Imported).to_owned(),
            title: imported_page.title.clone(),
            payload: sqlx::types::Json(serde_json::to_value(&imported_page).map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("failed to encode imported page payload: {error}")
                    })),
                )
            })?),
            created_at: now,
            updated_at: now,
        };

        let inserted = repository
            .insert_draft_change(&change_row)
            .await
            .map_err(json_db_error)?;

        let seo = page_seo_json(&imported_page.page_document, &imported_page.seo)?;
        let draft_page_document_row = DraftPageDocumentRow {
            id: Uuid::new_v4(),
            change_set_id,
            draft_change_id: Some(inserted.id),
            page_id: None,
            path: imported_page.path.clone(),
            slug: imported_page.slug.clone(),
            title: imported_page.title.clone(),
            template_definition_id: None,
            template_key: imported_page.page_document.template_key.clone(),
            schema_version: imported_page.page_document.schema_version,
            seo: sqlx::types::Json(seo),
            document: sqlx::types::Json(
                serde_json::to_value(&imported_page.page_document).map_err(|error| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("failed to encode imported page document: {error}")
                        })),
                    )
                })?,
            ),
            metadata: sqlx::types::Json(serde_json::json!({
                "migration_job_id": record.id,
                "migration_page_id": page.id,
                "import_source": "migration_artifact",
            })),
            created_at: now,
            updated_at: now,
        };
        repository
            .insert_draft_page_document(&draft_page_document_row)
            .await
            .map_err(json_db_error)?;

        if first_preview_url.is_none() {
            first_preview_url = Some(format!(
                "/preview/draft-change-sets/{}/changes/{}",
                change_set_id, inserted.id
            ));
        }
    }

    repository
        .update_migration_job_status(
            migration_id,
            migration_status_name(&MigrationStatus::Imported),
        )
        .await
        .map_err(json_db_error)?;

    Ok(Json(ImportMigrationDraftReceipt {
        imported: true,
        migration_id,
        draft_change_set_id: change_set_id,
        branch_name: branch.name,
        base_snapshot_id: branch.head_snapshot_id,
        change_count: record.pages.len(),
        preview_url: first_preview_url,
    }))
}

async fn draft_change_set_detail(
    Path(change_set_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<DraftChangeSetSummary>, StatusCode> {
    let repository = state
        .repository
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let change_set_row = repository
        .draft_change_set(change_set_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let branch = repository
        .list_branches_for_site(change_set_row.site_id)
        .await
        .map_err(internal_db_error)?
        .into_iter()
        .find(|row| row.id == change_set_row.branch_id)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let change_count = repository
        .draft_changes(change_set_id)
        .await
        .map_err(internal_db_error)?
        .len();

    draft_change_set_summary_from_row(change_set_row, &branch.name, change_count)
        .map(Json)
        .map_err(internal_mapping_error)
}

async fn draft_change_set_changes(
    Path(change_set_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<DraftChangeSummary>>, StatusCode> {
    let repository = state
        .repository
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let rows = repository
        .draft_changes(change_set_id)
        .await
        .map_err(internal_db_error)?;
    let changes = rows
        .into_iter()
        .map(draft_change_summary_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_mapping_error)?;
    Ok(Json(changes))
}

async fn draft_change_preview(
    Path((change_set_id, change_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let repository = state
        .repository
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let change_set_row = repository
        .draft_change_set(change_set_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let change_row = repository
        .draft_change(change_set_id, change_id)
        .await
        .map_err(internal_db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let branch = repository
        .list_branches_for_site(change_set_row.site_id)
        .await
        .map_err(internal_db_error)?
        .into_iter()
        .find(|row| row.id == change_set_row.branch_id)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let preview = DraftPreviewRef {
        site_id: change_set_row.site_id,
        branch_name: branch.name,
        change_set_id,
    };
    let page: ImportedPageDraft = serde_json::from_value(change_row.payload.0)
        .map_err(|error| internal_mapping_error(error.to_string()))?;

    let html = state
        .renderer
        .render_imported_page_preview(&preview, &page)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}

async fn widget_definitions(
    State(state): State<AppState>,
) -> Result<Json<Vec<WidgetDefinition>>, StatusCode> {
    if let Some(repository) = state.repository.as_ref() {
        let rows = repository
            .list_widget_definitions()
            .await
            .map_err(internal_db_error)?;
        let definitions = rows
            .into_iter()
            .map(widget_definition_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(internal_mapping_error)?;
        Ok(Json(definitions))
    } else {
        Ok(Json(state.catalog.widget_definitions().to_vec()))
    }
}

async fn widget_definition_detail(
    Path(slug): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<WidgetDefinition>, StatusCode> {
    if let Some(repository) = state.repository.as_ref() {
        let row = repository
            .widget_definition_by_slug(&slug)
            .await
            .map_err(internal_db_error)?
            .ok_or(StatusCode::NOT_FOUND)?;
        widget_definition_from_row(row)
            .map(Json)
            .map_err(internal_mapping_error)
    } else {
        state
            .catalog
            .widget_definition(&slug)
            .cloned()
            .map(Json)
            .ok_or(StatusCode::NOT_FOUND)
    }
}

async fn widget_definition_versions(
    Path(slug): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<WidgetDefinitionVersion>>, StatusCode> {
    if let Some(repository) = state.repository.as_ref() {
        if repository
            .widget_definition_by_slug(&slug)
            .await
            .map_err(internal_db_error)?
            .is_none()
        {
            return Err(StatusCode::NOT_FOUND);
        }

        let rows = repository
            .list_widget_definition_versions(&slug)
            .await
            .map_err(internal_db_error)?;
        let versions = rows
            .into_iter()
            .map(widget_definition_version_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(internal_mapping_error)?;
        Ok(Json(versions))
    } else {
        if state.catalog.widget_definition(&slug).is_none() {
            return Err(StatusCode::NOT_FOUND);
        }

        Ok(Json(state.catalog.widget_versions(&slug).to_vec()))
    }
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
            <a class="btn" href="/migration-console">Migration console</a>
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

async fn migration_console() -> Html<String> {
    Html(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Migration Console</title>
    <style>
      :root {
        --bg: #f3f0e8;
        --panel: #fffdf8;
        --ink: #111111;
        --muted: #555555;
        --line: #111111;
        --accent: #d64a1f;
      }
      * { box-sizing: border-box; }
      body {
        margin: 0;
        background: var(--bg);
        color: var(--ink);
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      }
      .shell {
        min-height: 100vh;
        display: grid;
        grid-template-columns: minmax(320px, 420px) minmax(0, 1fr);
      }
      .pane {
        padding: 20px;
      }
      .stack {
        display: grid;
        gap: 14px;
      }
      .card {
        background: var(--panel);
        border: 2px solid var(--line);
        box-shadow: 8px 8px 0 var(--line);
        padding: 18px;
      }
      h1, h2, h3, p {
        margin: 0;
      }
      h1 {
        font-size: 1.7rem;
        line-height: 1;
      }
      h2 {
        font-size: 0.95rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
      }
      p {
        color: var(--muted);
        line-height: 1.5;
      }
      .actions, .inline {
        display: flex;
        gap: 10px;
        flex-wrap: wrap;
      }
      label {
        display: grid;
        gap: 6px;
        font-size: 0.8rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
      }
      input, select {
        width: 100%;
        min-height: 42px;
        border: 2px solid var(--line);
        background: #fff;
        color: var(--ink);
        padding: 10px 12px;
        font: inherit;
      }
      input[type="checkbox"] {
        width: auto;
        min-height: auto;
      }
      button, a.btn {
        min-height: 42px;
        border: 2px solid var(--line);
        background: var(--accent);
        color: #fff;
        padding: 10px 14px;
        font: inherit;
        text-decoration: none;
        cursor: pointer;
      }
      button.secondary, a.btn.secondary {
        background: #fff;
        color: var(--ink);
      }
      form {
        display: grid;
        gap: 12px;
      }
      .row-2 {
        display: grid;
        gap: 12px;
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }
      .checklist {
        display: grid;
        gap: 8px;
      }
      .check {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 10px 12px;
        border: 2px solid var(--line);
        background: #fff;
      }
      .status {
        font-size: 0.78rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
      }
      pre {
        margin: 0;
        white-space: pre-wrap;
        word-break: break-word;
        overflow: auto;
        font: inherit;
      }
      .pages {
        display: grid;
        gap: 10px;
      }
      .page {
        border: 2px solid var(--line);
        background: #fff;
        padding: 12px;
      }
      .page strong {
        display: block;
        margin-bottom: 6px;
      }
      .pill {
        display: inline-block;
        border: 2px solid var(--line);
        padding: 2px 6px;
        margin: 4px 4px 0 0;
        background: #fff7eb;
      }
      @media (max-width: 1020px) {
        .shell {
          grid-template-columns: 1fr;
        }
        .row-2 {
          grid-template-columns: 1fr;
        }
      }
    </style>
  </head>
  <body>
    <div class="shell">
      <aside class="pane">
        <div class="stack">
          <section class="card">
            <h2>Migration Console</h2>
            <h1>Crawl-first sandbox</h1>
            <p>
              Thin UI for migration workflows. Use it to kick off discovery, inspect
              review records, and approve a job without building the full management app.
            </p>
            <div class="actions" style="margin-top:14px;">
              <a class="btn secondary" href="/viewer">Back to viewer</a>
              <button type="button" class="secondary" id="refresh-sites">Refresh sites</button>
            </div>
          </section>

          <section class="card">
            <form id="migration-form">
              <label>
                Target Mode
                <select id="target-mode" name="target_mode">
                  <option value="create_site">create_site</option>
                  <option value="existing_site">existing_site</option>
                </select>
              </label>
              <label>
                Existing Site
                <select id="site-id" name="site_id"></select>
              </label>
              <div class="row-2">
                <label>
                  New Site Name
                  <input id="new-site-name" value="Hearth Migration" />
                </label>
                <label>
                  New Site Slug
                  <input id="new-site-slug" value="hearth-migration" />
                </label>
              </div>
              <div class="row-2">
                <label>
                  New Site Host
                  <input id="new-site-host" value="hearth-migration.local" />
                </label>
                <label>
                  Site Kind
                  <select id="new-site-kind">
                    <option value="standard">standard</option>
                    <option value="template">template</option>
                  </select>
                </label>
              </div>
              <div class="row-2">
                <label>
                  Account Name
                  <input id="account-name" value="Migration Account" />
                </label>
                <label>
                  Account Slug
                  <input id="account-slug" value="migration-account" />
                </label>
              </div>
              <label>
                Homepage URL
                <input id="homepage-url" name="homepage_url" value="https://g5-clw-hdmhijtexe-hearth.g5static.com/" />
              </label>
              <div class="row-2">
                <label>
                  Client ID
                  <input id="client-id" name="client_id" value="aaaaaaaa-1111-1111-1111-111111111111" />
                </label>
                <label>
                  Location ID
                  <input id="location-id" name="location_id" value="bbbbbbbb-2222-2222-2222-222222222222" />
                </label>
              </div>
              <div class="row-2">
                <label>
                  Crawl Scope
                  <select id="crawl-scope" name="crawl_scope">
                    <option value="subpath">subpath</option>
                    <option value="homepage_only">homepage_only</option>
                    <option value="site">site</option>
                  </select>
                </label>
                <label>
                  Max Pages
                  <input id="max-pages" name="max_pages" type="number" min="1" value="50" />
                </label>
              </div>
              <div class="checklist">
                <label class="check"><input id="follow-subdomains" type="checkbox" /> follow_subdomains</label>
                <label class="check"><input id="respect-robots" type="checkbox" checked /> respect_robots</label>
                <label class="check"><input id="include-assets" type="checkbox" checked /> include_assets</label>
                <label class="check"><input id="detect-widgets" type="checkbox" checked /> detect_registered_widgets</label>
                <label class="check"><input id="legacy-enrichment" type="checkbox" /> use_legacy_api_enrichment</label>
                <label class="check"><input id="screenshot-compare" type="checkbox" /> screenshot_compare_after_import</label>
              </div>
              <div class="actions">
                <button type="submit">Create migration</button>
              </div>
            </form>
          </section>
        </div>
      </aside>

      <main class="pane">
        <div class="stack">
          <section class="card">
            <div class="status" id="status">idle</div>
            <pre id="log">Loading sites…</pre>
          </section>

          <section class="card">
            <h2>Review</h2>
            <div class="actions" style="margin: 12px 0 14px;">
              <button type="button" class="secondary" id="load-latest">Load latest job</button>
              <button type="button" class="secondary" id="sync-discovery">Sync discovery</button>
              <button type="button" class="secondary" id="extract-pages">Extract page documents</button>
              <button type="button" class="secondary" id="sync-extraction">Sync extraction</button>
              <button type="button" id="approve-job">Approve job</button>
              <button type="button" class="secondary" id="import-draft">Import draft</button>
            </div>
            <div id="summary" class="stack"></div>
            <div id="pages" class="pages"></div>
          </section>
        </div>
      </main>
    </div>

    <script>
      const statusEl = document.getElementById("status");
      const logEl = document.getElementById("log");
      const targetModeEl = document.getElementById("target-mode");
      const siteSelect = document.getElementById("site-id");
      const summaryEl = document.getElementById("summary");
      const pagesEl = document.getElementById("pages");
      let latestMigrationId = null;
      let latestExtractionWorkflowId = null;

      function setStatus(value) {
        statusEl.textContent = value;
      }

      function setLog(value) {
        logEl.textContent = typeof value === "string" ? value : JSON.stringify(value, null, 2);
      }

      function renderSummary(job) {
        summaryEl.innerHTML = "";
        const block = document.createElement("div");
        block.innerHTML = `
          <p><strong>migration_id</strong>: ${job.id}</p>
          <p><strong>workflow_id</strong>: ${job.workflow_id}</p>
          <p><strong>status</strong>: ${job.status}</p>
          <p><strong>homepage</strong>: ${job.homepage_url}</p>
          <p><strong>page_count</strong>: ${job.page_count}</p>
        `;
        summaryEl.appendChild(block);

        if (job.warnings && job.warnings.length > 0) {
          const warnings = document.createElement("div");
          warnings.innerHTML = `<p><strong>warnings</strong></p><pre>${job.warnings.join("\n")}</pre>`;
          summaryEl.appendChild(warnings);
        }
      }

      function renderPages(pages) {
        pagesEl.innerHTML = "";
        for (const page of pages) {
          const node = document.createElement("article");
          node.className = "page";
          const widgetPills = (page.widget_matches || []).map((match) => `<span class="pill">${match}</span>`).join("");
          node.innerHTML = `
            <strong>${page.path}</strong>
            <div>${page.title_guess}</div>
            <div>confidence: ${page.confidence}</div>
            <div>unknown_regions: ${page.unknown_regions}</div>
            <div>${widgetPills || "<span class=\"pill\">no widgets detected</span>"}</div>
          `;
          pagesEl.appendChild(node);
        }
      }

      async function fetchJson(url, options = {}) {
        const response = await fetch(url, options);
        const body = await response.json().catch(() => ({}));
        if (!response.ok) {
          throw new Error(body.error || `request failed: ${response.status}`);
        }
        return body;
      }

      async function loadSites() {
        setStatus("loading sites");
        const sites = await fetchJson("/api/sites");
        siteSelect.innerHTML = "";
        if (!sites.length) {
          const option = document.createElement("option");
          option.textContent = "No sites found";
          option.value = "";
          siteSelect.appendChild(option);
          setLog("No sites are available yet. Use create_site mode to let the migration create one.");
          setStatus("ready");
          return;
        }

        for (const site of sites) {
          const option = document.createElement("option");
          option.value = site.id;
          option.textContent = `${site.name} (${site.slug})`;
          siteSelect.appendChild(option);
        }
        setLog(sites);
        setStatus("ready");
      }

      async function loadMigration(migrationId) {
        setStatus("loading migration");
        const job = await fetchJson(`/api/migrations/${migrationId}`);
        const pages = await fetchJson(`/api/migrations/${migrationId}/pages`);
        latestMigrationId = migrationId;
        renderSummary(job);
        renderPages(pages);
        setLog(job);
        setStatus(`loaded ${migrationId}`);
      }

      document.getElementById("refresh-sites").addEventListener("click", async () => {
        try {
          await loadSites();
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      document.getElementById("load-latest").addEventListener("click", async () => {
        if (!latestMigrationId) {
          setStatus("idle");
          setLog("No migration job has been created in this console session yet.");
          return;
        }
        try {
          await loadMigration(latestMigrationId);
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      document.getElementById("approve-job").addEventListener("click", async () => {
        if (!latestMigrationId) {
          setStatus("idle");
          setLog("No migration selected.");
          return;
        }
        try {
          const result = await fetchJson(`/api/migrations/${latestMigrationId}/approve`, {
            method: "POST"
          });
          setLog(result);
          await loadMigration(latestMigrationId);
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      document.getElementById("sync-discovery").addEventListener("click", async () => {
        if (!latestMigrationId) {
          setStatus("idle");
          setLog("No migration selected.");
          return;
        }
        try {
          setStatus("syncing discovery");
          const result = await fetchJson(`/api/migrations/${latestMigrationId}/sync-discovery`, {
            method: "POST"
          });
          setLog(result);
          await loadMigration(latestMigrationId);
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      document.getElementById("import-draft").addEventListener("click", async () => {
        if (!latestMigrationId) {
          setStatus("idle");
          setLog("No migration selected.");
          return;
        }
        try {
          setStatus("importing draft");
          const result = await fetchJson(`/api/migrations/${latestMigrationId}/import-draft`, {
            method: "POST"
          });
          setLog(result);
          if (result.preview_url) {
            window.open(result.preview_url, "_blank", "noopener,noreferrer");
          }
          await loadMigration(latestMigrationId);
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      document.getElementById("extract-pages").addEventListener("click", async () => {
        if (!latestMigrationId) {
          setStatus("idle");
          setLog("No migration selected.");
          return;
        }
        try {
          setStatus("triggering extraction");
          const result = await fetchJson(`/api/migrations/${latestMigrationId}/extract-page-documents`, {
            method: "POST"
          });
          latestExtractionWorkflowId = result.workflow_id;
          setLog(result);
          setStatus(`extraction started: ${result.workflow_id}`);
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      document.getElementById("sync-extraction").addEventListener("click", async () => {
        if (!latestMigrationId) {
          setStatus("idle");
          setLog("No migration selected.");
          return;
        }
        if (!latestExtractionWorkflowId) {
          setStatus("idle");
          setLog("No extraction workflow has been started in this console session yet.");
          return;
        }
        try {
          setStatus("syncing extraction");
          const result = await fetchJson(`/api/migrations/${latestMigrationId}/sync-extraction`, {
            method: "POST",
            headers: {
              "content-type": "application/json"
            },
            body: JSON.stringify({
              workflow_id: latestExtractionWorkflowId
            })
          });
          setLog(result);
          await loadMigration(latestMigrationId);
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      document.getElementById("migration-form").addEventListener("submit", async (event) => {
        event.preventDefault();
        const targetMode = targetModeEl.value;

        const payload = {
          target_site_id: targetMode === "existing_site" && siteSelect.value ? siteSelect.value : null,
          create_site: targetMode === "create_site" ? {
            account_name: document.getElementById("account-name").value.trim(),
            account_slug: document.getElementById("account-slug").value.trim(),
            name: document.getElementById("new-site-name").value.trim(),
            slug: document.getElementById("new-site-slug").value.trim(),
            primary_host: document.getElementById("new-site-host").value.trim(),
            site_kind: document.getElementById("new-site-kind").value
          } : null,
          homepage_url: document.getElementById("homepage-url").value.trim(),
          client_id: document.getElementById("client-id").value.trim(),
          location_id: document.getElementById("location-id").value.trim(),
          options: {
            crawl_scope: document.getElementById("crawl-scope").value,
            follow_subdomains: document.getElementById("follow-subdomains").checked,
            max_pages: Number(document.getElementById("max-pages").value),
            respect_robots: document.getElementById("respect-robots").checked,
            include_assets: document.getElementById("include-assets").checked,
            detect_registered_widgets: document.getElementById("detect-widgets").checked,
            use_legacy_api_enrichment: document.getElementById("legacy-enrichment").checked,
            screenshot_compare_after_import: document.getElementById("screenshot-compare").checked
          }
        };

        if (targetMode === "existing_site" && !siteSelect.value) {
          setStatus("blocked");
          setLog("No existing site is selected. Switch to create_site mode or create a site first.");
          return;
        }

        try {
          setStatus("creating migration");
          const result = await fetchJson(`/api/migrations`, {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(payload)
          });
          latestMigrationId = result.migration_id;
          latestExtractionWorkflowId = null;
          setLog(result);
          await loadMigration(result.migration_id);
        } catch (error) {
          setStatus("error");
          setLog(String(error));
        }
      });

      loadSites().catch((error) => {
        setStatus("error");
        setLog(String(error));
      });
    </script>
  </body>
</html>"#
            .to_owned(),
    )
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
            "component_id": "hero.v1",
            "provider": "anthropic",
            "model": "claude-sonnet-4-5-20250929",
            "context_documents": [
                {
                    "id": "homepage",
                    "title": "Homepage",
                    "content": "The current hero copy is generic and underplays the lifestyle angle."
                },
                {
                    "id": "brand-voice",
                    "title": "Brand Voice",
                    "content": "Use direct, hospitality-led language and avoid filler."
                }
            ],
            "evaluation": {
                "provider": "langsmith",
                "project": "rusty-cms",
                "evaluators": ["brand_voice", "clarity"],
                "tags": ["ai-content", "draft"]
            }
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

fn migration_workflow_request(
    state: &AppState,
    site_id: Uuid,
    request: &CreateMigrationRequest,
) -> Result<WorkflowRequest, (StatusCode, Json<serde_json::Value>)> {
    if request.options.max_pages == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "options.max_pages must be greater than 0"
            })),
        ));
    }

    let definition = state
        .workflows
        .definition_for_kind(WorkflowKind::SiteMigration)
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "site migration workflow definition is missing"
            })),
        ))?;

    Ok(WorkflowRequest {
        id: Uuid::new_v4(),
        kind: WorkflowKind::SiteMigration,
        site_id,
        branch_name: "migration/draft".to_owned(),
        requested_runtime: AgentRuntime::Python,
        temporal_queue: definition.temporal_queue.clone(),
        input_payload: serde_json::json!({
            "homepage_url": request.homepage_url,
            "client_id": request.client_id,
            "location_id": request.location_id,
            "legacy_api_profile": request.legacy_api_profile,
            "options": request.options,
        }),
        artifact_contract: WorkflowArtifactContract {
            output_schema: "schemas/site-migration-output.json".to_owned(),
            creates_snapshot: true,
            mutates_branch_head: false,
        },
        safety_policy: WorkflowSafetyPolicy {
            requires_human_approval: true,
            max_sites_touched: 1,
            allow_publish_side_effects: false,
        },
    })
}

fn migration_page_extraction_workflow_request(
    state: &AppState,
    record: &MigrationJobRecord,
) -> Result<WorkflowRequest, (StatusCode, Json<serde_json::Value>)> {
    let definition = state
        .workflows
        .definition_for_kind(WorkflowKind::SiteMigration)
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "site migration workflow definition is missing"
            })),
        ))?;

    let pages = record
        .pages
        .iter()
        .map(|page| {
            serde_json::json!({
                "path": page.path,
                "title_guess": page.title_guess,
                "widget_matches": page.widget_matches,
                "source_url": source_url_for_migration_page(record, page),
            })
        })
        .collect::<Vec<_>>();

    Ok(WorkflowRequest {
        id: Uuid::new_v4(),
        kind: WorkflowKind::SiteMigration,
        site_id: record.site_id,
        branch_name: record.branch_name.clone(),
        requested_runtime: AgentRuntime::Python,
        temporal_queue: definition.temporal_queue.clone(),
        input_payload: serde_json::json!({
            "action": "extract_page_documents",
            "migration_job_id": record.id,
            "homepage_url": record.homepage_url,
            "client_id": record.client_id,
            "location_id": record.location_id,
            "legacy_api_profile": record.legacy_api_profile,
            "options": record.options,
            "pages": pages,
        }),
        artifact_contract: WorkflowArtifactContract {
            output_schema: "schemas/site-migration-output.json".to_owned(),
            creates_snapshot: false,
            mutates_branch_head: false,
        },
        safety_policy: WorkflowSafetyPolicy {
            requires_human_approval: true,
            max_sites_touched: 1,
            allow_publish_side_effects: false,
        },
    })
}

async fn resolve_migration_site(
    state: &AppState,
    path_site_id: Option<Uuid>,
    request: &CreateMigrationRequest,
) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    match (
        path_site_id,
        request.target_site_id,
        request.create_site.as_ref(),
    ) {
        (Some(path_id), Some(body_id), _) if path_id != body_id => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "path site_id does not match target_site_id"
            })),
        )),
        (Some(path_id), _, Some(_)) => {
            ensure_site_exists(state, path_id).await?;
            Ok(path_id)
        }
        (Some(path_id), _, None) => {
            ensure_site_exists(state, path_id).await?;
            Ok(path_id)
        }
        (None, Some(site_id), None) => {
            ensure_site_exists(state, site_id).await?;
            Ok(site_id)
        }
        (None, None, Some(create_site)) => create_migration_target_site(state, create_site).await,
        (None, Some(_), Some(_)) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "provide either target_site_id or create_site, not both"
            })),
        )),
        (None, None, None) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "a migration target is required; provide target_site_id or create_site"
            })),
        )),
    }
}

async fn create_migration_target_site(
    state: &AppState,
    create_site: &CreateSiteTarget,
) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    if create_site.slug.trim().is_empty()
        || create_site.name.trim().is_empty()
        || create_site.primary_host.trim().is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "create_site name, slug, and primary_host are required"
            })),
        ));
    }

    let repository = state.repository.as_ref().ok_or((
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(serde_json::json!({
            "error": "DATABASE_URL must be configured to create a new site during migration"
        })),
    ))?;

    let now = OffsetDateTime::now_utc();
    let account_id = if let Some(account_id) = create_site.account_id {
        account_id
    } else {
        let account_name = create_site.account_name.clone().ok_or((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "create_site.account_name is required when account_id is omitted"
            })),
        ))?;
        let account_slug = create_site.account_slug.clone().ok_or((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "create_site.account_slug is required when account_id is omitted"
            })),
        ))?;
        let account_row = AccountRow {
            id: Uuid::new_v4(),
            name: account_name,
            slug: account_slug,
            created_at: now,
            updated_at: now,
        };
        repository
            .insert_account(&account_row)
            .await
            .map_err(json_db_error)?
            .id
    };

    let site_row = SiteRow {
        id: Uuid::new_v4(),
        account_id,
        name: create_site.name.clone(),
        slug: create_site.slug.clone(),
        primary_host: create_site.primary_host.clone(),
        site_kind: site_kind_name(create_site.site_kind).to_owned(),
        source_template_site_id: None,
        created_at: now,
        updated_at: now,
    };
    let site_id = repository
        .insert_site(&site_row)
        .await
        .map_err(json_db_error)?
        .id;

    for branch_name in ["draft", "production", "migration/draft"] {
        let branch_row = BranchRow {
            id: Uuid::new_v4(),
            site_id,
            name: branch_name.to_owned(),
            head_snapshot_id: None,
            created_at: now,
            updated_at: now,
        };
        repository
            .insert_branch(&branch_row)
            .await
            .map_err(json_db_error)?;
    }

    Ok(site_id)
}

fn migration_record_from_request(
    site_id: Uuid,
    request: &CreateMigrationRequest,
    workflow_request: &WorkflowRequest,
    workflow_id: &str,
) -> MigrationJobRecord {
    let homepage_page_id = Uuid::new_v4();
    MigrationJobRecord {
        id: Uuid::new_v4(),
        site_id,
        workflow_request_id: workflow_request.id,
        workflow_id: workflow_id.to_owned(),
        branch_name: workflow_request.branch_name.clone(),
        homepage_url: request.homepage_url.clone(),
        client_id: request.client_id,
        location_id: request.location_id,
        legacy_api_profile: request.legacy_api_profile.clone(),
        status: MigrationStatus::Running,
        options: request.options.clone(),
        pages: vec![MigrationPageDetail {
            id: homepage_page_id,
            path: homepage_path(&request.homepage_url),
            title_guess: "Homepage".to_owned(),
            widget_matches: if request.options.detect_registered_widgets {
                vec!["registry-detection-pending".to_owned()]
            } else {
                Vec::new()
            },
            unknown_regions: 1,
            confidence: 0.25,
            warnings: vec![
                "Discovery scaffolded only. Crawl and DOM extraction are not implemented yet."
                    .to_owned(),
            ],
            extraction_notes: vec![
                "This placeholder page record exists so the review UI contract can be built before crawler integration."
                    .to_owned(),
            ],
        }],
        warnings: migration_warnings(request),
        created_at: OffsetDateTime::now_utc(),
        approved_at: None,
    }
}

fn migration_summary(record: &MigrationJobRecord) -> MigrationJobSummary {
    MigrationJobSummary {
        id: record.id,
        site_id: record.site_id,
        workflow_request_id: record.workflow_request_id,
        workflow_id: record.workflow_id.clone(),
        branch_name: record.branch_name.clone(),
        homepage_url: record.homepage_url.clone(),
        client_id: record.client_id,
        location_id: record.location_id,
        legacy_api_profile: record.legacy_api_profile.clone(),
        status: record.status.clone(),
        options: record.options.clone(),
        page_count: record.pages.len(),
        warnings: record.warnings.clone(),
        created_at: record.created_at,
        approved_at: record.approved_at,
    }
}

fn migration_page_summary(page: &MigrationPageDetail) -> MigrationPageSummary {
    MigrationPageSummary {
        id: page.id,
        path: page.path.clone(),
        title_guess: page.title_guess.clone(),
        widget_matches: page.widget_matches.clone(),
        unknown_regions: page.unknown_regions,
        confidence: page.confidence,
    }
}

async fn resolve_or_create_migration_branch(
    repository: &PgRepository,
    record: &MigrationJobRecord,
) -> Result<BranchRow, sqlx::Error> {
    if let Some(branch) = repository
        .branch_head(record.site_id, &record.branch_name)
        .await?
    {
        return Ok(branch);
    }

    let fallback_branch = repository
        .branch_head(record.site_id, "draft")
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    let now = OffsetDateTime::now_utc();
    let branch_row = BranchRow {
        id: Uuid::new_v4(),
        site_id: record.site_id,
        name: record.branch_name.clone(),
        head_snapshot_id: fallback_branch.head_snapshot_id,
        created_at: now,
        updated_at: now,
    };

    repository.insert_branch(&branch_row).await
}

fn imported_page_draft_from_migration_page(
    record: &MigrationJobRecord,
    page: &MigrationPageDetail,
    artifact_row: Option<MigrationPageArtifactRow>,
) -> ImportedPageDraft {
    let slug = slug_from_path(&page.path);
    let artifact = artifact_row
        .map(|row| row.artifact.0)
        .unwrap_or_else(|| serde_json::json!({}));
    let seo = artifact
        .get("seo")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let schema_types = artifact
        .get("schema_types")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let layout = artifact
        .get("layout")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "regions": [], "counts": {} }));
    let text_blocks = artifact
        .get("text_blocks")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let html_excerpt = artifact
        .get("html_excerpt")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let images = artifact
        .get("images")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let media_text_regions = artifact
        .get("media_text_regions")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let document_candidate = artifact
        .get("document_candidate")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "regions": { "main": [] } }));
    let page_document = page_document_from_artifact(&artifact, &page.title_guess);

    let summary = if page.widget_matches.is_empty() {
        format!(
            "Imported discovery summary for {} with {} unknown regions.",
            page.path, page.unknown_regions
        )
    } else {
        format!(
            "Imported discovery summary for {} with widget matches: {}.",
            page.path,
            page.widget_matches.join(", ")
        )
    };

    ImportedPageDraft {
        migration_job_id: record.id,
        migration_page_id: page.id,
        path: page.path.clone(),
        slug,
        title: page.title_guess.clone(),
        summary,
        widget_matches: page.widget_matches.clone(),
        warnings: page.warnings.clone(),
        extraction_notes: page.extraction_notes.clone(),
        unknown_regions: page.unknown_regions,
        seo,
        schema_types,
        layout,
        text_blocks,
        images,
        media_text_regions,
        html_excerpt,
        page_document,
        document_candidate,
    }
}

fn source_url_for_migration_page(
    record: &MigrationJobRecord,
    page: &MigrationPageDetail,
) -> String {
    let base = record.homepage_url.trim_end_matches('/');
    if page.path == "/" {
        return format!("{base}/");
    }
    format!("{base}{}", page.path)
}

fn page_document_from_artifact(artifact: &serde_json::Value, page_title: &str) -> PageDocument {
    let document_candidate = artifact
        .get("document_candidate")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "regions": { "main": [] } }));
    let mut document = PageDocument::from_candidate(&document_candidate);

    if document.is_empty() {
        let mut fallback_blocks = Vec::new();

        if let Some(media_text_regions) = artifact
            .get("media_text_regions")
            .and_then(serde_json::Value::as_array)
        {
            for (index, region) in media_text_regions.iter().enumerate() {
                fallback_blocks.push(serde_json::json!({
                    "id": format!("media_text_{}", index + 1),
                    "kind": "primitive",
                    "primitive_type": "media_text",
                    "content": region,
                }));
            }
        }

        if fallback_blocks.is_empty()
            && let Some(images) = artifact.get("images").and_then(serde_json::Value::as_array)
        {
            for (index, image) in images.iter().take(4).enumerate() {
                fallback_blocks.push(serde_json::json!({
                    "id": format!("image_{}", index + 1),
                    "kind": "primitive",
                    "primitive_type": "image",
                    "content": image,
                }));
            }
        }

        if fallback_blocks.is_empty()
            && let Some(text_blocks) = artifact
                .get("text_blocks")
                .and_then(serde_json::Value::as_array)
        {
            for (index, text) in text_blocks.iter().take(4).enumerate() {
                fallback_blocks.push(serde_json::json!({
                    "id": format!("text_{}", index + 1),
                    "kind": "primitive",
                    "primitive_type": "rich_text",
                    "content": {
                        "text": text
                    },
                }));
            }
        }

        document = PageDocument::from_candidate(&serde_json::json!({
            "template_key": "marketing-default",
            "metadata": {
                "import_title": page_title,
            },
            "regions": {
                "main": fallback_blocks
            }
        }));
    }

    document
}

fn page_seo_json(
    document: &PageDocument,
    fallback: &serde_json::Value,
) -> Result<serde_json::Value, (StatusCode, Json<serde_json::Value>)> {
    let parsed = if fallback.is_null()
        || fallback.as_object().is_some_and(|object| object.is_empty())
    {
        PageSeo::default()
    } else {
        serde_json::from_value::<PageSeo>(fallback.clone()).unwrap_or_else(|_| PageSeo::default())
    };

    let seo = PageSeo {
        schema_types: if parsed.schema_types.is_empty() {
            fallback
                .get("schema_types")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            parsed.schema_types
        },
        metadata: serde_json::json!({
            "template_key": document.template_key,
        }),
        ..parsed
    };

    serde_json::to_value(seo).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("failed to encode page seo: {error}")
            })),
        )
    })
}

async fn store_migration_record(
    state: &AppState,
    record: &MigrationJobRecord,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if let Some(repository) = state.repository.as_ref() {
        let job_row = migration_job_row_from_record(record).map_err(json_mapping_error)?;
        repository
            .insert_migration_job(&job_row)
            .await
            .map_err(json_db_error)?;

        for page in &record.pages {
            let page_row = migration_page_row_from_detail(record.id, page);
            repository
                .insert_migration_page(&page_row)
                .await
                .map_err(json_db_error)?;
        }
    } else {
        state
            .migrations
            .write()
            .await
            .insert(record.id, record.clone());
    }

    Ok(())
}

async fn load_migration_record(
    state: &AppState,
    migration_id: Uuid,
) -> Result<Option<MigrationJobRecord>, sqlx::Error> {
    if let Some(repository) = state.repository.as_ref() {
        let Some(job_row) = repository.migration_job(migration_id).await? else {
            return Ok(None);
        };
        let page_rows = repository.migration_pages(migration_id).await?;
        migration_record_from_rows(job_row, page_rows)
            .map(Some)
            .map_err(sqlx::Error::Protocol)
    } else {
        Ok(state.migrations.read().await.get(&migration_id).cloned())
    }
}

fn migration_warnings(request: &CreateMigrationRequest) -> Vec<String> {
    let mut warnings = vec![
        "Migration foundation is scaffolded; crawler, classifier, and importer are pending."
            .to_owned(),
    ];
    if request.options.use_legacy_api_enrichment {
        warnings.push(
            "Legacy API enrichment is enabled in the request contract but not implemented yet."
                .to_owned(),
        );
    }
    if request.options.screenshot_compare_after_import {
        warnings.push(
            "Screenshot comparison is planned, but no capture/diff pipeline exists yet.".to_owned(),
        );
    }
    warnings
}

fn homepage_path(homepage_url: &str) -> String {
    if let Some((_, remainder)) = homepage_url.split_once("://") {
        if let Some((_, path)) = remainder.split_once('/') {
            return format!("/{}", path);
        }
    }
    "/".to_owned()
}

fn slug_from_path(path: &str) -> String {
    path.trim_matches('/')
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("home")
        .to_owned()
}

fn migration_job_row_from_record(record: &MigrationJobRecord) -> Result<MigrationJobRow, String> {
    Ok(MigrationJobRow {
        id: record.id,
        site_id: record.site_id,
        workflow_request_id: record.workflow_request_id,
        workflow_id: record.workflow_id.clone(),
        branch_name: record.branch_name.clone(),
        homepage_url: record.homepage_url.clone(),
        client_id: record.client_id,
        location_id: record.location_id,
        legacy_api_profile: record.legacy_api_profile.clone(),
        status: migration_status_name(&record.status).to_owned(),
        options: sqlx::types::Json(
            serde_json::to_value(&record.options).map_err(|error| error.to_string())?,
        ),
        warnings: sqlx::types::Json(record.warnings.clone()),
        created_at: record.created_at,
        approved_at: record.approved_at,
    })
}

fn migration_page_row_from_detail(
    migration_job_id: Uuid,
    page: &MigrationPageDetail,
) -> MigrationPageRow {
    let now = OffsetDateTime::now_utc();
    MigrationPageRow {
        id: page.id,
        migration_job_id,
        path: page.path.clone(),
        title_guess: page.title_guess.clone(),
        widget_matches: sqlx::types::Json(page.widget_matches.clone()),
        unknown_regions: page.unknown_regions as i32,
        confidence: page.confidence,
        warnings: sqlx::types::Json(page.warnings.clone()),
        extraction_notes: sqlx::types::Json(page.extraction_notes.clone()),
        created_at: now,
        updated_at: now,
    }
}

fn migration_record_from_rows(
    job_row: MigrationJobRow,
    page_rows: Vec<MigrationPageRow>,
) -> Result<MigrationJobRecord, String> {
    Ok(MigrationJobRecord {
        id: job_row.id,
        site_id: job_row.site_id,
        workflow_request_id: job_row.workflow_request_id,
        workflow_id: job_row.workflow_id,
        branch_name: job_row.branch_name,
        homepage_url: job_row.homepage_url,
        client_id: job_row.client_id,
        location_id: job_row.location_id,
        legacy_api_profile: job_row.legacy_api_profile,
        status: parse_migration_status(&job_row.status)?,
        options: serde_json::from_value(job_row.options.0).map_err(|error| error.to_string())?,
        pages: page_rows
            .into_iter()
            .map(migration_page_detail_from_row)
            .collect::<Vec<_>>(),
        warnings: job_row.warnings.0,
        created_at: job_row.created_at,
        approved_at: job_row.approved_at,
    })
}

fn migration_page_detail_from_row(row: MigrationPageRow) -> MigrationPageDetail {
    MigrationPageDetail {
        id: row.id,
        path: row.path,
        title_guess: row.title_guess,
        widget_matches: row.widget_matches.0,
        unknown_regions: row.unknown_regions.max(0) as u32,
        confidence: row.confidence,
        warnings: row.warnings.0,
        extraction_notes: row.extraction_notes.0,
    }
}

fn migration_page_artifact_detail_from_row(
    row: MigrationPageArtifactRow,
) -> Result<MigrationPageArtifactDetail, String> {
    Ok(MigrationPageArtifactDetail {
        page_id: row.migration_page_id,
        source_url: row.source_url,
        http_status: row.http_status.map(|value| value.max(0) as u16),
        final_url: row.final_url,
        artifact: row.artifact.0,
    })
}

async fn persist_migration_workflow_result(
    repository: &PgRepository,
    record: &MigrationJobRecord,
    workflow_result: &serde_json::Value,
) -> Result<MigrationJobRecord, String> {
    let migration = workflow_result
        .get("migration")
        .and_then(serde_json::Value::as_object)
        .ok_or("workflow result is missing a migration payload")?;
    let status = migration
        .get("status")
        .and_then(serde_json::Value::as_str)
        .ok_or("migration result is missing status")?;
    let warnings = migration
        .get("warnings")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let pages = migration
        .get("pages")
        .and_then(serde_json::Value::as_array)
        .ok_or("migration result is missing pages")?;

    repository
        .update_migration_job_review_data(
            record.id,
            migration_status_name(&parse_migration_status(status)?),
            &sqlx::types::Json(warnings.clone()),
        )
        .await
        .map_err(|error| error.to_string())?;
    repository
        .delete_migration_pages(record.id)
        .await
        .map_err(|error| error.to_string())?;

    let mut page_details = Vec::new();
    for page in pages {
        let page_id = Uuid::new_v4();
        let page_detail = migration_page_detail_from_result(page_id, page)?;
        let page_row = migration_page_row_from_detail(record.id, &page_detail);
        repository
            .insert_migration_page(&page_row)
            .await
            .map_err(|error| error.to_string())?;
        let artifact_row = migration_page_artifact_row_from_result(page_id, page)?;
        repository
            .upsert_migration_page_artifact(&artifact_row)
            .await
            .map_err(|error| error.to_string())?;
        page_details.push(page_detail);
    }

    Ok(MigrationJobRecord {
        id: record.id,
        site_id: record.site_id,
        workflow_request_id: record.workflow_request_id,
        workflow_id: record.workflow_id.clone(),
        branch_name: record.branch_name.clone(),
        homepage_url: record.homepage_url.clone(),
        client_id: record.client_id,
        location_id: record.location_id,
        legacy_api_profile: record.legacy_api_profile.clone(),
        status: parse_migration_status(status)?,
        options: record.options.clone(),
        pages: page_details,
        warnings,
        created_at: record.created_at,
        approved_at: record.approved_at,
    })
}

async fn persist_migration_extraction_result(
    repository: &PgRepository,
    record: &MigrationJobRecord,
    workflow_result: &serde_json::Value,
) -> Result<MigrationJobRecord, String> {
    let migration = workflow_result
        .get("migration")
        .and_then(serde_json::Value::as_object)
        .ok_or("workflow result is missing a migration payload")?;
    let action = migration
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if action != "extract_page_documents" {
        return Err(format!(
            "workflow result action was {action:?}; expected extract_page_documents"
        ));
    }

    let warnings = migration
        .get("warnings")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let pages = migration
        .get("pages")
        .and_then(serde_json::Value::as_array)
        .ok_or("migration extraction result is missing pages")?;

    repository
        .update_migration_job_review_data(
            record.id,
            migration_status_name(&MigrationStatus::ReviewReady),
            &sqlx::types::Json(warnings.clone()),
        )
        .await
        .map_err(|error| error.to_string())?;

    let existing_rows = repository
        .migration_pages(record.id)
        .await
        .map_err(|error| error.to_string())?;
    let by_path = existing_rows
        .into_iter()
        .map(|row| (row.path.clone(), row))
        .collect::<HashMap<_, _>>();

    for page in pages {
        let path = page
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or("extracted page is missing path")?;
        let Some(existing_row) = by_path.get(path) else {
            continue;
        };
        let page_detail = migration_page_detail_from_result(existing_row.id, page)?;
        let page_row = migration_page_row_from_detail(record.id, &page_detail);
        repository
            .update_migration_page(&page_row)
            .await
            .map_err(|error| error.to_string())?;
        let artifact_row = migration_page_artifact_row_from_result(existing_row.id, page)?;
        repository
            .upsert_migration_page_artifact(&artifact_row)
            .await
            .map_err(|error| error.to_string())?;
    }

    Ok(MigrationJobRecord {
        warnings,
        ..record.clone()
    })
}

fn migration_page_detail_from_result(
    page_id: Uuid,
    page: &serde_json::Value,
) -> Result<MigrationPageDetail, String> {
    let object = page
        .as_object()
        .ok_or("migration page result must be an object")?;
    Ok(MigrationPageDetail {
        id: page_id,
        path: object
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("/")
            .to_owned(),
        title_guess: object
            .get("title_guess")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Homepage")
            .to_owned(),
        widget_matches: object
            .get("widget_matches")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        unknown_regions: object
            .get("unknown_regions")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32,
        confidence: object
            .get("confidence")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0) as f32,
        warnings: object
            .get("warnings")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        extraction_notes: object
            .get("extraction_notes")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    })
}

fn migration_page_artifact_row_from_result(
    page_id: Uuid,
    page: &serde_json::Value,
) -> Result<MigrationPageArtifactRow, String> {
    let object = page
        .as_object()
        .ok_or("migration page result must be an object")?;
    let now = OffsetDateTime::now_utc();
    Ok(MigrationPageArtifactRow {
        id: Uuid::new_v4(),
        migration_page_id: page_id,
        source_url: object
            .get("source_url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        http_status: object
            .get("http_status")
            .and_then(serde_json::Value::as_i64)
            .map(|value| value as i32),
        final_url: object
            .get("final_url")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        artifact: sqlx::types::Json(serde_json::json!({
            "seo": object.get("seo").cloned().unwrap_or_else(|| serde_json::json!({})),
            "schema_types": object.get("schema_types").cloned().unwrap_or_else(|| serde_json::json!([])),
            "layout": object.get("layout").cloned().unwrap_or_else(|| serde_json::json!({ "regions": [], "counts": {} })),
            "text_blocks": object.get("text_blocks").cloned().unwrap_or_else(|| serde_json::json!([])),
            "images": object.get("images").cloned().unwrap_or_else(|| serde_json::json!([])),
            "media_text_regions": object.get("media_text_regions").cloned().unwrap_or_else(|| serde_json::json!([])),
            "html_excerpt": object.get("html_excerpt").cloned().unwrap_or(serde_json::Value::Null),
            "document_candidate": object.get("document_candidate").cloned().unwrap_or_else(|| serde_json::json!({ "regions": { "main": [] } })),
            "internal_links": object.get("internal_links").cloned().unwrap_or_else(|| serde_json::json!([])),
            "asset_urls": object.get("asset_urls").cloned().unwrap_or_else(|| serde_json::json!([])),
        })),
        created_at: now,
        updated_at: now,
    })
}

fn draft_change_set_summary_from_row(
    row: DraftChangeSetRow,
    branch_name: &str,
    change_count: usize,
) -> Result<DraftChangeSetSummary, String> {
    Ok(DraftChangeSetSummary {
        id: row.id,
        site_id: row.site_id,
        branch_name: branch_name.to_owned(),
        base_snapshot_id: row.base_snapshot_id,
        source_kind: parse_draft_change_set_source_kind(&row.source_kind)?,
        status: parse_draft_change_set_status(&row.status)?,
        name: row.name,
        description: row.description,
        metadata: row.metadata.0,
        created_at: row.created_at,
        updated_at: row.updated_at,
        change_count,
    })
}

fn draft_change_summary_from_row(row: DraftChangeRow) -> Result<DraftChangeSummary, String> {
    Ok(DraftChangeSummary {
        id: row.id,
        change_set_id: row.change_set_id,
        site_id: row.site_id,
        migration_job_id: row.migration_job_id,
        migration_page_id: row.migration_page_id,
        change_kind: parse_draft_change_kind(&row.change_kind)?,
        resource_kind: parse_draft_resource_kind(&row.resource_kind)?,
        resource_key: row.resource_key,
        status: parse_draft_change_status(&row.status)?,
        title: row.title,
        payload: row.payload.0,
        preview_url: format!(
            "/preview/draft-change-sets/{}/changes/{}",
            row.change_set_id, row.id
        ),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn migration_status_name(status: &MigrationStatus) -> &'static str {
    match status {
        MigrationStatus::Queued => "queued",
        MigrationStatus::Running => "running",
        MigrationStatus::ReviewReady => "review_ready",
        MigrationStatus::Approved => "approved",
        MigrationStatus::Imported => "imported",
        MigrationStatus::Failed => "failed",
    }
}

fn draft_change_set_source_kind_name(kind: DraftChangeSetSourceKind) -> &'static str {
    match kind {
        DraftChangeSetSourceKind::MigrationImport => "migration_import",
        DraftChangeSetSourceKind::ManualEdits => "manual_edits",
        DraftChangeSetSourceKind::TemplateSync => "template_sync",
        DraftChangeSetSourceKind::BulkOperation => "bulk_operation",
    }
}

fn draft_change_set_status_name(status: DraftChangeSetStatus) -> &'static str {
    match status {
        DraftChangeSetStatus::Open => "open",
        DraftChangeSetStatus::PreviewReady => "preview_ready",
        DraftChangeSetStatus::Published => "published",
        DraftChangeSetStatus::Discarded => "discarded",
    }
}

fn draft_change_kind_name(kind: DraftChangeKind) -> &'static str {
    match kind {
        DraftChangeKind::UpsertPageShell => "upsert_page_shell",
        DraftChangeKind::UpsertPageDocument => "upsert_page_document",
        DraftChangeKind::ApplyPageMutation => "apply_page_mutation",
    }
}

fn draft_resource_kind_name(kind: DraftResourceKind) -> &'static str {
    match kind {
        DraftResourceKind::Page => "page",
        DraftResourceKind::Template => "template",
    }
}

fn draft_change_status_name(status: DraftChangeStatus) -> &'static str {
    match status {
        DraftChangeStatus::Pending => "pending",
        DraftChangeStatus::Selected => "selected",
        DraftChangeStatus::Imported => "imported",
        DraftChangeStatus::Discarded => "discarded",
    }
}

fn parse_migration_status(value: &str) -> Result<MigrationStatus, String> {
    match normalize_name(value).as_str() {
        "queued" => Ok(MigrationStatus::Queued),
        "running" => Ok(MigrationStatus::Running),
        "reviewready" | "review_ready" => Ok(MigrationStatus::ReviewReady),
        "approved" => Ok(MigrationStatus::Approved),
        "imported" => Ok(MigrationStatus::Imported),
        "failed" => Ok(MigrationStatus::Failed),
        _ => Err(format!("unknown migration status: {value}")),
    }
}

fn parse_draft_change_set_source_kind(value: &str) -> Result<DraftChangeSetSourceKind, String> {
    match normalize_name(value).as_str() {
        "migrationimport" | "migration_import" => Ok(DraftChangeSetSourceKind::MigrationImport),
        "manualedits" | "manual_edits" => Ok(DraftChangeSetSourceKind::ManualEdits),
        "templatesync" | "template_sync" => Ok(DraftChangeSetSourceKind::TemplateSync),
        "bulkoperation" | "bulk_operation" => Ok(DraftChangeSetSourceKind::BulkOperation),
        _ => Err(format!("unknown draft change set source kind: {value}")),
    }
}

fn parse_draft_change_set_status(value: &str) -> Result<DraftChangeSetStatus, String> {
    match normalize_name(value).as_str() {
        "open" => Ok(DraftChangeSetStatus::Open),
        "previewready" | "preview_ready" => Ok(DraftChangeSetStatus::PreviewReady),
        "published" => Ok(DraftChangeSetStatus::Published),
        "discarded" => Ok(DraftChangeSetStatus::Discarded),
        _ => Err(format!("unknown draft change set status: {value}")),
    }
}

fn parse_draft_change_kind(value: &str) -> Result<DraftChangeKind, String> {
    match normalize_name(value).as_str() {
        "upsertpageshell" | "upsert_page_shell" => Ok(DraftChangeKind::UpsertPageShell),
        "upsertpagedocument" | "upsert_page_document" => Ok(DraftChangeKind::UpsertPageDocument),
        "applypagemutation" | "apply_page_mutation" => Ok(DraftChangeKind::ApplyPageMutation),
        _ => Err(format!("unknown draft change kind: {value}")),
    }
}

fn parse_draft_resource_kind(value: &str) -> Result<DraftResourceKind, String> {
    match normalize_name(value).as_str() {
        "page" => Ok(DraftResourceKind::Page),
        "template" => Ok(DraftResourceKind::Template),
        _ => Err(format!("unknown draft resource kind: {value}")),
    }
}

fn parse_draft_change_status(value: &str) -> Result<DraftChangeStatus, String> {
    match normalize_name(value).as_str() {
        "pending" => Ok(DraftChangeStatus::Pending),
        "selected" => Ok(DraftChangeStatus::Selected),
        "imported" => Ok(DraftChangeStatus::Imported),
        "discarded" => Ok(DraftChangeStatus::Discarded),
        _ => Err(format!("unknown draft change status: {value}")),
    }
}

async fn ensure_site_exists(
    state: &AppState,
    site_id: Uuid,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if let Some(repository) = state.repository.as_ref() {
        let exists = repository
            .site_exists(site_id)
            .await
            .map_err(json_db_error)?;
        if !exists {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("site {site_id} was not found")
                })),
            ));
        }
    } else if !state.catalog.sites().iter().any(|site| site.id == site_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("site {site_id} was not found")
            })),
        ));
    }

    Ok(())
}

async fn upsert_workflow_request(
    state: &AppState,
    request: &WorkflowRequest,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let Some(repository) = state.repository.as_ref() else {
        return Ok(());
    };

    let now = OffsetDateTime::now_utc();
    let request_row = WorkflowRequestRow {
        id: request.id,
        site_id: request.site_id,
        branch_name: request.branch_name.clone(),
        workflow_kind: workflow_kind_name(request.kind).to_owned(),
        requested_runtime: agent_runtime_name(request.requested_runtime).to_owned(),
        temporal_queue: request.temporal_queue.clone(),
        input_payload: sqlx::types::Json(request.input_payload.clone()),
        output_schema: request.artifact_contract.output_schema.clone(),
        requires_human_approval: request.safety_policy.requires_human_approval,
        max_sites_touched: request.safety_policy.max_sites_touched as i32,
        allow_publish_side_effects: request.safety_policy.allow_publish_side_effects,
        created_at: now,
    };
    repository
        .insert_workflow_request(&request_row)
        .await
        .map_err(json_db_error)?;

    Ok(())
}

async fn emit_workflow_event(
    state: &AppState,
    request: &WorkflowRequest,
    topic: &str,
    extra_payload: Option<serde_json::Value>,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let Some(repository) = state.repository.as_ref() else {
        return Ok(());
    };

    let now = OffsetDateTime::now_utc();
    let outbox_row = OutboxEventRow {
        id: Uuid::new_v4(),
        topic: topic.to_owned(),
        event_key: request.id.to_string(),
        payload: sqlx::types::Json(merge_json(
            serde_json::json!({
            "workflow_request_id": request.id,
            "site_id": request.site_id,
            "branch_name": request.branch_name,
            "workflow_kind": workflow_kind_name(request.kind),
            "requested_runtime": agent_runtime_name(request.requested_runtime),
            "temporal_queue": request.temporal_queue,
            }),
            extra_payload,
        )),
        available_at: now,
        published_at: None,
        created_at: now,
    };
    repository
        .insert_outbox_event(&outbox_row)
        .await
        .map_err(json_db_error)?;

    Ok(())
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

fn site_summary_from_row(row: cms_db::models::SiteRow) -> Result<SiteSummary, String> {
    Ok(SiteSummary {
        id: row.id,
        name: row.name,
        slug: row.slug,
        primary_host: row.primary_host,
        site_kind: parse_site_kind(&row.site_kind)?,
        source_template_site_id: row.source_template_site_id,
    })
}

fn site_kind_name(kind: SiteKind) -> &'static str {
    match kind {
        SiteKind::Standard => "standard",
        SiteKind::Template => "template",
    }
}

fn branch_summary_from_row(row: cms_db::models::BranchRow) -> BranchSummary {
    BranchSummary {
        site_id: row.site_id,
        name: row.name,
        head_snapshot_id: row.head_snapshot_id,
    }
}

fn widget_definition_from_row(
    row: cms_db::models::WidgetDefinitionRow,
) -> Result<WidgetDefinition, String> {
    Ok(WidgetDefinition {
        id: row.id,
        slug: row.slug,
        display_name: row.display_name,
        source_kind: parse_widget_source_kind(&row.source_kind)?,
        component_source_id: row.component_source_id,
        description: row.description,
        is_primitive: row.is_primitive,
    })
}

fn widget_definition_version_from_row(
    row: cms_db::models::WidgetDefinitionVersionRow,
) -> Result<WidgetDefinitionVersion, String> {
    Ok(WidgetDefinitionVersion {
        id: row.id,
        definition_id: row.widget_definition_id,
        version: row.version,
        runtime: parse_widget_runtime(&row.runtime)?,
        html_support_mode: parse_html_support_mode(&row.html_support_mode)?,
        settings_schema: row.settings_schema.0,
        editor_schema: row.editor_schema.0,
        asset_manifest: row.asset_manifest.0,
        supports_server_render: row.supports_server_render,
    })
}

fn parse_site_kind(value: &str) -> Result<SiteKind, String> {
    match normalize_name(value).as_str() {
        "standard" => Ok(SiteKind::Standard),
        "template" => Ok(SiteKind::Template),
        _ => Err(format!("unknown site kind: {value}")),
    }
}

fn parse_widget_source_kind(value: &str) -> Result<WidgetSourceKind, String> {
    match normalize_name(value).as_str() {
        "builtin" => Ok(WidgetSourceKind::Builtin),
        "registryrepo" | "registry_repo" => Ok(WidgetSourceKind::RegistryRepo),
        _ => Err(format!("unknown widget source kind: {value}")),
    }
}

fn parse_widget_runtime(value: &str) -> Result<WidgetRuntime, String> {
    match normalize_name(value).as_str() {
        "servertemplate" | "server_template" => Ok(WidgetRuntime::ServerTemplate),
        "svelte" => Ok(WidgetRuntime::Svelte),
        "react" => Ok(WidgetRuntime::React),
        "vue" => Ok(WidgetRuntime::Vue),
        "webcomponent" | "web_component" => Ok(WidgetRuntime::WebComponent),
        "rawjavascript" | "raw_javascript" => Ok(WidgetRuntime::RawJavascript),
        _ => Err(format!("unknown widget runtime: {value}")),
    }
}

fn parse_html_support_mode(value: &str) -> Result<HtmlSupportMode, String> {
    match normalize_name(value).as_str() {
        "none" => Ok(HtmlSupportMode::None),
        "sanitizedfragment" | "sanitized_fragment" => Ok(HtmlSupportMode::SanitizedFragment),
        "trustedfragment" | "trusted_fragment" => Ok(HtmlSupportMode::TrustedFragment),
        _ => Err(format!("unknown html support mode: {value}")),
    }
}

fn normalize_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| *character != '-' && !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn merge_json(base: serde_json::Value, extra: Option<serde_json::Value>) -> serde_json::Value {
    match (base, extra) {
        (serde_json::Value::Object(mut base_map), Some(serde_json::Value::Object(extra_map))) => {
            base_map.extend(extra_map);
            serde_json::Value::Object(base_map)
        }
        (base_value, _) => base_value,
    }
}

fn workflow_kind_name(kind: WorkflowKind) -> &'static str {
    match kind {
        WorkflowKind::SitePublish => "site_publish",
        WorkflowKind::RestoreSnapshot => "restore_snapshot",
        WorkflowKind::BulkApplySnapshot => "bulk_apply_snapshot",
        WorkflowKind::SiteMigration => "site_migration",
        WorkflowKind::AiContentOperation => "ai_content_operation",
    }
}

fn agent_runtime_name(runtime: AgentRuntime) -> &'static str {
    match runtime {
        AgentRuntime::Rust => "rust",
        AgentRuntime::BunTypescript => "bun_typescript",
        AgentRuntime::Python => "python",
    }
}

fn internal_db_error(error: sqlx::Error) -> StatusCode {
    error!(error = %error, "database query failed");
    db_error_status(&error)
}

fn json_db_error(error: sqlx::Error) -> (StatusCode, Json<serde_json::Value>) {
    error!(error = %error, "database query failed");
    (
        db_error_status(&error),
        Json(serde_json::json!({
            "error": db_error_message(&error)
        })),
    )
}

fn json_mapping_error(error: String) -> (StatusCode, Json<serde_json::Value>) {
    error!(error = %error, "value mapping failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "value mapping failed"
        })),
    )
}

fn internal_mapping_error(error: String) -> StatusCode {
    error!(error = %error, "database value mapping failed");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn db_error_status(error: &sqlx::Error) -> StatusCode {
    match error {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        sqlx::Error::Database(database_error) => match database_error.code().as_deref() {
            Some("23505") => StatusCode::CONFLICT,
            Some("23503") | Some("23502") | Some("23514") => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn db_error_message(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::RowNotFound => "record was not found".to_owned(),
        sqlx::Error::Database(database_error) => {
            let code = database_error
                .code()
                .map(|code| code.into_owned())
                .unwrap_or_default();
            let detail = database_error
                .message()
                .trim()
                .trim_end_matches('.')
                .to_owned();
            match code.as_str() {
                "23505" => format!("duplicate value violates a unique constraint: {detail}"),
                "23503" => format!("referenced record is missing: {detail}"),
                "23502" => format!("required field is missing: {detail}"),
                "23514" => format!("database check constraint failed: {detail}"),
                _ if !detail.is_empty() => detail,
                _ => "database query failed".to_owned(),
            }
        }
        _ => "database query failed".to_owned(),
    }
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

async fn load_temporal_workflow_result(
    config: &AppConfig,
    workflow_id: &str,
) -> Result<TemporalWorkflowResult, String> {
    let mut child = Command::new(&config.temporal_runner_python);
    child
        .arg(&config.temporal_runner_result_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = child
        .spawn()
        .map_err(|error| format!("failed to spawn temporal result script: {error}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::json!({
            "workflow_id": workflow_id,
            "timeout_seconds": 2.0
        });
        stdin
            .write_all(payload.to_string().as_bytes())
            .await
            .map_err(|error| format!("failed to write workflow id to temporal script: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|error| format!("failed waiting for temporal result script: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("temporal result script failed: {}", stderr.trim()));
    }

    serde_json::from_slice::<TemporalWorkflowResult>(&output.stdout)
        .map_err(|error| format!("failed to decode temporal result output: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use std::collections::HashMap;
    use tower::ServiceExt;

    fn state() -> AppState {
        AppState {
            config: AppConfig::for_tests(),
            renderer: RenderEngine,
            workflows: WorkflowRuntimeMatrix::default(),
            catalog: Arc::new(ApiCatalog::default()),
            migrations: Arc::new(RwLock::new(HashMap::new())),
            repository: None,
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

    #[tokio::test]
    async fn create_migration_stores_scaffolded_review_record() {
        let site_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
        let response = build_router(state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/sites/{site_id}/migrations"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "target_site_id": site_id,
                            "homepage_url": "https://example.com/",
                            "client_id": "cccccccc-cccc-cccc-cccc-cccccccccccc",
                            "location_id": "dddddddd-dddd-dddd-dddd-dddddddddddd",
                            "options": {
                                "crawl_scope": "subpath",
                                "follow_subdomains": false,
                                "max_pages": 25,
                                "respect_robots": true,
                                "include_assets": true,
                                "detect_registered_widgets": true,
                                "use_legacy_api_enrichment": false,
                                "screenshot_compare_after_import": false
                            }
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn create_migration_without_target_is_rejected() {
        let response = build_router(state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/migrations")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "homepage_url": "https://example.com/",
                            "client_id": "cccccccc-cccc-cccc-cccc-cccccccccccc",
                            "location_id": "dddddddd-dddd-dddd-dddd-dddddddddddd",
                            "options": {
                                "crawl_scope": "subpath",
                                "follow_subdomains": false,
                                "max_pages": 25,
                                "respect_robots": true,
                                "include_assets": true,
                                "detect_registered_widgets": true,
                                "use_legacy_api_enrichment": false,
                                "screenshot_compare_after_import": false
                            }
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn import_draft_requires_database_configuration() {
        let response = build_router(state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/migrations/aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/import-draft")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn imported_page_draft_uses_last_path_segment_as_slug() {
        let record = MigrationJobRecord {
            id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            site_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            workflow_request_id: Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
            workflow_id: "workflow-1".to_owned(),
            branch_name: "migration/draft".to_owned(),
            homepage_url: "https://example.com/".to_owned(),
            client_id: Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap(),
            location_id: Uuid::parse_str("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee").unwrap(),
            legacy_api_profile: None,
            status: MigrationStatus::ReviewReady,
            options: MigrationOptions {
                crawl_scope: MigrationCrawlScope::Subpath,
                follow_subdomains: false,
                max_pages: 10,
                respect_robots: true,
                include_assets: true,
                detect_registered_widgets: true,
                use_legacy_api_enrichment: false,
                screenshot_compare_after_import: false,
            },
            pages: Vec::new(),
            warnings: Vec::new(),
            created_at: OffsetDateTime::now_utc(),
            approved_at: None,
        };
        let page = MigrationPageDetail {
            id: Uuid::parse_str("ffffffff-ffff-ffff-ffff-ffffffffffff").unwrap(),
            path: "/apartments/vt/shelburne/floor-plans".to_owned(),
            title_guess: "Floor Plans".to_owned(),
            widget_matches: vec!["floor-plans-plus".to_owned()],
            unknown_regions: 2,
            confidence: 0.92,
            warnings: vec![],
            extraction_notes: vec![],
        };

        let imported = imported_page_draft_from_migration_page(&record, &page, None);
        assert_eq!(imported.slug, "floor-plans");
        assert!(imported.summary.contains("floor-plans-plus"));
        assert!(imported.schema_types.is_empty());
    }

    #[test]
    fn workflow_kind_name_includes_site_migration() {
        assert_eq!(
            workflow_kind_name(WorkflowKind::SiteMigration),
            "site_migration"
        );
    }
}
