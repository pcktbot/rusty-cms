use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum AgentRuntime {
    Rust,
    BunTypescript,
    Python,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum WorkflowKind {
    SitePublish,
    RestoreSnapshot,
    BulkApplySnapshot,
    SiteMigration,
    AiContentOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorkflowDefinition {
    pub kind: WorkflowKind,
    pub name: String,
    pub temporal_queue: String,
    pub accepted_runtimes: Vec<AgentRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorkflowSafetyPolicy {
    pub requires_human_approval: bool,
    pub max_sites_touched: u32,
    pub allow_publish_side_effects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorkflowArtifactContract {
    pub output_schema: String,
    pub creates_snapshot: bool,
    pub mutates_branch_head: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorkflowRequest {
    pub id: Uuid,
    pub kind: WorkflowKind,
    pub site_id: Uuid,
    pub branch_name: String,
    pub requested_runtime: AgentRuntime,
    pub temporal_queue: String,
    pub input_payload: serde_json::Value,
    pub artifact_contract: WorkflowArtifactContract,
    pub safety_policy: WorkflowSafetyPolicy,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkflowAdmissionError {
    #[error("workflow kind {kind:?} is not registered")]
    UnknownWorkflowKind { kind: WorkflowKind },
    #[error("runtime {runtime:?} is not allowed for workflow kind {kind:?}")]
    RuntimeNotAllowed {
        runtime: AgentRuntime,
        kind: WorkflowKind,
    },
}

#[derive(Debug, Clone)]
pub struct WorkflowRuntimeMatrix {
    definitions: Vec<WorkflowDefinition>,
}

impl Default for WorkflowRuntimeMatrix {
    fn default() -> Self {
        Self {
            definitions: vec![
                WorkflowDefinition {
                    kind: WorkflowKind::SitePublish,
                    name: "site-publish".to_owned(),
                    temporal_queue: "cms-publish".to_owned(),
                    accepted_runtimes: vec![AgentRuntime::Rust],
                },
                WorkflowDefinition {
                    kind: WorkflowKind::RestoreSnapshot,
                    name: "restore-snapshot".to_owned(),
                    temporal_queue: "cms-restore".to_owned(),
                    accepted_runtimes: vec![AgentRuntime::Rust],
                },
                WorkflowDefinition {
                    kind: WorkflowKind::BulkApplySnapshot,
                    name: "bulk-apply-snapshot".to_owned(),
                    temporal_queue: "cms-bulk".to_owned(),
                    accepted_runtimes: vec![AgentRuntime::Rust],
                },
                WorkflowDefinition {
                    kind: WorkflowKind::SiteMigration,
                    name: "site-migration".to_owned(),
                    temporal_queue: "cms-migrations".to_owned(),
                    accepted_runtimes: vec![AgentRuntime::Python, AgentRuntime::BunTypescript],
                },
                WorkflowDefinition {
                    kind: WorkflowKind::AiContentOperation,
                    name: "ai-content-operation".to_owned(),
                    temporal_queue: "cms-agent-ops".to_owned(),
                    accepted_runtimes: vec![
                        AgentRuntime::Rust,
                        AgentRuntime::BunTypescript,
                        AgentRuntime::Python,
                    ],
                },
            ],
        }
    }
}

impl WorkflowRuntimeMatrix {
    pub fn supported_runtimes(&self) -> Vec<AgentRuntime> {
        let mut runtimes = Vec::new();
        for definition in &self.definitions {
            for runtime in &definition.accepted_runtimes {
                if !runtimes.contains(runtime) {
                    runtimes.push(*runtime);
                }
            }
        }
        runtimes.sort_by_key(|runtime| match runtime {
            AgentRuntime::Rust => 0,
            AgentRuntime::BunTypescript => 1,
            AgentRuntime::Python => 2,
        });
        runtimes
    }

    pub fn definitions(&self) -> &[WorkflowDefinition] {
        &self.definitions
    }

    pub fn definition_for_kind(&self, kind: WorkflowKind) -> Option<&WorkflowDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.kind == kind)
    }

    pub fn admit(
        &self,
        request: &WorkflowRequest,
    ) -> Result<&WorkflowDefinition, WorkflowAdmissionError> {
        let definition = self
            .definition_for_kind(request.kind)
            .ok_or(WorkflowAdmissionError::UnknownWorkflowKind { kind: request.kind })?;

        if definition
            .accepted_runtimes
            .contains(&request.requested_runtime)
        {
            Ok(definition)
        } else {
            Err(WorkflowAdmissionError::RuntimeNotAllowed {
                runtime: request.requested_runtime,
                kind: request.kind,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentRuntime, WorkflowAdmissionError, WorkflowArtifactContract, WorkflowKind,
        WorkflowRequest, WorkflowRuntimeMatrix, WorkflowSafetyPolicy,
    };
    use uuid::Uuid;

    #[test]
    fn matrix_deduplicates_supported_runtimes() {
        let matrix = WorkflowRuntimeMatrix::default();

        assert_eq!(
            matrix.supported_runtimes(),
            vec![
                AgentRuntime::Rust,
                AgentRuntime::BunTypescript,
                AgentRuntime::Python,
            ]
        );
    }

    #[test]
    fn matrix_admits_allowed_runtime() {
        let matrix = WorkflowRuntimeMatrix::default();
        let request = WorkflowRequest {
            id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            kind: WorkflowKind::SiteMigration,
            site_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            branch_name: "migration/draft".to_owned(),
            requested_runtime: AgentRuntime::Python,
            temporal_queue: "cms-migrations".to_owned(),
            input_payload: serde_json::json!({ "homepage_url": "https://example.com" }),
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
        };

        let definition = matrix.admit(&request).unwrap();
        assert_eq!(definition.temporal_queue, "cms-migrations");
    }

    #[test]
    fn matrix_rejects_runtime_not_allowed() {
        let matrix = WorkflowRuntimeMatrix::default();
        let request = WorkflowRequest {
            id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            kind: WorkflowKind::SitePublish,
            site_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            branch_name: "production".to_owned(),
            requested_runtime: AgentRuntime::Python,
            temporal_queue: "cms-publish".to_owned(),
            input_payload: serde_json::json!({ "publish": true }),
            artifact_contract: WorkflowArtifactContract {
                output_schema: "schemas/publish-output.json".to_owned(),
                creates_snapshot: false,
                mutates_branch_head: false,
            },
            safety_policy: WorkflowSafetyPolicy {
                requires_human_approval: false,
                max_sites_touched: 1,
                allow_publish_side_effects: true,
            },
        };

        let error = matrix.admit(&request).unwrap_err();
        assert_eq!(
            error,
            WorkflowAdmissionError::RuntimeNotAllowed {
                runtime: AgentRuntime::Python,
                kind: WorkflowKind::SitePublish,
            }
        );
    }
}
