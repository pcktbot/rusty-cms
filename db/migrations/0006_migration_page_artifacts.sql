CREATE TABLE migration_page_artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    migration_page_id UUID NOT NULL REFERENCES migration_pages(id) ON DELETE CASCADE,
    source_url TEXT NOT NULL,
    http_status INTEGER NULL,
    final_url TEXT NULL,
    artifact JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (migration_page_id)
);

CREATE INDEX idx_migration_page_artifacts_page_id
    ON migration_page_artifacts (migration_page_id);
