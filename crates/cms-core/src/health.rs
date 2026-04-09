use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct HealthStatus {
    pub service: String,
    pub status: String,
}

impl HealthStatus {
    pub fn ok(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            status: "ok".to_owned(),
        }
    }
}
