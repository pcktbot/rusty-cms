use blake3::Hasher;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {
    use super::PublishRequest;
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
}
