CREATE TABLE migration_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    workflow_request_id UUID NOT NULL REFERENCES workflow_requests(id) ON DELETE CASCADE,
    workflow_id TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    homepage_url TEXT NOT NULL,
    client_id UUID NOT NULL,
    location_id UUID NOT NULL,
    legacy_api_profile TEXT NULL,
    status TEXT NOT NULL,
    options JSONB NOT NULL DEFAULT '{}'::JSONB,
    warnings JSONB NOT NULL DEFAULT '[]'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at TIMESTAMPTZ NULL
);

CREATE TABLE migration_pages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    migration_job_id UUID NOT NULL REFERENCES migration_jobs(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    title_guess TEXT NOT NULL,
    widget_matches JSONB NOT NULL DEFAULT '[]'::JSONB,
    unknown_regions INTEGER NOT NULL DEFAULT 0,
    confidence REAL NOT NULL DEFAULT 0,
    warnings JSONB NOT NULL DEFAULT '[]'::JSONB,
    extraction_notes JSONB NOT NULL DEFAULT '[]'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_migration_jobs_site_created_at
    ON migration_jobs (site_id, created_at DESC);

CREATE INDEX idx_migration_jobs_workflow_request_id
    ON migration_jobs (workflow_request_id);

CREATE INDEX idx_migration_pages_migration_job_id
    ON migration_pages (migration_job_id, path);
