use dawaa_core::id::TenantId;
use sqlx::PgConnection;
use thiserror::Error;

/// Database error wrapper for Dawaa database operations.
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Sets the PostgreSQL session variable `app.tenant_id` within the local transaction scope.
///
/// Invariant I-2: Postgres Row-Level Security is enabled on every tenant-scoped table.
/// The `true` parameter ensures this setting is scoped strictly to the current transaction.
pub async fn set_tenant_context(
    conn: &mut PgConnection,
    tenant_id: TenantId,
) -> Result<(), DbError> {
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id.0.to_string())
        .execute(conn)
        .await?;
    Ok(())
}
