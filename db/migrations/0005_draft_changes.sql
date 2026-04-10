CREATE TABLE draft_change_sets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE CASCADE,
    base_snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE SET NULL,
    source_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE draft_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    change_set_id UUID NOT NULL REFERENCES draft_change_sets(id) ON DELETE CASCADE,
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    page_id UUID NULL REFERENCES pages(id) ON DELETE SET NULL,
    migration_job_id UUID NULL REFERENCES migration_jobs(id) ON DELETE SET NULL,
    migration_page_id UUID NULL REFERENCES migration_pages(id) ON DELETE SET NULL,
    change_kind TEXT NOT NULL,
    resource_kind TEXT NOT NULL,
    resource_key TEXT NOT NULL,
    status TEXT NOT NULL,
    title TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (change_set_id, resource_key, change_kind)
);

CREATE INDEX idx_draft_change_sets_site_status
    ON draft_change_sets (site_id, status, created_at DESC);

CREATE INDEX idx_draft_change_sets_branch
    ON draft_change_sets (branch_id, created_at DESC);

CREATE INDEX idx_draft_changes_change_set
    ON draft_changes (change_set_id, created_at ASC);

CREATE INDEX idx_draft_changes_migration_page
    ON draft_changes (migration_page_id);
