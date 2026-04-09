ALTER TABLE sites
    ADD COLUMN site_kind TEXT NOT NULL DEFAULT 'standard',
    ADD COLUMN source_template_site_id UUID NULL REFERENCES sites(id) ON DELETE SET NULL;

CREATE INDEX idx_sites_source_template_site_id ON sites (source_template_site_id);
