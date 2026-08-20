use async_trait::async_trait;
use shifa_core::context::TenantContext;

/// Base Repository trait for database access in the Shifa platform.
///
/// Invariant I-7: No raw SQL outside repository modules. All access through typed repositories.
#[async_trait]
pub trait Repository<T, ID>: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Retrieve a single entity by its strongly-typed identifier within the tenant scope.
    async fn find_by_id(&self, ctx: &TenantContext, id: ID) -> Result<Option<T>, Self::Error>;
}

