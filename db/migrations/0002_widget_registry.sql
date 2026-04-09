CREATE TABLE component_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE,
    source_kind TEXT NOT NULL,
    repo_url TEXT NULL,
    default_ref TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE widget_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    component_source_id UUID NULL REFERENCES component_sources(id) ON DELETE SET NULL,
    description TEXT NULL,
    is_primitive BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE widget_definition_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    widget_definition_id UUID NOT NULL REFERENCES widget_definitions(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    runtime TEXT NOT NULL,
    html_support_mode TEXT NOT NULL DEFAULT 'none',
    settings_schema JSONB NOT NULL DEFAULT '{}'::JSONB,
    editor_schema JSONB NOT NULL DEFAULT '{}'::JSONB,
    asset_manifest JSONB NOT NULL DEFAULT '{}'::JSONB,
    supports_server_render BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (widget_definition_id, version)
);

CREATE TABLE widget_instances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    page_id UUID NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    snapshot_id UUID NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    parent_widget_instance_id UUID NULL REFERENCES widget_instances(id) ON DELETE CASCADE,
    region TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    widget_definition_id UUID NOT NULL REFERENCES widget_definitions(id) ON DELETE RESTRICT,
    widget_definition_version_id UUID NOT NULL REFERENCES widget_definition_versions(id) ON DELETE RESTRICT,
    settings JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_widget_definitions_source_kind ON widget_definitions (source_kind);
CREATE INDEX idx_widget_definition_versions_definition_id ON widget_definition_versions (widget_definition_id);
CREATE INDEX idx_widget_instances_page_snapshot ON widget_instances (page_id, snapshot_id);
CREATE INDEX idx_widget_instances_parent_id ON widget_instances (parent_widget_instance_id);
