use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct SiteRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub slug: String,
    pub primary_host: String,
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
pub struct OutboxEventRow {
    pub id: Uuid,
    pub topic: String,
    pub event_key: String,
    pub payload: Json<serde_json::Value>,
    pub available_at: OffsetDateTime,
    pub published_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}
