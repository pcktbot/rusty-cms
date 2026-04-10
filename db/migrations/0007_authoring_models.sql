CREATE TABLE template_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NULL REFERENCES sites(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    display_name TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (site_id, slug)
);

CREATE TABLE template_targets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_definition_id UUID NOT NULL REFERENCES template_definitions(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    allows_primitives JSONB NOT NULL DEFAULT '[]'::JSONB,
    allows_widgets BOOLEAN NOT NULL DEFAULT TRUE,
    max_blocks INTEGER NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (template_definition_id, name)
);

CREATE TABLE draft_page_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    change_set_id UUID NOT NULL REFERENCES draft_change_sets(id) ON DELETE CASCADE,
    draft_change_id UUID NULL REFERENCES draft_changes(id) ON DELETE SET NULL,
    page_id UUID NULL REFERENCES pages(id) ON DELETE SET NULL,
    path TEXT NOT NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    template_definition_id UUID NULL REFERENCES template_definitions(id) ON DELETE SET NULL,
    template_key TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    seo JSONB NOT NULL DEFAULT '{}'::JSONB,
    document JSONB NOT NULL DEFAULT '{}'::JSONB,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (change_set_id, path),
    UNIQUE (change_set_id, draft_change_id)
);

CREATE INDEX idx_template_definitions_site_slug
    ON template_definitions (site_id, slug);

CREATE INDEX idx_template_targets_definition_position
    ON template_targets (template_definition_id, position);

CREATE INDEX idx_draft_page_documents_change_set
    ON draft_page_documents (change_set_id, path);
