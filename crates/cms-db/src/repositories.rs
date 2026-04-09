use crate::models::{
    BranchRow, SiteRow, WidgetDefinitionRow, WidgetDefinitionVersionRow, WorkflowRequestRow,
};
use sqlx::{PgPool, query_as};
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
}
