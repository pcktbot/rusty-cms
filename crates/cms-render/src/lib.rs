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

    pub fn render_preview_document(
        &self,
        snapshot: &SiteSnapshotRef,
    ) -> Result<String, RenderError> {
        let marker = self.render_snapshot(snapshot)?;

        Ok(format!(
            r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>CMS Preview</title>
    <style>
      :root {{
        color-scheme: light;
        --bg: #f5efe4;
        --paper: #fffdf8;
        --ink: #1f1c18;
        --accent: #a55233;
        --muted: #7d7165;
        --line: #dfd4c4;
      }}
      * {{ box-sizing: border-box; }}
      body {{
        margin: 0;
        font-family: Georgia, "Times New Roman", serif;
        color: var(--ink);
        background:
          radial-gradient(circle at top left, rgba(165, 82, 51, 0.15), transparent 30%),
          linear-gradient(180deg, #f6f1e8, #ebe1d2);
      }}
      main {{
        max-width: 960px;
        margin: 0 auto;
        padding: 48px 24px 80px;
      }}
      .eyebrow {{
        letter-spacing: 0.16em;
        text-transform: uppercase;
        color: var(--accent);
        font-size: 0.82rem;
      }}
      .hero {{
        background: var(--paper);
        border: 1px solid var(--line);
        border-radius: 24px;
        padding: 40px;
        box-shadow: 0 20px 60px rgba(60, 35, 22, 0.08);
      }}
      h1 {{
        margin: 12px 0 16px;
        font-size: clamp(2.5rem, 5vw, 4.5rem);
        line-height: 0.95;
      }}
      p {{
        font-size: 1.05rem;
        line-height: 1.7;
        max-width: 58ch;
      }}
      .meta {{
        margin-top: 32px;
        padding-top: 24px;
        border-top: 1px solid var(--line);
        color: var(--muted);
        font-family: "SFMono-Regular", ui-monospace, monospace;
        font-size: 0.92rem;
      }}
      .grid {{
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 16px;
        margin-top: 28px;
      }}
      .card {{
        background: rgba(255, 253, 248, 0.7);
        border: 1px solid var(--line);
        border-radius: 18px;
        padding: 18px;
      }}
      .card strong {{
        display: block;
        margin-bottom: 6px;
      }}
    </style>
  </head>
  <body>
    <main>
      <section class="hero">
        <div class="eyebrow">Server Render Preview</div>
        <h1>Snapshot-first page rendering.</h1>
        <p>
          This is a temporary preview surface served directly by the Rust API.
          The goal is to make render and publish contracts visible before the
          full Bun and Svelte management UI lands.
        </p>
        <div class="grid">
          <div class="card">
            <strong>Render model</strong>
            One immutable snapshot in, one deterministic page out.
          </div>
          <div class="card">
            <strong>Publish model</strong>
            Build to release directories and promote atomically.
          </div>
          <div class="card">
            <strong>Workflow model</strong>
            Temporal orchestrates; the CMS validates inputs and outputs.
          </div>
        </div>
        <div class="meta">{marker}</div>
      </section>
    </main>
  </body>
</html>"#
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
