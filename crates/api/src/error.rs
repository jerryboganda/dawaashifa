use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found")]
    NotFound,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("Auth error: {0}")]
    Auth(#[from] shifa_identity::AuthError),

    #[error("Catalog error: {0}")]
    Catalog(#[from] shifa_catalog::CatalogError),

    #[error("Core error: {0}")]
    Core(#[from] shifa_core::error::CoreError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Catalog(shifa_catalog::CatalogError::ProductNotFound(_)) => {
                (StatusCode::NOT_FOUND, "Product not found".to_string())
            }
            ApiError::Catalog(shifa_catalog::CatalogError::AboveMrp { mrp, attempted }) => (
                StatusCode::BAD_REQUEST,
                format!("Sale price {} cannot exceed MRP {}", attempted, mrp),
            ),
            ApiError::Catalog(shifa_catalog::CatalogError::Unauthorized(msg)) => {
                (StatusCode::FORBIDDEN, msg)
            }
            ApiError::Catalog(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Auth(shifa_identity::AuthError::InvalidCredentials) => (
                StatusCode::UNAUTHORIZED,
                "Invalid phone/email or password".to_string(),
            ),
            ApiError::Auth(shifa_identity::AuthError::RateLimitExceeded) => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many failed login attempts. Please try again in 15 minutes.".to_string(),
            ),
            ApiError::Auth(shifa_identity::AuthError::SessionRevoked) => (
                StatusCode::UNAUTHORIZED,
                "Session has expired or was revoked".to_string(),
            ),
            ApiError::Auth(shifa_identity::AuthError::SessionFamilyRevoked) => (
                StatusCode::UNAUTHORIZED,
                "Session family terminated due to suspicious token reuse".to_string(),
            ),
            ApiError::Auth(shifa_identity::AuthError::PermissionDenied(p)) => (
                StatusCode::FORBIDDEN,
                format!("Missing required permission: {}", p),
            ),
            ApiError::Auth(shifa_identity::AuthError::BranchAccessDenied(b)) => (
                StatusCode::FORBIDDEN,
                format!("Access denied to branch: {}", b),
            ),
            ApiError::Auth(shifa_identity::AuthError::NotFound) => {
                (StatusCode::NOT_FOUND, "Resource not found".to_string())
            }
            ApiError::Core(shifa_core::error::CoreError::PermissionDenied(p)) => (
                StatusCode::FORBIDDEN,
                format!("Missing required permission: {}", p),
            ),
            ApiError::Core(shifa_core::error::CoreError::BranchAccessDenied(b)) => (
                StatusCode::FORBIDDEN,
                format!("Access denied to branch: {}", b),
            ),
            ApiError::Auth(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Core(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Internal(err) => {
                tracing::error!("Internal server error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An unexpected error occurred".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": message
            }
        }));

        (status, body).into_response()
    }
}
