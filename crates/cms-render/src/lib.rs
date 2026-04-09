use cms_core::site::SiteSnapshotRef;
use thiserror::Error;

#[derive(Debug, Default, Clone)]
pub struct RenderEngine;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("invalid page document")]
    InvalidPageDocument,
}

impl RenderEngine {
    pub fn name(&self) -> &'static str {
        "snapshot-renderer"
    }

    pub fn render_snapshot(&self, snapshot: &SiteSnapshotRef) -> Result<String, RenderError> {
        Ok(format!(
            "<!-- site:{} branch:{} snapshot:{} -->",
            snapshot.site_id, snapshot.branch_name, snapshot.snapshot_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::RenderEngine;
    use cms_core::site::SiteSnapshotRef;
    use uuid::Uuid;

    #[test]
    fn renderer_includes_snapshot_identity() {
        let renderer = RenderEngine;
        let snapshot = SiteSnapshotRef::new(
            Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            "draft",
            Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
        );

        let html = renderer.render_snapshot(&snapshot).unwrap();
        assert!(html.contains("branch:draft"));
        assert!(html.contains("snapshot:bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"));
    }
}

