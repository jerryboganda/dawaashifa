use crate::rls::DbError;
use async_trait::async_trait;
use shifa_core::context::TenantContext;
use shifa_core::id::BranchId;
use sqlx::PgConnection;

/// Base Repository trait for database access in the Shifa platform.
///
/// Invariant I-7: No raw SQL outside repository modules.
/// Every method takes `&TenantContext`. Implementors MUST filter with
/// `AND tenant_id = $n` using `ctx.tenant_id()` â€” unused `ctx` is a review BLOCKER.
/// RLS is a second layer, not a substitute for the SQL filter.
#[async_trait]
pub trait Repository<T, ID>: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn find_by_id(&self, ctx: &TenantContext, id: ID) -> Result<Option<T>, Self::Error>;
}

/// Proof-of-concept `find_by_id`: branch row only if it belongs to `ctx.tenant_id`.
pub async fn find_branch_id(
    conn: &mut PgConnection,
    ctx: &TenantContext,
    id: BranchId,
) -> Result<Option<BranchId>, DbError> {
    let row: Option<(sqlx::types::Uuid,)> =
        sqlx::query_as("SELECT id FROM branches WHERE id = $1 AND tenant_id = $2")
            .bind(id.0)
            .bind(ctx.tenant_id().0)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(row.map(|raw| BranchId::from(raw.0)))
}
