use cms_core::site::SiteKind;
use cms_core::widget::{
    HtmlSupportMode, WidgetDefinition, WidgetDefinitionVersion, WidgetRuntime, WidgetSourceKind,
};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct SiteSummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub primary_host: String,
    pub site_kind: SiteKind,
    pub source_template_site_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchSummary {
    pub site_id: Uuid,
    pub name: String,
    pub head_snapshot_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ApiCatalog {
    sites: Vec<SiteSummary>,
    branches: HashMap<Uuid, Vec<BranchSummary>>,
    widget_definitions: Vec<WidgetDefinition>,
    widget_versions: HashMap<String, Vec<WidgetDefinitionVersion>>,
}

impl Default for ApiCatalog {
    fn default() -> Self {
        let template_site_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let derived_site_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();

        let sites = vec![
            SiteSummary {
                id: template_site_id,
                name: "Base Multifamily Template".to_owned(),
                slug: "base-multifamily-template".to_owned(),
                primary_host: "template.local".to_owned(),
                site_kind: SiteKind::Template,
                source_template_site_id: None,
            },
            SiteSummary {
                id: derived_site_id,
                name: "Austin Heights".to_owned(),
                slug: "austin-heights".to_owned(),
                primary_host: "austin-heights.local".to_owned(),
                site_kind: SiteKind::Standard,
                source_template_site_id: Some(template_site_id),
            },
        ];

        let branches = HashMap::from([
            (
                template_site_id,
                vec![
                    BranchSummary {
                        site_id: template_site_id,
                        name: "draft".to_owned(),
                        head_snapshot_id: Some(
                            Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                        ),
                    },
                    BranchSummary {
                        site_id: template_site_id,
                        name: "production".to_owned(),
                        head_snapshot_id: Some(
                            Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
                        ),
                    },
                ],
            ),
            (
                derived_site_id,
                vec![
                    BranchSummary {
                        site_id: derived_site_id,
                        name: "draft".to_owned(),
                        head_snapshot_id: Some(
                            Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
                        ),
                    },
                    BranchSummary {
                        site_id: derived_site_id,
                        name: "production".to_owned(),
                        head_snapshot_id: Some(
                            Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap(),
                        ),
                    },
                ],
            ),
        ]);

        let widget_definitions = vec![
            WidgetDefinition {
                id: Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap(),
                slug: "rich-text".to_owned(),
                display_name: "Rich Text".to_owned(),
                source_kind: WidgetSourceKind::Builtin,
                component_source_id: None,
                description: Some("Primitive text editor widget".to_owned()),
                is_primitive: true,
            },
            WidgetDefinition {
                id: Uuid::parse_str("66666666-6666-6666-6666-666666666666").unwrap(),
                slug: "hero-banner".to_owned(),
                display_name: "Hero Banner".to_owned(),
                source_kind: WidgetSourceKind::RegistryRepo,
                component_source_id: Some(
                    Uuid::parse_str("77777777-7777-7777-7777-777777777777").unwrap(),
                ),
                description: Some("Registry-backed marketing hero".to_owned()),
                is_primitive: false,
            },
        ];

        let widget_versions = HashMap::from([
            (
                "rich-text".to_owned(),
                vec![WidgetDefinitionVersion {
                    id: Uuid::parse_str("88888888-8888-8888-8888-888888888888").unwrap(),
                    definition_id: Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap(),
                    version: "1.0.0".to_owned(),
                    runtime: WidgetRuntime::ServerTemplate,
                    html_support_mode: HtmlSupportMode::SanitizedFragment,
                    settings_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "content": { "type": "object" }
                        },
                        "required": ["content"]
                    }),
                    editor_schema: serde_json::json!({
                        "kind": "rich_text"
                    }),
                    asset_manifest: serde_json::json!({}),
                    supports_server_render: true,
                }],
            ),
            (
                "hero-banner".to_owned(),
                vec![
                    WidgetDefinitionVersion {
                        id: Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap(),
                        definition_id: Uuid::parse_str("66666666-6666-6666-6666-666666666666")
                            .unwrap(),
                        version: "3.4.1".to_owned(),
                        runtime: WidgetRuntime::Svelte,
                        html_support_mode: HtmlSupportMode::SanitizedFragment,
                        settings_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "headline": { "type": "string" },
                                "cta_text": { "type": "string" },
                                "image_asset_id": { "type": "string" }
                            },
                            "required": ["headline"]
                        }),
                        editor_schema: serde_json::json!({
                            "layout": "stacked_form"
                        }),
                        asset_manifest: serde_json::json!({
                            "js": ["hero-banner.js"],
                            "css": ["hero-banner.css"]
                        }),
                        supports_server_render: true,
                    },
                    WidgetDefinitionVersion {
                        id: Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap(),
                        definition_id: Uuid::parse_str("66666666-6666-6666-6666-666666666666")
                            .unwrap(),
                        version: "4.0.0".to_owned(),
                        runtime: WidgetRuntime::Svelte,
                        html_support_mode: HtmlSupportMode::TrustedFragment,
                        settings_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "headline": { "type": "string" },
                                "body_html": { "type": "string" }
                            },
                            "required": ["headline"]
                        }),
                        editor_schema: serde_json::json!({
                            "layout": "advanced_form"
                        }),
                        asset_manifest: serde_json::json!({
                            "js": ["hero-banner-v4.js"],
                            "css": ["hero-banner-v4.css"]
                        }),
                        supports_server_render: true,
                    },
                ],
            ),
        ]);

        Self {
            sites,
            branches,
            widget_definitions,
            widget_versions,
        }
    }
}

impl ApiCatalog {
    pub fn sites(&self) -> &[SiteSummary] {
        &self.sites
    }

    pub fn branches_for_site(&self, site_id: Uuid) -> &[BranchSummary] {
        self.branches
            .get(&site_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn branch_head(&self, site_id: Uuid, branch_name: &str) -> Option<&BranchSummary> {
        self.branches_for_site(site_id)
            .iter()
            .find(|branch| branch.name == branch_name)
    }

    pub fn widget_definitions(&self) -> &[WidgetDefinition] {
        &self.widget_definitions
    }

    pub fn widget_definition(&self, slug: &str) -> Option<&WidgetDefinition> {
        self.widget_definitions
            .iter()
            .find(|definition| definition.slug == slug)
    }

    pub fn widget_versions(&self, slug: &str) -> &[WidgetDefinitionVersion] {
        self.widget_versions
            .get(slug)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}
