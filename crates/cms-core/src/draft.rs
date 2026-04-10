use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DraftChangeSetSourceKind {
    MigrationImport,
    ManualEdits,
    TemplateSync,
    BulkOperation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DraftChangeSetStatus {
    Open,
    PreviewReady,
    Published,
    Discarded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DraftChangeKind {
    UpsertPageShell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DraftResourceKind {
    Page,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DraftChangeStatus {
    Pending,
    Selected,
    Imported,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DraftPreviewRef {
    pub site_id: Uuid,
    pub branch_name: String,
    pub change_set_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DraftChangeSet {
    pub id: Uuid,
    pub site_id: Uuid,
    pub branch_name: String,
    pub base_snapshot_id: Option<Uuid>,
    pub source_kind: DraftChangeSetSourceKind,
    pub status: DraftChangeSetStatus,
    pub name: String,
    pub description: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DraftChange {
    pub id: Uuid,
    pub change_set_id: Uuid,
    pub site_id: Uuid,
    pub page_id: Option<Uuid>,
    pub migration_job_id: Option<Uuid>,
    pub migration_page_id: Option<Uuid>,
    pub change_kind: DraftChangeKind,
    pub resource_kind: DraftResourceKind,
    pub resource_key: String,
    pub status: DraftChangeStatus,
    pub title: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ImportedPageDraft {
    pub migration_job_id: Uuid,
    pub migration_page_id: Uuid,
    pub path: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub widget_matches: Vec<String>,
    pub warnings: Vec<String>,
    pub extraction_notes: Vec<String>,
    pub unknown_regions: u32,
    pub seo: Value,
    pub schema_types: Vec<String>,
    pub layout: Value,
    pub text_blocks: Vec<String>,
    pub html_excerpt: Option<String>,
    pub document_candidate: Value,
}

#[cfg(test)]
mod tests {
    use super::{DraftChangeSetSourceKind, DraftChangeSetStatus};

    #[test]
    fn draft_status_serializes_to_snake_case() {
        let value = serde_json::to_string(&DraftChangeSetStatus::PreviewReady).unwrap();
        assert_eq!(value, "\"preview_ready\"");
    }

    #[test]
    fn draft_source_kind_serializes_to_snake_case() {
        let value = serde_json::to_string(&DraftChangeSetSourceKind::MigrationImport).unwrap();
        assert_eq!(value, "\"migration_import\"");
    }
}
