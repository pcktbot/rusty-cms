use blake3::Hasher;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PublishRequest {
    pub site_id: Uuid,
    pub snapshot_id: Uuid,
    pub target_dir: String,
}

impl PublishRequest {
    pub fn content_key(&self) -> String {
        let mut hasher = Hasher::new();
        hasher.update(self.site_id.as_bytes());
        hasher.update(self.snapshot_id.as_bytes());
        hasher.update(self.target_dir.as_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum PublishJobState {
    Queued,
    Snapshotting,
    Building,
    Validating,
    Promoting,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PublishJob {
    pub id: Uuid,
    pub request: PublishRequest,
    pub state: PublishJobState,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PublishStateError {
    #[error("invalid publish state transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: PublishJobState,
        to: PublishJobState,
    },
}

impl PublishJob {
    pub fn new(request: PublishRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            request,
            state: PublishJobState::Queued,
        }
    }

    pub fn advance(self, next: PublishJobState) -> Result<Self, PublishStateError> {
        if self.state.can_transition_to(next) {
            Ok(Self {
                state: next,
                ..self
            })
        } else {
            Err(PublishStateError::InvalidTransition {
                from: self.state,
                to: next,
            })
        }
    }
}

impl PublishJobState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use PublishJobState as S;

        matches!(
            (self, next),
            (S::Queued, S::Snapshotting)
                | (S::Queued, S::Cancelled)
                | (S::Snapshotting, S::Building)
                | (S::Snapshotting, S::Failed)
                | (S::Building, S::Validating)
                | (S::Building, S::Failed)
                | (S::Validating, S::Promoting)
                | (S::Validating, S::Failed)
                | (S::Promoting, S::Completed)
                | (S::Promoting, S::Failed)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{PublishJob, PublishJobState, PublishRequest, PublishStateError};
    use uuid::Uuid;

    #[test]
    fn publish_request_hash_is_stable() {
        let request = PublishRequest {
            site_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            snapshot_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            target_dir: "/srv/sites/example".to_owned(),
        };

        assert_eq!(
            request.content_key(),
            "df145d94b590028afdd1114b0f41568e77baaeb08d6e53afce9c8383f07bf4a5"
        );
    }

    #[test]
    fn publish_job_allows_happy_path_transitions() {
        let request = PublishRequest {
            site_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            snapshot_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            target_dir: "/srv/sites/example".to_owned(),
        };

        let job = PublishJob::new(request)
            .advance(PublishJobState::Snapshotting)
            .unwrap()
            .advance(PublishJobState::Building)
            .unwrap()
            .advance(PublishJobState::Validating)
            .unwrap()
            .advance(PublishJobState::Promoting)
            .unwrap()
            .advance(PublishJobState::Completed)
            .unwrap();

        assert_eq!(job.state, PublishJobState::Completed);
    }

    #[test]
    fn publish_job_rejects_invalid_transition() {
        let request = PublishRequest {
            site_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            snapshot_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            target_dir: "/srv/sites/example".to_owned(),
        };

        let error = PublishJob::new(request)
            .advance(PublishJobState::Completed)
            .unwrap_err();

        assert_eq!(
            error,
            PublishStateError::InvalidTransition {
                from: PublishJobState::Queued,
                to: PublishJobState::Completed,
            }
        );
    }
}
