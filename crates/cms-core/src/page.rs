use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

pub const DEFAULT_TEMPLATE_KEY: &str = "marketing-default";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveType {
    HeadingGroup,
    RichText,
    HtmlFragment,
    Image,
    MediaText,
    CtaBand,
    StatGroup,
    Quote,
    FaqList,
    Divider,
    Spacer,
    Container,
    Row,
    ColumnGroup,
    Aside,
    Stack,
    Grid,
    StickyCta,
    Modal,
    Drawer,
    Banner,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Primitive,
    Widget,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentWidth {
    Narrow,
    Standard,
    Wide,
    Full,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentAlignment {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaOrientation {
    ImageLeft,
    ImageRight,
    Stacked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColumnRatio {
    FiftyFifty,
    FortySixty,
    SixtyForty,
    ThirtyThreeSixtySeven,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MobileOrder {
    ContentFirst,
    MediaFirst,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpacingScale {
    None,
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default)]
pub struct BlockLayout {
    pub width: Option<ContentWidth>,
    pub alignment: Option<ContentAlignment>,
    pub orientation: Option<MediaOrientation>,
    pub column_ratio: Option<ColumnRatio>,
    pub mobile_order: Option<MobileOrder>,
    pub spacing_top: Option<SpacingScale>,
    pub spacing_bottom: Option<SpacingScale>,
    pub metadata: Value,
}

impl Default for BlockLayout {
    fn default() -> Self {
        Self {
            width: Some(ContentWidth::Standard),
            alignment: Some(ContentAlignment::Start),
            orientation: None,
            column_ratio: None,
            mobile_order: None,
            spacing_top: Some(SpacingScale::Md),
            spacing_bottom: Some(SpacingScale::Md),
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default)]
pub struct VisibilityRules {
    pub desktop: bool,
    pub mobile: bool,
    pub preview_only: bool,
}

impl Default for VisibilityRules {
    fn default() -> Self {
        Self {
            desktop: true,
            mobile: true,
            preview_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default)]
pub struct PageBlock {
    pub id: String,
    pub kind: BlockKind,
    #[serde(rename = "type")]
    pub block_type: String,
    pub display_name: Option<String>,
    pub props: Value,
    pub layout: BlockLayout,
    pub visibility: VisibilityRules,
    pub slots: BTreeMap<String, Vec<PageBlock>>,
    pub metadata: Value,
}

impl PageBlock {
    pub fn primitive(id: impl Into<String>, primitive_type: PrimitiveType, props: Value) -> Self {
        Self {
            id: id.into(),
            kind: BlockKind::Primitive,
            block_type: primitive_type_name(primitive_type).to_owned(),
            display_name: None,
            props,
            layout: BlockLayout::default(),
            visibility: VisibilityRules::default(),
            slots: BTreeMap::new(),
            metadata: serde_json::json!({}),
        }
    }

    pub fn widget(id: impl Into<String>, widget_slug: impl Into<String>, props: Value) -> Self {
        Self {
            id: id.into(),
            kind: BlockKind::Widget,
            block_type: widget_slug.into(),
            display_name: None,
            props,
            layout: BlockLayout::default(),
            visibility: VisibilityRules::default(),
            slots: BTreeMap::new(),
            metadata: serde_json::json!({}),
        }
    }
}

impl Default for PageBlock {
    fn default() -> Self {
        Self::primitive(
            "blk_default",
            PrimitiveType::HtmlFragment,
            serde_json::json!({}),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(default)]
pub struct PageSeo {
    pub title: Option<String>,
    pub h1: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub robots: Option<String>,
    pub open_graph: Value,
    pub twitter: Value,
    pub schema_types: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default)]
pub struct PageDocument {
    pub schema_version: i32,
    pub template_key: String,
    pub slots: BTreeMap<String, Vec<PageBlock>>,
    pub metadata: Value,
}

impl Default for PageDocument {
    fn default() -> Self {
        Self::empty_default()
    }
}

impl PageDocument {
    pub fn empty_default() -> Self {
        let mut slots = BTreeMap::new();
        slots.insert("header".to_owned(), Vec::new());
        slots.insert("before_main".to_owned(), Vec::new());
        slots.insert("main".to_owned(), Vec::new());
        slots.insert("after_main".to_owned(), Vec::new());
        slots.insert("footer".to_owned(), Vec::new());

        Self {
            schema_version: 1,
            template_key: DEFAULT_TEMPLATE_KEY.to_owned(),
            slots,
            metadata: serde_json::json!({}),
        }
    }

    pub fn from_candidate(candidate: &Value) -> Self {
        let template_key = candidate
            .get("template_key")
            .or_else(|| candidate.get("template"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_TEMPLATE_KEY)
            .to_owned();

        let source_slots = candidate
            .get("slots")
            .or_else(|| candidate.get("regions"))
            .and_then(Value::as_object);

        let mut document = Self {
            schema_version: candidate
                .get("schema_version")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or(1),
            template_key,
            slots: BTreeMap::new(),
            metadata: candidate
                .get("metadata")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        };

        if let Some(source_slots) = source_slots {
            for (slot_name, blocks) in source_slots {
                let parsed = blocks
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .enumerate()
                            .map(|(index, item)| {
                                candidate_block_to_page_block(item, slot_name, index)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                document.slots.insert(slot_name.clone(), parsed);
            }
        }

        if document.slots.is_empty() {
            document = Self::empty_default();
        }

        document
    }

    pub fn is_empty(&self) -> bool {
        self.slots.values().all(Vec::is_empty)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TemplateTargetDefinition {
    pub name: String,
    pub display_name: String,
    pub allows_primitives: Vec<PrimitiveType>,
    pub allows_widgets: bool,
    pub max_blocks: Option<u16>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TemplateDefinition {
    pub id: Uuid,
    pub site_id: Option<Uuid>,
    pub slug: String,
    pub display_name: String,
    pub schema_version: i32,
    pub targets: Vec<TemplateTargetDefinition>,
    pub metadata: Value,
}

fn candidate_block_to_page_block(value: &Value, slot_name: &str, index: usize) -> PageBlock {
    let fallback_id = format!("blk_{}_{}", sanitize_slot_name(slot_name), index + 1);
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(&fallback_id)
        .to_owned();

    let slots = value
        .get("slots")
        .or_else(|| value.get("regions"))
        .and_then(Value::as_object)
        .map(|items| {
            items
                .iter()
                .map(|(name, children)| {
                    let parsed = children
                        .as_array()
                        .map(|children| {
                            children
                                .iter()
                                .enumerate()
                                .map(|(child_index, child)| {
                                    candidate_block_to_page_block(child, name, child_index)
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    (name.clone(), parsed)
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let mut block = match value.get("kind").and_then(Value::as_str) {
        Some("widget") => {
            let widget_slug = value
                .get("widget_slug")
                .or_else(|| value.get("definition"))
                .or_else(|| value.get("slug"))
                .and_then(Value::as_str)
                .unwrap_or("unknown-widget");
            let props = value
                .get("settings")
                .cloned()
                .unwrap_or_else(|| value.clone());
            PageBlock::widget(id, widget_slug.to_owned(), props)
        }
        _ => {
            let primitive = value
                .get("primitive_type")
                .or_else(|| value.get("type"))
                .and_then(Value::as_str)
                .and_then(parse_primitive_type)
                .unwrap_or(PrimitiveType::HtmlFragment);
            let props = value
                .get("content")
                .cloned()
                .or_else(|| value.get("props").cloned())
                .unwrap_or_else(|| value.clone());
            PageBlock::primitive(id, primitive, props)
        }
    };

    block.display_name = value
        .get("display_name")
        .or_else(|| value.get("heading"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    block.metadata = serde_json::json!({
        "migration_candidate": value
    });
    block.slots = slots;
    block
}

fn sanitize_slot_name(value: &str) -> String {
    value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char
            } else {
                '_'
            }
        })
        .collect()
}

pub fn primitive_type_name(value: PrimitiveType) -> &'static str {
    match value {
        PrimitiveType::HeadingGroup => "heading_group",
        PrimitiveType::RichText => "rich_text",
        PrimitiveType::HtmlFragment => "html_fragment",
        PrimitiveType::Image => "image",
        PrimitiveType::MediaText => "media_text",
        PrimitiveType::CtaBand => "cta_band",
        PrimitiveType::StatGroup => "stat_group",
        PrimitiveType::Quote => "quote",
        PrimitiveType::FaqList => "faq_list",
        PrimitiveType::Divider => "divider",
        PrimitiveType::Spacer => "spacer",
        PrimitiveType::Container => "container",
        PrimitiveType::Row => "row",
        PrimitiveType::ColumnGroup => "column_group",
        PrimitiveType::Aside => "aside",
        PrimitiveType::Stack => "stack",
        PrimitiveType::Grid => "grid",
        PrimitiveType::StickyCta => "sticky_cta",
        PrimitiveType::Modal => "modal",
        PrimitiveType::Drawer => "drawer",
        PrimitiveType::Banner => "banner",
    }
}

pub fn parse_primitive_type(value: &str) -> Option<PrimitiveType> {
    match normalize_name(value).as_str() {
        "headinggroup" | "heading_group" => Some(PrimitiveType::HeadingGroup),
        "richtext" | "rich_text" => Some(PrimitiveType::RichText),
        "htmlfragment" | "html_fragment" | "html" => Some(PrimitiveType::HtmlFragment),
        "image" => Some(PrimitiveType::Image),
        "mediatext" | "media_text" => Some(PrimitiveType::MediaText),
        "ctaband" | "cta_band" => Some(PrimitiveType::CtaBand),
        "statgroup" | "stat_group" => Some(PrimitiveType::StatGroup),
        "quote" => Some(PrimitiveType::Quote),
        "faqlist" | "faq_list" => Some(PrimitiveType::FaqList),
        "divider" => Some(PrimitiveType::Divider),
        "spacer" => Some(PrimitiveType::Spacer),
        "container" => Some(PrimitiveType::Container),
        "row" => Some(PrimitiveType::Row),
        "columngroup" | "column_group" => Some(PrimitiveType::ColumnGroup),
        "aside" => Some(PrimitiveType::Aside),
        "stack" => Some(PrimitiveType::Stack),
        "grid" => Some(PrimitiveType::Grid),
        "stickycta" | "sticky_cta" => Some(PrimitiveType::StickyCta),
        "modal" => Some(PrimitiveType::Modal),
        "drawer" => Some(PrimitiveType::Drawer),
        "banner" => Some(PrimitiveType::Banner),
        _ => None,
    }
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PageDocument, PrimitiveType, VisibilityRules, parse_primitive_type};
    use serde_json::json;

    #[test]
    fn candidate_document_uses_regions_as_slots() {
        let document = PageDocument::from_candidate(&json!({
            "template_key": "marketing-default",
            "regions": {
                "main": [
                    {
                        "kind": "widget",
                        "widget_slug": "floor-plans-plus",
                        "settings": {
                            "headline": "Find your floor plan"
                        }
                    },
                    {
                        "kind": "primitive",
                        "primitive_type": "media_text",
                        "content": {
                            "heading": "Designed for comfort"
                        }
                    }
                ]
            }
        }));

        assert_eq!(document.template_key, "marketing-default");
        assert_eq!(document.slots["main"].len(), 2);
        assert_eq!(document.slots["main"][0].block_type, "floor-plans-plus");
        assert_eq!(document.slots["main"][1].block_type, "media_text");
    }

    #[test]
    fn parse_primitive_type_accepts_snake_case_and_compact_forms() {
        assert_eq!(
            parse_primitive_type("media_text"),
            Some(PrimitiveType::MediaText)
        );
        assert_eq!(
            parse_primitive_type("headinggroup"),
            Some(PrimitiveType::HeadingGroup)
        );
        assert_eq!(
            parse_primitive_type("html"),
            Some(PrimitiveType::HtmlFragment)
        );
    }

    #[test]
    fn visibility_defaults_are_both_visible() {
        let rules = VisibilityRules::default();
        assert!(rules.desktop);
        assert!(rules.mobile);
        assert!(!rules.preview_only);
    }
}
