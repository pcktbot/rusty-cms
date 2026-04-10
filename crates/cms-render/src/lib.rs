use cms_core::{
    draft::{DraftPreviewRef, ImportedPageDraft},
    site::SiteSnapshotRef,
};
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

    pub fn render_imported_page_preview(
        &self,
        preview: &DraftPreviewRef,
        page: &ImportedPageDraft,
    ) -> Result<String, RenderError> {
        let widget_matches = if page.widget_matches.is_empty() {
            "<li>no registered widgets detected yet</li>".to_owned()
        } else {
            page.widget_matches
                .iter()
                .map(|item| format!("<li>{item}</li>"))
                .collect::<Vec<_>>()
                .join("")
        };

        let warnings = if page.warnings.is_empty() {
            "<li>no migration warnings</li>".to_owned()
        } else {
            page.warnings
                .iter()
                .map(|item| format!("<li>{item}</li>"))
                .collect::<Vec<_>>()
                .join("")
        };

        let notes = if page.extraction_notes.is_empty() {
            "<li>discovery produced metadata only; DOM import is not wired yet</li>".to_owned()
        } else {
            page.extraction_notes
                .iter()
                .map(|item| format!("<li>{item}</li>"))
                .collect::<Vec<_>>()
                .join("")
        };

        Ok(format!(
            r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <style>
      :root {{
        --bg: #f3f0e8;
        --paper: #fffdf8;
        --ink: #111111;
        --muted: #555555;
        --line: #111111;
        --accent: #d64a1f;
      }}
      * {{ box-sizing: border-box; }}
      body {{
        margin: 0;
        background: var(--bg);
        color: var(--ink);
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      }}
      main {{
        max-width: 980px;
        margin: 0 auto;
        padding: 28px 18px 72px;
      }}
      .hero,
      .panel {{
        background: var(--paper);
        border: 2px solid var(--line);
        box-shadow: 8px 8px 0 var(--line);
        padding: 18px;
      }}
      .hero {{
        margin-bottom: 16px;
      }}
      .eyebrow {{
        font-size: 0.78rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--accent);
      }}
      h1, h2, p, ul {{
        margin: 0;
      }}
      h1 {{
        margin-top: 10px;
        font-size: 2rem;
        line-height: 1;
      }}
      .grid {{
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 14px;
        margin-top: 16px;
      }}
      .meta {{
        margin-top: 14px;
        color: var(--muted);
      }}
      ul {{
        padding-left: 18px;
        line-height: 1.5;
      }}
      .pill {{
        display: inline-block;
        padding: 4px 8px;
        border: 2px solid var(--line);
        margin: 6px 8px 0 0;
        background: #fff4e8;
      }}
    </style>
  </head>
  <body>
    <main>
      <section class="hero">
        <div class="eyebrow">Imported Draft Preview</div>
        <h1>{title}</h1>
        <p class="meta">path: {path}</p>
        <p class="meta">change_set: {change_set_id}</p>
        <p class="meta">site: {site_id} | branch: {branch_name}</p>
        <p class="meta">{summary}</p>
      </section>

      <section class="grid">
        <article class="panel">
          <h2>Widget signals</h2>
          <div>{widget_badges}</div>
          <ul>{widget_matches}</ul>
        </article>
        <article class="panel">
          <h2>Warnings</h2>
          <ul>{warnings}</ul>
        </article>
        <article class="panel">
          <h2>Import notes</h2>
          <ul>{notes}</ul>
        </article>
        <article class="panel">
          <h2>Unknown regions</h2>
          <p>{unknown_regions}</p>
        </article>
      </section>
    </main>
  </body>
</html>"#,
            title = page.title,
            path = page.path,
            change_set_id = preview.change_set_id,
            site_id = preview.site_id,
            branch_name = preview.branch_name,
            summary = page.summary,
            widget_badges = page
                .widget_matches
                .iter()
                .map(|item| format!("<span class=\"pill\">{item}</span>"))
                .collect::<Vec<_>>()
                .join(""),
            widget_matches = widget_matches,
            warnings = warnings,
            notes = notes,
            unknown_regions = page.unknown_regions,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::RenderEngine;
    use cms_core::{
        draft::{DraftPreviewRef, ImportedPageDraft},
        site::SiteSnapshotRef,
    };
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

    #[test]
    fn imported_page_preview_includes_change_set_identity() {
        let renderer = RenderEngine;
        let preview = DraftPreviewRef {
            site_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            branch_name: "migration/draft".to_owned(),
            change_set_id: Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
        };
        let page = ImportedPageDraft {
            migration_job_id: Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap(),
            migration_page_id: Uuid::parse_str("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee").unwrap(),
            path: "/".to_owned(),
            slug: "home".to_owned(),
            title: "Hearth".to_owned(),
            summary: "Initial imported shell.".to_owned(),
            widget_matches: vec!["hero-banner".to_owned()],
            warnings: vec![],
            extraction_notes: vec![],
            unknown_regions: 3,
        };

        let html = renderer
            .render_imported_page_preview(&preview, &page)
            .unwrap();
        assert!(html.contains("Imported Draft Preview"));
        assert!(html.contains("change_set: cccccccc-cccc-cccc-cccc-cccccccccccc"));
        assert!(html.contains("hero-banner"));
    }
}
