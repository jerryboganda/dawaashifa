use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Account is suspended")]
    AccountSuspended,

    #[error("Rate limit exceeded: too many failed login attempts. Please try again later.")]
    RateLimitExceeded,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Session has been revoked")]
    SessionRevoked,

    #[error("Reused refresh token detected: session family revoked for security")]
    SessionFamilyRevoked,

    #[error("Password does not meet complexity requirements: {0}")]
    WeakPassword(String),

    #[error("Unauthorized: missing required permission '{0}'")]
    PermissionDenied(String),

    #[error("Branch access denied: not assigned to branch '{0}'")]
    BranchAccessDenied(String),

    #[error("Resource not found")]
    NotFound,

    #[error("Duplicate resource: {0}")]
    Duplicate(String),

    #[error("Database error: {0}")]
    Db(#[from] shifa_db::DbError),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}
