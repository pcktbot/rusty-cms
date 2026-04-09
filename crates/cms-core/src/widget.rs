use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum WidgetSourceKind {
    Builtin,
    RegistryRepo,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum WidgetRuntime {
    ServerTemplate,
    Svelte,
    React,
    Vue,
    WebComponent,
    RawJavascript,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum HtmlSupportMode {
    None,
    SanitizedFragment,
    TrustedFragment,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WidgetDefinitionRef {
    pub definition_id: Uuid,
    pub version_id: Uuid,
    pub slug: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WidgetDefinition {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub source_kind: WidgetSourceKind,
    pub component_source_id: Option<Uuid>,
    pub description: Option<String>,
    pub is_primitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WidgetDefinitionVersion {
    pub id: Uuid,
    pub definition_id: Uuid,
    pub version: String,
    pub runtime: WidgetRuntime,
    pub html_support_mode: HtmlSupportMode,
    pub settings_schema: serde_json::Value,
    pub editor_schema: serde_json::Value,
    pub asset_manifest: serde_json::Value,
    pub supports_server_render: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WidgetInstance {
    pub id: Uuid,
    pub page_id: Uuid,
    pub snapshot_id: Uuid,
    pub parent_widget_id: Option<Uuid>,
    pub region: String,
    pub position: i32,
    pub definition: WidgetDefinitionRef,
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WidgetCommand {
    InsertWidget {
        region: String,
        position: i32,
        definition: WidgetDefinitionRef,
        settings: serde_json::Value,
    },
    UpdateWidgetSettings {
        widget_id: Uuid,
        patch: serde_json::Value,
    },
    MoveWidget {
        widget_id: Uuid,
        to_region: String,
        to_position: i32,
    },
    ReplaceWidget {
        widget_id: Uuid,
        definition: WidgetDefinitionRef,
        settings: serde_json::Value,
    },
    RemoveWidget {
        widget_id: Uuid,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        HtmlSupportMode, WidgetCommand, WidgetDefinition, WidgetDefinitionRef,
        WidgetDefinitionVersion, WidgetRuntime, WidgetSourceKind,
    };
    use uuid::Uuid;

    #[test]
    fn primitive_widget_definition_has_builtin_source() {
        let definition = WidgetDefinition {
            id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            slug: "rich-text".to_owned(),
            display_name: "Rich Text".to_owned(),
            source_kind: WidgetSourceKind::Builtin,
            component_source_id: None,
            description: Some("Primitive rich text widget".to_owned()),
            is_primitive: true,
        };

        assert_eq!(definition.source_kind, WidgetSourceKind::Builtin);
        assert!(definition.is_primitive);
    }

    #[test]
    fn registry_widget_version_can_describe_svelte_runtime() {
        let version = WidgetDefinitionVersion {
            id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            definition_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            version: "3.4.1".to_owned(),
            runtime: WidgetRuntime::Svelte,
            html_support_mode: HtmlSupportMode::SanitizedFragment,
            settings_schema: serde_json::json!({ "type": "object" }),
            editor_schema: serde_json::json!({ "layout": "form" }),
            asset_manifest: serde_json::json!({ "js": ["widget.js"] }),
            supports_server_render: true,
        };

        assert_eq!(version.runtime, WidgetRuntime::Svelte);
        assert!(version.supports_server_render);
    }

    #[test]
    fn widget_command_serializes_with_tagged_type() {
        let command = WidgetCommand::InsertWidget {
            region: "main".to_owned(),
            position: 0,
            definition: WidgetDefinitionRef {
                definition_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
                version_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
                slug: "hero-banner".to_owned(),
                version: "1.0.0".to_owned(),
            },
            settings: serde_json::json!({ "headline": "Hello" }),
        };

        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["type"], "insert_widget");
    }
}
