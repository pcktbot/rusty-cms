CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    primary_host TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (account_id, slug),
    UNIQUE (primary_host)
);

CREATE TABLE branches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    head_snapshot_id UUID NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (site_id, name)
);

CREATE TABLE snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    created_by TEXT NOT NULL,
    manifest JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE branches
    ADD CONSTRAINT branches_head_snapshot_fk
    FOREIGN KEY (head_snapshot_id) REFERENCES snapshots(id) ON DELETE SET NULL;

CREATE TABLE pages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    parent_id UUID NULL REFERENCES pages(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (site_id, path),
    UNIQUE (site_id, parent_id, slug)
);

CREATE TABLE page_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    page_id UUID NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    snapshot_id UUID NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    schema_version INTEGER NOT NULL DEFAULT 1,
    document JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (page_id, snapshot_id)
);

CREATE TABLE publish_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    snapshot_id UUID NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    state TEXT NOT NULL,
    target_dir TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    release_key TEXT NOT NULL,
    failure_message TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE publish_artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    publish_job_id UUID NOT NULL REFERENCES publish_jobs(id) ON DELETE CASCADE,
    logical_path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    byte_size BIGINT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (publish_job_id, logical_path)
);

CREATE TABLE workflow_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_id UUID NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    branch_name TEXT NOT NULL,
    workflow_kind TEXT NOT NULL,
    requested_runtime TEXT NOT NULL,
    temporal_queue TEXT NOT NULL,
    input_payload JSONB NOT NULL DEFAULT '{}'::JSONB,
    output_schema TEXT NOT NULL,
    requires_human_approval BOOLEAN NOT NULL DEFAULT FALSE,
    max_sites_touched INTEGER NOT NULL DEFAULT 1,
    allow_publish_side_effects BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE outbox_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    topic TEXT NOT NULL,
    event_key TEXT NOT NULL,
    payload JSONB NOT NULL,
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pages_site_id_path ON pages (site_id, path);
CREATE INDEX idx_page_documents_snapshot_id ON page_documents (snapshot_id);
CREATE INDEX idx_publish_jobs_site_state ON publish_jobs (site_id, state);
CREATE INDEX idx_workflow_requests_site_created_at ON workflow_requests (site_id, created_at DESC);
CREATE INDEX idx_outbox_events_topic_available_at ON outbox_events (topic, available_at);
