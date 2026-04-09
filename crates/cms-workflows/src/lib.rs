use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum AgentRuntime {
    Rust,
    BunTypescript,
    Python,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorkflowDefinition {
    pub name: String,
    pub temporal_queue: String,
    pub accepted_runtimes: Vec<AgentRuntime>,
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
                    name: "site-publish".to_owned(),
                    temporal_queue: "cms-publish".to_owned(),
                    accepted_runtimes: vec![AgentRuntime::Rust],
                },
                WorkflowDefinition {
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
        runtimes
    }

    pub fn definitions(&self) -> &[WorkflowDefinition] {
        &self.definitions
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentRuntime, WorkflowRuntimeMatrix};

    #[test]
    fn matrix_deduplicates_supported_runtimes() {
        let matrix = WorkflowRuntimeMatrix::default();

        assert_eq!(
            matrix.supported_runtimes(),
            vec![
                AgentRuntime::Rust,
                AgentRuntime::BunTypescript,
                AgentRuntime::Python
            ]
        );
    }
}

