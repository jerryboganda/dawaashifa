use thiserror::Error;

/// Core domain error primitives for the Shifa platform.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("Invalid money scale: {0}")]
    InvalidMoneyScale(u32),

    #[error("Arithmetic overflow in money operation")]
    MoneyOverflow,

    #[error("Unauthorized access: missing required permission '{0}'")]
    PermissionDenied(String),

    #[error("Unauthorized branch access: branch '{0}' is outside user scope")]
    BranchAccessDenied(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

