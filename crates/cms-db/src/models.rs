use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct AccountRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct SiteRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub slug: String,
    pub primary_host: String,
    pub site_kind: String,
    pub source_template_site_id: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct BranchRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub name: String,
    pub head_snapshot_id: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct SnapshotRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub branch_id: Uuid,
    pub label: String,
    pub created_by: String,
    pub manifest: Json<serde_json::Value>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct PageRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub path: String,
    pub slug: String,
    pub title: String,
    pub position: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct PageDocumentRow {
    pub id: Uuid,
    pub page_id: Uuid,
    pub snapshot_id: Uuid,
    pub schema_version: i32,
    pub document: Json<serde_json::Value>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct PublishJobRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub snapshot_id: Uuid,
    pub state: String,
    pub target_dir: String,
    pub requested_by: String,
    pub release_key: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct WorkflowRequestRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub branch_name: String,
    pub workflow_kind: String,
    pub requested_runtime: String,
    pub temporal_queue: String,
    pub input_payload: Json<serde_json::Value>,
    pub output_schema: String,
    pub requires_human_approval: bool,
    pub max_sites_touched: i32,
    pub allow_publish_side_effects: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct MigrationJobRow {
    pub id: Uuid,
    pub site_id: Uuid,
    pub workflow_request_id: Uuid,
    pub workflow_id: String,
    pub branch_name: String,
    pub homepage_url: String,
    pub client_id: Uuid,
    pub location_id: Uuid,
    pub legacy_api_profile: Option<String>,
    pub status: String,
    pub options: Json<serde_json::Value>,
    pub warnings: Json<Vec<String>>,
    pub created_at: OffsetDateTime,
    pub approved_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct MigrationPageRow {
    pub id: Uuid,
    pub migration_job_id: Uuid,
    pub path: String,
    pub title_guess: String,
    pub widget_matches: Json<Vec<String>>,
    pub unknown_regions: i32,
    pub confidence: f32,
    pub warnings: Json<Vec<String>>,
    pub extraction_notes: Json<Vec<String>>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct OutboxEventRow {
    pub id: Uuid,
    pub topic: String,
    pub event_key: String,
    pub payload: Json<serde_json::Value>,
    pub available_at: OffsetDateTime,
    pub published_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct ComponentSourceRow {
    pub id: Uuid,
    pub slug: String,
    pub source_kind: String,
    pub repo_url: Option<String>,
    pub default_ref: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct WidgetDefinitionRow {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub source_kind: String,
    pub component_source_id: Option<Uuid>,
    pub description: Option<String>,
    pub is_primitive: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct WidgetDefinitionVersionRow {
    pub id: Uuid,
    pub widget_definition_id: Uuid,
    pub version: String,
    pub runtime: String,
    pub html_support_mode: String,
    pub settings_schema: Json<serde_json::Value>,
    pub editor_schema: Json<serde_json::Value>,
    pub asset_manifest: Json<serde_json::Value>,
    pub supports_server_render: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct WidgetInstanceRow {
    pub id: Uuid,
    pub page_id: Uuid,
    pub snapshot_id: Uuid,
    pub parent_widget_instance_id: Option<Uuid>,
    pub region: String,
    pub position: i32,
    pub widget_definition_id: Uuid,
    pub widget_definition_version_id: Uuid,
    pub settings: Json<serde_json::Value>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
