use sqlx::{PgPool, migrate::Migrator};
use std::path::Path;

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let migration_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../db/migrations");
    let migrator = Migrator::new(migration_dir.as_path()).await?;
    migrator.run(pool).await
}
