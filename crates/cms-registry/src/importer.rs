use cms_core::widget::{HtmlSupportMode, WidgetRuntime, WidgetSourceKind};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{collections::HashMap, fs, path::Path, process::Command};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WidgetImportError {
    #[error("widget source path does not exist: {0}")]
    PathMissing(String),
    #[error("required file missing: {0}")]
    RequiredFileMissing(String),
    #[error("failed to read {path}: {source}")]
    ReadFailure {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    ParseFailure {
        path: String,
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedWidgetSetting {
    pub name: String,
    pub default_value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedWidgetPackage {
    pub repo_path: String,
    pub git_sha: Option<String>,
    pub widget_slug: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub source_kind: WidgetSourceKind,
    pub runtime: WidgetRuntime,
    pub html_support_mode: HtmlSupportMode,
    pub settings_count: usize,
    pub settings_schema: Value,
    pub editor_schema: Value,
    pub asset_manifest: Value,
    pub verticals: Vec<String>,
    pub remote_javascripts: Vec<String>,
    pub local_javascripts: Vec<String>,
    pub local_stylesheets: Vec<String>,
    pub has_show_template: bool,
    pub has_edit_template: bool,
    pub has_source_directory: bool,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct WidgetSourceImporter;

#[derive(Debug, Deserialize)]
struct IndexManifest {
    name: String,
    summary: Option<String>,
    liquid: Option<bool>,
    verticals: Option<Vec<String>>,
    remote_javascripts: Option<Vec<String>>,
    lib_edit_stylesheets: Option<Vec<String>>,
    settings: Option<Vec<IndexSetting>>,
}

#[derive(Debug, Deserialize)]
struct IndexSetting {
    name: String,
    default_value: Value,
}

#[derive(Debug, Deserialize)]
struct PackageManifest {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<HashMap<String, String>>,
    dev_dependencies: Option<HashMap<String, String>>,
}

impl WidgetSourceImporter {
    pub fn import_from_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ImportedWidgetPackage, WidgetImportError> {
        let root = path.as_ref();
        if !root.exists() {
            return Err(WidgetImportError::PathMissing(root.display().to_string()));
        }

        let index_path = root.join("index.json");
        if !index_path.exists() {
            return Err(WidgetImportError::RequiredFileMissing(
                index_path.display().to_string(),
            ));
        }

        let index_manifest: IndexManifest = read_json(&index_path)?;
        let package_manifest: Option<PackageManifest> = {
            let package_path = root.join("package.json");
            if package_path.exists() {
                Some(read_json(&package_path)?)
            } else {
                None
            }
        };

        let settings = index_manifest.settings.unwrap_or_default();
        let remote_javascripts = index_manifest.remote_javascripts.unwrap_or_default();
        let mut local_stylesheets = collect_relative_files(root, "stylesheets");
        for stylesheet in index_manifest.lib_edit_stylesheets.unwrap_or_default() {
            if !local_stylesheets.contains(&stylesheet) {
                local_stylesheets.push(stylesheet);
            }
        }

        Ok(ImportedWidgetPackage {
            repo_path: root.display().to_string(),
            git_sha: git_sha(root),
            widget_slug: slugify(&index_manifest.name),
            display_name: index_manifest.name,
            summary: index_manifest.summary,
            source_kind: WidgetSourceKind::RegistryRepo,
            runtime: infer_runtime(package_manifest.as_ref()),
            html_support_mode: infer_html_support_mode(
                root,
                index_manifest.liquid.unwrap_or(false),
            ),
            settings_count: settings.len(),
            settings_schema: settings_schema(&settings),
            editor_schema: editor_schema(root),
            asset_manifest: asset_manifest(root),
            verticals: index_manifest.verticals.unwrap_or_default(),
            remote_javascripts,
            local_javascripts: collect_relative_files(root, "javascripts"),
            local_stylesheets,
            has_show_template: root.join("show.html").exists(),
            has_edit_template: root.join("edit.html").exists()
                || root.join("edit-raul.html").exists(),
            has_source_directory: root.join("src").exists(),
            package_name: package_manifest
                .as_ref()
                .and_then(|manifest| manifest.name.clone()),
            package_version: package_manifest.and_then(|manifest| manifest.version),
        })
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, WidgetImportError> {
    let body = fs::read_to_string(path).map_err(|source| WidgetImportError::ReadFailure {
        path: path.display().to_string(),
        source,
    })?;

    serde_json::from_str(&body).map_err(|source| WidgetImportError::ParseFailure {
        path: path.display().to_string(),
        source,
    })
}

fn collect_relative_files(root: &Path, relative_dir: &str) -> Vec<String> {
    let dir = root.join(relative_dir);
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(relative) = path.strip_prefix(root) {
                    files.push(relative.display().to_string());
                }
            } else if path.is_dir() {
                files.extend(collect_relative_files(
                    root,
                    &path.strip_prefix(root).unwrap().display().to_string(),
                ));
            }
        }
    }
    files.sort();
    files
}

fn infer_runtime(package_manifest: Option<&PackageManifest>) -> WidgetRuntime {
    let dependencies = package_manifest
        .and_then(|manifest| manifest.dependencies.as_ref())
        .cloned()
        .unwrap_or_default();
    let dev_dependencies = package_manifest
        .and_then(|manifest| manifest.dev_dependencies.as_ref())
        .cloned()
        .unwrap_or_default();

    let has_dep =
        |name: &str| dependencies.contains_key(name) || dev_dependencies.contains_key(name);

    if has_dep("svelte") || has_dep("@sveltejs/kit") {
        WidgetRuntime::Svelte
    } else if has_dep("vue") {
        WidgetRuntime::Vue
    } else if has_dep("react") {
        WidgetRuntime::React
    } else {
        WidgetRuntime::RawJavascript
    }
}

fn infer_html_support_mode(root: &Path, liquid_enabled: bool) -> HtmlSupportMode {
    if liquid_enabled || root.join("show.html").exists() || root.join("edit.html").exists() {
        HtmlSupportMode::TrustedFragment
    } else {
        HtmlSupportMode::None
    }
}

fn settings_schema(settings: &[IndexSetting]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for setting in settings {
        properties.insert(
            setting.name.clone(),
            json!({
                "type": json_schema_type(&setting.default_value),
                "default": setting.default_value,
            }),
        );
        required.push(setting.name.clone());
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": true
    })
}

fn editor_schema(root: &Path) -> Value {
    let edit_template_paths = [
        root.join("edit.html").exists().then_some("edit.html"),
        root.join("edit-raul.html")
            .exists()
            .then_some("edit-raul.html"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    json!({
        "edit_template_paths": edit_template_paths,
        "show_template_path": root.join("show.html").exists().then_some("show.html"),
    })
}

fn asset_manifest(root: &Path) -> Value {
    json!({
        "show_template_path": root.join("show.html").exists().then_some("show.html"),
        "edit_template_path": root.join("edit.html").exists().then_some("edit.html"),
        "edit_raul_template_path": root.join("edit-raul.html").exists().then_some("edit-raul.html"),
        "javascripts": collect_relative_files(root, "javascripts"),
        "stylesheets": collect_relative_files(root, "stylesheets"),
        "has_source_directory": root.join("src").exists(),
        "has_graphql_schema": root.join("graphql.schema.json").exists(),
    })
}

fn json_schema_type(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Null => "null",
        Value::String(_) => "string",
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn git_sha(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::WidgetSourceImporter;
    use cms_core::widget::{HtmlSupportMode, WidgetRuntime};
    use std::path::Path;

    #[test]
    fn importer_reads_fixture_widget_repo() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple-widget");
        let package = WidgetSourceImporter::default()
            .import_from_path(&fixture)
            .unwrap();

        assert_eq!(package.widget_slug, "simple-hero");
        assert_eq!(package.runtime, WidgetRuntime::Svelte);
        assert_eq!(package.html_support_mode, HtmlSupportMode::TrustedFragment);
        assert_eq!(package.settings_count, 2);
        assert!(package.has_show_template);
        assert!(package.has_edit_template);
        assert!(package.has_source_directory);
    }
}
