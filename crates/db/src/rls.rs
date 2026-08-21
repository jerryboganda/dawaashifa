use shifa_core::context::TenantContext;
use sqlx::{PgConnection, PgPool};
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Database error wrapper for Shifa database operations.
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Sets `app.tenant_id` for the current transaction (`is_local = true`).
/// Crate-private so callers cannot skip [`with_tenant`].
pub(crate) async fn set_tenant_context(
    conn: &mut PgConnection,
    ctx: &TenantContext,
) -> Result<(), DbError> {
    sqlx::query("SELECT set_config('app.tenant_id', $1, true), set_config('app.current_tenant_id', $1, true)")
        .bind(ctx.tenant_id().0.to_string())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Begin a transaction, set tenant GUC, run `f` on **this** connection, commit.
///
/// `f` must use the provided `&mut PgConnection`, not a captured `PgPool`.
/// Nested `with_tenant` opens a **different** pooled connection.
pub async fn with_tenant<F, T>(pool: &PgPool, ctx: &TenantContext, f: F) -> Result<T, DbError>
where
    F: for<'c> FnOnce(&'c mut PgConnection) -> BoxFuture<'c, Result<T, DbError>>,
{
    let mut tx = pool.begin().await?;
    set_tenant_context(&mut tx, ctx).await?;
    match f(&mut tx).await {
        Ok(value) => {
            tx.commit().await?;
            Ok(value)
        }
        Err(e) => {
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}
