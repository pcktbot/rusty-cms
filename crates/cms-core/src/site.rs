use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SiteSnapshotRef {
    pub site_id: Uuid,
    pub branch_name: String,
    pub snapshot_id: Uuid,
}

impl SiteSnapshotRef {
    pub fn new(site_id: Uuid, branch_name: impl Into<String>, snapshot_id: Uuid) -> Self {
        Self {
            site_id,
            branch_name: branch_name.into(),
            snapshot_id,
        }
    }
}

