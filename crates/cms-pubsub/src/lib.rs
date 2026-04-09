use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PubSubMessage {
    pub topic: String,
    pub key: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum PubSubError {
    #[error("publish is not available")]
    PublishUnavailable,
}

#[async_trait]
pub trait PubSub: Send + Sync {
    fn backend_name(&self) -> &'static str;

    async fn publish(&self, message: PubSubMessage) -> Result<(), PubSubError>;
}

#[derive(Debug, Default, Clone)]
pub struct MemoryPubSub;

#[async_trait]
impl PubSub for MemoryPubSub {
    fn backend_name(&self) -> &'static str {
        "memory"
    }

    async fn publish(&self, _message: PubSubMessage) -> Result<(), PubSubError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryPubSub, PubSub, PubSubMessage};

    #[tokio::test]
    async fn memory_pubsub_accepts_publish() {
        let pubsub = MemoryPubSub;
        let result = pubsub
            .publish(PubSubMessage {
                topic: "publish.completed".to_owned(),
                key: "site-1".to_owned(),
                payload: serde_json::json!({ "ok": true }),
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(pubsub.backend_name(), "memory");
    }
}

