use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SiteKind {
    Standard,
    Template,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SiteTemplateBinding {
    pub site_id: Uuid,
    pub kind: SiteKind,
    pub source_template_site_id: Option<Uuid>,
}

impl SiteTemplateBinding {
    pub fn template(site_id: Uuid) -> Self {
        Self {
            site_id,
            kind: SiteKind::Template,
            source_template_site_id: None,
        }
    }

    pub fn derived(site_id: Uuid, source_template_site_id: Uuid) -> Self {
        Self {
            site_id,
            kind: SiteKind::Standard,
            source_template_site_id: Some(source_template_site_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SiteKind, SiteTemplateBinding};
    use uuid::Uuid;

    #[test]
    fn template_site_binding_has_no_source_template() {
        let binding = SiteTemplateBinding::template(
            Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
        );

        assert_eq!(binding.kind, SiteKind::Template);
        assert_eq!(binding.source_template_site_id, None);
    }

    #[test]
    fn derived_site_binding_points_to_template() {
        let binding = SiteTemplateBinding::derived(
            Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
        );

        assert_eq!(binding.kind, SiteKind::Standard);
        assert_eq!(
            binding.source_template_site_id,
            Some(Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap())
        );
    }
}
