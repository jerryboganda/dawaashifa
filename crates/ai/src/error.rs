use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiError {
    #[error("Circuit breaker is OPEN for task {0}: work queued for human review")]
    CircuitBreakerOpen(String),

    #[error("Model request timed out after {0} ms")]
    Timeout(u64),

    #[error("Provider API error: {0}")]
    ProviderError(String),

    #[error("Invalid prompt version: {0}")]
    InvalidPromptVersion(String),

    #[error("Voice transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Unauthorized AI action: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    Db(#[from] shifa_db::DbError),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
