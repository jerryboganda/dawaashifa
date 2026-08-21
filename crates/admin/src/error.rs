use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
