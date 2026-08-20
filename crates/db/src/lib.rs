//! Database connection pool, migration runner, RLS session setup,
//! seed data generator, and repository abstractions for the Shifa platform.

pub mod repository;
pub mod rls;
pub mod seed;

pub use repository::{find_branch_id, Repository};
pub use rls::{set_tenant_context, with_tenant, DbError};
pub use seed::SeedStats;

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Create a PostgreSQL connection pool with configured maximum connections.
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
}

/// Run all pending SQL migrations embedded at compile-time against the database.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}
