use shifa_core::id::PrescriptionId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RxError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Prescription not found: {0}")]
    PrescriptionNotFound(PrescriptionId),

    #[error("Incomplete review: decision missing for line {0}")]
    IncompleteReview(i32),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Invalid prescription image: {0}")]
    InvalidImage(String),

    #[error("Invalid drug substitution: {0}")]
    InvalidSubstitution(String),

    #[error("Prescription image is write-once and cannot be replaced")]
    ImageImmutable,

    #[error("Prescription already claimed by another pharmacist")]
    AlreadyClaimed,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Core error: {0}")]
    Core(#[from] shifa_core::error::CoreError),

    #[error("Catalog error: {0}")]
    Catalog(#[from] shifa_catalog::CatalogError),

    #[error("AI error: {0}")]
    Ai(#[from] shifa_ai::AiError),

    #[error("Internal error: {0}")]
    Internal(String),
}
