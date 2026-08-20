use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Source adapter error: {0}")]
    Source(String),

    #[error("Mapping parse/validation error: {0}")]
    Mapping(String),

    #[error("Transform error for field '{field}' on value '{value}': {reason}")]
    Transform {
        field: String,
        value: String,
        reason: String,
    },

    #[error("Validation failed for record: {0}")]
    Validation(String),

    #[error("Batch not found: {0}")]
    BatchNotFound(Uuid),

    #[error(
        "Rollback refused: {count} records in batch {batch_id} have dependent records ({reason})"
    )]
    RollbackRefused {
        batch_id: Uuid,
        count: usize,
        reason: String,
    },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Core error: {0}")]
    Core(#[from] shifa_core::error::CoreError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
