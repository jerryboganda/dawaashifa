use shifa_core::context::TenantContext;
use sqlx::{PgConnection, PgPool};
use thiserror::Error;

/// Database error wrapper for Shifa database operations.
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Sets `app.tenant_id` for the **current transaction** (`is_local = true`).
///
/// Must be called only after `BEGIN`. Outside a transaction Postgres ignores
/// a local GUC and RLS would not apply. Prefer [`with_tenant`].
pub async fn set_tenant_context(
    conn: &mut PgConnection,
    ctx: &TenantContext,
) -> Result<(), DbError> {
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(ctx.tenant_id().0.to_string())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Begin a transaction, set tenant GUC from `ctx`, run `f`, commit.
/// This is the required RLS entry point â€” do not skip it for tenant queries.
pub async fn with_tenant<F, Fut, T>(pool: &PgPool, ctx: &TenantContext, f: F) -> Result<T, DbError>
where
    F: FnOnce(&mut PgConnection) -> Fut,
    Fut: std::future::Future<Output = Result<T, DbError>>,
{
    let mut tx = pool.begin().await?;
    set_tenant_context(&mut tx, ctx).await?;
    let result = f(&mut tx).await?;
    tx.commit().await?;
    Ok(result)
}
