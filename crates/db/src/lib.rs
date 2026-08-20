//! Database connection pool, migration runner, RLS session setup,
//! and repository abstractions for the Shifa platform.

pub mod repository;
pub mod rls;

pub use repository::Repository;
pub use rls::{set_tenant_context, DbError};

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::path::Path;

/// Create a PostgreSQL connection pool with configured maximum connections.
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
}

/// Run all pending SQL migrations from the migrations directory against the database.
pub async fn run_migrations(pool: &PgPool, migrations_path: &Path) -> Result<(), DbError> {
    let migrator = sqlx::migrate::Migrator::new(migrations_path).await?;
    migrator.run(pool).await?;
    Ok(())
}

