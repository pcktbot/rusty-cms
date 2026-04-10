use crate::models::{
    AccountRow, BranchRow, DraftChangeRow, DraftChangeSetRow, DraftPageDocumentRow,
    MigrationJobRow, MigrationPageArtifactRow, MigrationPageRow, OutboxEventRow, SiteRow,
    TemplateDefinitionRow, TemplateTargetRow, WidgetDefinitionRow, WidgetDefinitionVersionRow,
    WorkflowRequestRow,
};
use sqlx::{PgPool, query_as, query_scalar};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn list_sites(&self) -> Result<Vec<SiteRow>, sqlx::Error> {
        query_as::<_, SiteRow>(
            r#"
            SELECT id, account_id, name, slug, primary_host, site_kind, source_template_site_id, created_at, updated_at
            FROM sites
            ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn insert_account(&self, row: &AccountRow) -> Result<AccountRow, sqlx::Error> {
        query_as::<_, AccountRow>(
            r#"
            INSERT INTO accounts (
                id,
                name,
                slug,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, slug, created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(&row.name)
        .bind(&row.slug)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_site(&self, row: &SiteRow) -> Result<SiteRow, sqlx::Error> {
        query_as::<_, SiteRow>(
            r#"
            INSERT INTO sites (
                id,
                account_id,
                name,
                slug,
                primary_host,
                site_kind,
                source_template_site_id,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, account_id, name, slug, primary_host, site_kind, source_template_site_id, created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.account_id)
        .bind(&row.name)
        .bind(&row.slug)
        .bind(&row.primary_host)
        .bind(&row.site_kind)
        .bind(row.source_template_site_id)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_branch(&self, row: &BranchRow) -> Result<BranchRow, sqlx::Error> {
        query_as::<_, BranchRow>(
            r#"
            INSERT INTO branches (
                id,
                site_id,
                name,
                head_snapshot_id,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, site_id, name, head_snapshot_id, created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.site_id)
        .bind(&row.name)
        .bind(row.head_snapshot_id)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_branches_for_site(
        &self,
        site_id: Uuid,
    ) -> Result<Vec<BranchRow>, sqlx::Error> {
        query_as::<_, BranchRow>(
            r#"
            SELECT id, site_id, name, head_snapshot_id, created_at, updated_at
            FROM branches
            WHERE site_id = $1
            ORDER BY name ASC
            "#,
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn site_exists(&self, site_id: Uuid) -> Result<bool, sqlx::Error> {
        query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM sites
                WHERE id = $1
            )
            "#,
        )
        .bind(site_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn branch_head(
        &self,
        site_id: Uuid,
        branch_name: &str,
    ) -> Result<Option<BranchRow>, sqlx::Error> {
        query_as::<_, BranchRow>(
            r#"
            SELECT id, site_id, name, head_snapshot_id, created_at, updated_at
            FROM branches
            WHERE site_id = $1 AND name = $2
            LIMIT 1
            "#,
        )
        .bind(site_id)
        .bind(branch_name)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_widget_definitions(&self) -> Result<Vec<WidgetDefinitionRow>, sqlx::Error> {
        query_as::<_, WidgetDefinitionRow>(
            r#"
            SELECT id, slug, display_name, source_kind, component_source_id, description, is_primitive, created_at, updated_at
            FROM widget_definitions
            ORDER BY slug ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn widget_definition_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<WidgetDefinitionRow>, sqlx::Error> {
        query_as::<_, WidgetDefinitionRow>(
            r#"
            SELECT id, slug, display_name, source_kind, component_source_id, description, is_primitive, created_at, updated_at
            FROM widget_definitions
            WHERE slug = $1
            LIMIT 1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_widget_definition_versions(
        &self,
        slug: &str,
    ) -> Result<Vec<WidgetDefinitionVersionRow>, sqlx::Error> {
        query_as::<_, WidgetDefinitionVersionRow>(
            r#"
            SELECT versions.id,
                   versions.widget_definition_id,
                   versions.version,
                   versions.runtime,
                   versions.html_support_mode,
                   versions.settings_schema,
                   versions.editor_schema,
                   versions.asset_manifest,
                   versions.supports_server_render,
                   versions.created_at
            FROM widget_definition_versions versions
            INNER JOIN widget_definitions definitions
                ON definitions.id = versions.widget_definition_id
            WHERE definitions.slug = $1
            ORDER BY versions.version DESC
            "#,
        )
        .bind(slug)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn insert_workflow_request(
        &self,
        row: &WorkflowRequestRow,
    ) -> Result<WorkflowRequestRow, sqlx::Error> {
        query_as::<_, WorkflowRequestRow>(
            r#"
            INSERT INTO workflow_requests (
                id,
                site_id,
                branch_name,
                workflow_kind,
                requested_runtime,
                temporal_queue,
                input_payload,
                output_schema,
                requires_human_approval,
                max_sites_touched,
                allow_publish_side_effects,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE
            SET site_id = EXCLUDED.site_id,
                branch_name = EXCLUDED.branch_name,
                workflow_kind = EXCLUDED.workflow_kind,
                requested_runtime = EXCLUDED.requested_runtime,
                temporal_queue = EXCLUDED.temporal_queue,
                input_payload = EXCLUDED.input_payload,
                output_schema = EXCLUDED.output_schema,
                requires_human_approval = EXCLUDED.requires_human_approval,
                max_sites_touched = EXCLUDED.max_sites_touched,
                allow_publish_side_effects = EXCLUDED.allow_publish_side_effects
            RETURNING id, site_id, branch_name, workflow_kind, requested_runtime, temporal_queue,
                      input_payload, output_schema, requires_human_approval, max_sites_touched,
                      allow_publish_side_effects, created_at
            "#,
        )
        .bind(row.id)
        .bind(row.site_id)
        .bind(&row.branch_name)
        .bind(&row.workflow_kind)
        .bind(&row.requested_runtime)
        .bind(&row.temporal_queue)
        .bind(&row.input_payload)
        .bind(&row.output_schema)
        .bind(row.requires_human_approval)
        .bind(row.max_sites_touched)
        .bind(row.allow_publish_side_effects)
        .bind(row.created_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_outbox_event(
        &self,
        row: &OutboxEventRow,
    ) -> Result<OutboxEventRow, sqlx::Error> {
        query_as::<_, OutboxEventRow>(
            r#"
            INSERT INTO outbox_events (
                id,
                topic,
                event_key,
                payload,
                available_at,
                published_at,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, topic, event_key, payload, available_at, published_at, created_at
            "#,
        )
        .bind(row.id)
        .bind(&row.topic)
        .bind(&row.event_key)
        .bind(&row.payload)
        .bind(row.available_at)
        .bind(row.published_at)
        .bind(row.created_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_migration_job(
        &self,
        row: &MigrationJobRow,
    ) -> Result<MigrationJobRow, sqlx::Error> {
        query_as::<_, MigrationJobRow>(
            r#"
            INSERT INTO migration_jobs (
                id,
                site_id,
                workflow_request_id,
                workflow_id,
                branch_name,
                homepage_url,
                client_id,
                location_id,
                legacy_api_profile,
                status,
                options,
                warnings,
                created_at,
                approved_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, site_id, workflow_request_id, workflow_id, branch_name, homepage_url,
                      client_id, location_id, legacy_api_profile, status, options, warnings,
                      created_at, approved_at
            "#,
        )
        .bind(row.id)
        .bind(row.site_id)
        .bind(row.workflow_request_id)
        .bind(&row.workflow_id)
        .bind(&row.branch_name)
        .bind(&row.homepage_url)
        .bind(row.client_id)
        .bind(row.location_id)
        .bind(&row.legacy_api_profile)
        .bind(&row.status)
        .bind(&row.options)
        .bind(&row.warnings)
        .bind(row.created_at)
        .bind(row.approved_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_migration_page(
        &self,
        row: &MigrationPageRow,
    ) -> Result<MigrationPageRow, sqlx::Error> {
        query_as::<_, MigrationPageRow>(
            r#"
            INSERT INTO migration_pages (
                id,
                migration_job_id,
                path,
                title_guess,
                widget_matches,
                unknown_regions,
                confidence,
                warnings,
                extraction_notes,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, migration_job_id, path, title_guess, widget_matches, unknown_regions,
                      confidence, warnings, extraction_notes, created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.migration_job_id)
        .bind(&row.path)
        .bind(&row.title_guess)
        .bind(&row.widget_matches)
        .bind(row.unknown_regions)
        .bind(row.confidence)
        .bind(&row.warnings)
        .bind(&row.extraction_notes)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn upsert_migration_page_artifact(
        &self,
        row: &MigrationPageArtifactRow,
    ) -> Result<MigrationPageArtifactRow, sqlx::Error> {
        query_as::<_, MigrationPageArtifactRow>(
            r#"
            INSERT INTO migration_page_artifacts (
                id,
                migration_page_id,
                source_url,
                http_status,
                final_url,
                artifact,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (migration_page_id) DO UPDATE
            SET source_url = EXCLUDED.source_url,
                http_status = EXCLUDED.http_status,
                final_url = EXCLUDED.final_url,
                artifact = EXCLUDED.artifact,
                updated_at = EXCLUDED.updated_at
            RETURNING id, migration_page_id, source_url, http_status, final_url, artifact,
                      created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.migration_page_id)
        .bind(&row.source_url)
        .bind(row.http_status)
        .bind(&row.final_url)
        .bind(&row.artifact)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn migration_job(
        &self,
        migration_job_id: Uuid,
    ) -> Result<Option<MigrationJobRow>, sqlx::Error> {
        query_as::<_, MigrationJobRow>(
            r#"
            SELECT id, site_id, workflow_request_id, workflow_id, branch_name, homepage_url,
                   client_id, location_id, legacy_api_profile, status, options, warnings,
                   created_at, approved_at
            FROM migration_jobs
            WHERE id = $1
            LIMIT 1
            "#,
        )
        .bind(migration_job_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn migration_pages(
        &self,
        migration_job_id: Uuid,
    ) -> Result<Vec<MigrationPageRow>, sqlx::Error> {
        query_as::<_, MigrationPageRow>(
            r#"
            SELECT id, migration_job_id, path, title_guess, widget_matches, unknown_regions,
                   confidence, warnings, extraction_notes, created_at, updated_at
            FROM migration_pages
            WHERE migration_job_id = $1
            ORDER BY path ASC
            "#,
        )
        .bind(migration_job_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn migration_page(
        &self,
        migration_job_id: Uuid,
        page_id: Uuid,
    ) -> Result<Option<MigrationPageRow>, sqlx::Error> {
        query_as::<_, MigrationPageRow>(
            r#"
            SELECT id, migration_job_id, path, title_guess, widget_matches, unknown_regions,
                   confidence, warnings, extraction_notes, created_at, updated_at
            FROM migration_pages
            WHERE migration_job_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(migration_job_id)
        .bind(page_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn migration_page_artifact(
        &self,
        migration_page_id: Uuid,
    ) -> Result<Option<MigrationPageArtifactRow>, sqlx::Error> {
        query_as::<_, MigrationPageArtifactRow>(
            r#"
            SELECT id, migration_page_id, source_url, http_status, final_url, artifact,
                   created_at, updated_at
            FROM migration_page_artifacts
            WHERE migration_page_id = $1
            LIMIT 1
            "#,
        )
        .bind(migration_page_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete_migration_pages(&self, migration_job_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM migration_pages
            WHERE migration_job_id = $1
            "#,
        )
        .bind(migration_job_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn approve_migration_job(
        &self,
        migration_job_id: Uuid,
        approved_at: time::OffsetDateTime,
    ) -> Result<Option<MigrationJobRow>, sqlx::Error> {
        query_as::<_, MigrationJobRow>(
            r#"
            UPDATE migration_jobs
            SET status = 'approved',
                approved_at = $2
            WHERE id = $1
            RETURNING id, site_id, workflow_request_id, workflow_id, branch_name, homepage_url,
                      client_id, location_id, legacy_api_profile, status, options, warnings,
                      created_at, approved_at
            "#,
        )
        .bind(migration_job_id)
        .bind(approved_at)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_migration_job_status(
        &self,
        migration_job_id: Uuid,
        status: &str,
    ) -> Result<Option<MigrationJobRow>, sqlx::Error> {
        query_as::<_, MigrationJobRow>(
            r#"
            UPDATE migration_jobs
            SET status = $2
            WHERE id = $1
            RETURNING id, site_id, workflow_request_id, workflow_id, branch_name, homepage_url,
                      client_id, location_id, legacy_api_profile, status, options, warnings,
                      created_at, approved_at
            "#,
        )
        .bind(migration_job_id)
        .bind(status)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_migration_job_review_data(
        &self,
        migration_job_id: Uuid,
        status: &str,
        warnings: &sqlx::types::Json<Vec<String>>,
    ) -> Result<Option<MigrationJobRow>, sqlx::Error> {
        query_as::<_, MigrationJobRow>(
            r#"
            UPDATE migration_jobs
            SET status = $2,
                warnings = $3
            WHERE id = $1
            RETURNING id, site_id, workflow_request_id, workflow_id, branch_name, homepage_url,
                      client_id, location_id, legacy_api_profile, status, options, warnings,
                      created_at, approved_at
            "#,
        )
        .bind(migration_job_id)
        .bind(status)
        .bind(warnings)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_draft_change_set(
        &self,
        row: &DraftChangeSetRow,
    ) -> Result<DraftChangeSetRow, sqlx::Error> {
        query_as::<_, DraftChangeSetRow>(
            r#"
            INSERT INTO draft_change_sets (
                id,
                site_id,
                branch_id,
                base_snapshot_id,
                source_kind,
                status,
                name,
                description,
                metadata,
                created_by,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, site_id, branch_id, base_snapshot_id, source_kind, status, name,
                      description, metadata, created_by, created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.site_id)
        .bind(row.branch_id)
        .bind(row.base_snapshot_id)
        .bind(&row.source_kind)
        .bind(&row.status)
        .bind(&row.name)
        .bind(&row.description)
        .bind(&row.metadata)
        .bind(&row.created_by)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_draft_change(
        &self,
        row: &DraftChangeRow,
    ) -> Result<DraftChangeRow, sqlx::Error> {
        query_as::<_, DraftChangeRow>(
            r#"
            INSERT INTO draft_changes (
                id,
                change_set_id,
                site_id,
                page_id,
                migration_job_id,
                migration_page_id,
                change_kind,
                resource_kind,
                resource_key,
                status,
                title,
                payload,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, change_set_id, site_id, page_id, migration_job_id, migration_page_id,
                      change_kind, resource_kind, resource_key, status, title, payload,
                      created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.change_set_id)
        .bind(row.site_id)
        .bind(row.page_id)
        .bind(row.migration_job_id)
        .bind(row.migration_page_id)
        .bind(&row.change_kind)
        .bind(&row.resource_kind)
        .bind(&row.resource_key)
        .bind(&row.status)
        .bind(&row.title)
        .bind(&row.payload)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_draft_page_document(
        &self,
        row: &DraftPageDocumentRow,
    ) -> Result<DraftPageDocumentRow, sqlx::Error> {
        query_as::<_, DraftPageDocumentRow>(
            r#"
            INSERT INTO draft_page_documents (
                id,
                change_set_id,
                draft_change_id,
                page_id,
                path,
                slug,
                title,
                template_definition_id,
                template_key,
                schema_version,
                seo,
                document,
                metadata,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING id, change_set_id, draft_change_id, page_id, path, slug, title,
                      template_definition_id, template_key, schema_version, seo, document,
                      metadata, created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.change_set_id)
        .bind(row.draft_change_id)
        .bind(row.page_id)
        .bind(&row.path)
        .bind(&row.slug)
        .bind(&row.title)
        .bind(row.template_definition_id)
        .bind(&row.template_key)
        .bind(row.schema_version)
        .bind(&row.seo)
        .bind(&row.document)
        .bind(&row.metadata)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn draft_page_document(
        &self,
        change_set_id: Uuid,
        draft_change_id: Uuid,
    ) -> Result<Option<DraftPageDocumentRow>, sqlx::Error> {
        query_as::<_, DraftPageDocumentRow>(
            r#"
            SELECT id, change_set_id, draft_change_id, page_id, path, slug, title,
                   template_definition_id, template_key, schema_version, seo, document,
                   metadata, created_at, updated_at
            FROM draft_page_documents
            WHERE change_set_id = $1 AND draft_change_id = $2
            LIMIT 1
            "#,
        )
        .bind(change_set_id)
        .bind(draft_change_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_template_definition(
        &self,
        row: &TemplateDefinitionRow,
    ) -> Result<TemplateDefinitionRow, sqlx::Error> {
        query_as::<_, TemplateDefinitionRow>(
            r#"
            INSERT INTO template_definitions (
                id,
                site_id,
                slug,
                display_name,
                schema_version,
                metadata,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, site_id, slug, display_name, schema_version, metadata,
                      created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.site_id)
        .bind(&row.slug)
        .bind(&row.display_name)
        .bind(row.schema_version)
        .bind(&row.metadata)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn insert_template_target(
        &self,
        row: &TemplateTargetRow,
    ) -> Result<TemplateTargetRow, sqlx::Error> {
        query_as::<_, TemplateTargetRow>(
            r#"
            INSERT INTO template_targets (
                id,
                template_definition_id,
                name,
                display_name,
                position,
                allows_primitives,
                allows_widgets,
                max_blocks,
                metadata,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, template_definition_id, name, display_name, position,
                      allows_primitives, allows_widgets, max_blocks, metadata,
                      created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(row.template_definition_id)
        .bind(&row.name)
        .bind(&row.display_name)
        .bind(row.position)
        .bind(&row.allows_primitives)
        .bind(row.allows_widgets)
        .bind(row.max_blocks)
        .bind(&row.metadata)
        .bind(row.created_at)
        .bind(row.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_template_definitions(
        &self,
        site_id: Option<Uuid>,
    ) -> Result<Vec<TemplateDefinitionRow>, sqlx::Error> {
        match site_id {
            Some(site_id) => {
                query_as::<_, TemplateDefinitionRow>(
                    r#"
                    SELECT id, site_id, slug, display_name, schema_version, metadata, created_at, updated_at
                    FROM template_definitions
                    WHERE site_id = $1 OR site_id IS NULL
                    ORDER BY site_id NULLS FIRST, slug ASC
                    "#,
                )
                .bind(site_id)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                query_as::<_, TemplateDefinitionRow>(
                    r#"
                    SELECT id, site_id, slug, display_name, schema_version, metadata, created_at, updated_at
                    FROM template_definitions
                    ORDER BY site_id NULLS FIRST, slug ASC
                    "#,
                )
                .fetch_all(&self.pool)
                .await
            }
        }
    }

    pub async fn template_targets(
        &self,
        template_definition_id: Uuid,
    ) -> Result<Vec<TemplateTargetRow>, sqlx::Error> {
        query_as::<_, TemplateTargetRow>(
            r#"
            SELECT id, template_definition_id, name, display_name, position, allows_primitives,
                   allows_widgets, max_blocks, metadata, created_at, updated_at
            FROM template_targets
            WHERE template_definition_id = $1
            ORDER BY position ASC, name ASC
            "#,
        )
        .bind(template_definition_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn draft_change_set(
        &self,
        change_set_id: Uuid,
    ) -> Result<Option<DraftChangeSetRow>, sqlx::Error> {
        query_as::<_, DraftChangeSetRow>(
            r#"
            SELECT id, site_id, branch_id, base_snapshot_id, source_kind, status, name,
                   description, metadata, created_by, created_at, updated_at
            FROM draft_change_sets
            WHERE id = $1
            LIMIT 1
            "#,
        )
        .bind(change_set_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn draft_changes(
        &self,
        change_set_id: Uuid,
    ) -> Result<Vec<DraftChangeRow>, sqlx::Error> {
        query_as::<_, DraftChangeRow>(
            r#"
            SELECT id, change_set_id, site_id, page_id, migration_job_id, migration_page_id,
                   change_kind, resource_kind, resource_key, status, title, payload,
                   created_at, updated_at
            FROM draft_changes
            WHERE change_set_id = $1
            ORDER BY created_at ASC, resource_key ASC
            "#,
        )
        .bind(change_set_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn draft_change(
        &self,
        change_set_id: Uuid,
        change_id: Uuid,
    ) -> Result<Option<DraftChangeRow>, sqlx::Error> {
        query_as::<_, DraftChangeRow>(
            r#"
            SELECT id, change_set_id, site_id, page_id, migration_job_id, migration_page_id,
                   change_kind, resource_kind, resource_key, status, title, payload,
                   created_at, updated_at
            FROM draft_changes
            WHERE change_set_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(change_set_id)
        .bind(change_id)
        .fetch_optional(&self.pool)
        .await
    }
}
