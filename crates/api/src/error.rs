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

    #[error("Inventory error: {0}")]
    Inventory(#[from] shifa_inventory::InventoryError),

    #[error("Conversation error: {0}")]
    Conversation(#[from] shifa_conversation::ConversationError),

    #[error("Order error: {0}")]
    Order(#[from] shifa_orders::OrderError),

    #[error("AI error: {0}")]
    Ai(#[from] shifa_ai::AiError),

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
            ApiError::Ai(shifa_ai::AiError::CircuitBreakerOpen(task)) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("AI Circuit breaker open for task {}: work queued for human review", task),
            ),
            ApiError::Ai(shifa_ai::AiError::Timeout(ms)) => (
                StatusCode::GATEWAY_TIMEOUT,
                format!("AI model request timed out after {} ms", ms),
            ),
            ApiError::Ai(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Order(shifa_orders::OrderError::InvalidTransition { from, to }) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid order state transition from {} to {}", from, to),
            ),
            ApiError::Order(shifa_orders::OrderError::RxItemRequiresReview) => (
                StatusCode::BAD_REQUEST,
                "Rx items present in order: must pass through AwaitingRx review (Invariant I-3 / I-6)".to_string(),
            ),
            ApiError::Order(shifa_orders::OrderError::AboveMrp { attempted, mrp }) => (
                StatusCode::BAD_REQUEST,
                format!("Sale price {} exceeds product MRP {}", attempted, mrp),
            ),
            ApiError::Order(shifa_orders::OrderError::ReservationFailed) => (
                StatusCode::BAD_REQUEST,
                "Reservation failed: insufficient stock available to confirm order".to_string(),
            ),
            ApiError::Order(shifa_orders::OrderError::NotFound(_)) => (
                StatusCode::NOT_FOUND,
                "Order not found".to_string(),
            ),
            ApiError::Order(shifa_orders::OrderError::Unauthorized(msg)) => (
                StatusCode::FORBIDDEN,
                msg,
            ),
            ApiError::Order(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Conversation(shifa_conversation::ConversationError::AlreadyClaimed(u)) => (
                StatusCode::CONFLICT,
                format!("Conversation already claimed by user {}", u),
            ),
            ApiError::Conversation(shifa_conversation::ConversationError::BulkApprovalRejectedForRx) => (
                StatusCode::BAD_REQUEST,
                "Bulk approval rejected: Rx-linked conversations require individual review (Invariant I-6)".to_string(),
            ),
            ApiError::Conversation(shifa_conversation::ConversationError::OutsideServiceWindow) => (
                StatusCode::BAD_REQUEST,
                "Free-form message rejected: outside 24h WhatsApp service window, template required".to_string(),
            ),
            ApiError::Conversation(shifa_conversation::ConversationError::Unauthorized(msg)) => (
                StatusCode::FORBIDDEN,
                msg,
            ),
            ApiError::Conversation(err) => (StatusCode::BAD_REQUEST, err.to_string()),
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
            ApiError::Inventory(shifa_inventory::InventoryError::InsufficientStock { requested, available, .. }) => (
                StatusCode::BAD_REQUEST,
                format!("Insufficient stock: requested {}, available {}", requested, available),
            ),
            ApiError::Inventory(shifa_inventory::InventoryError::Unauthorized(msg)) => {
                (StatusCode::FORBIDDEN, msg)
            }
            ApiError::Inventory(err) => (StatusCode::BAD_REQUEST, err.to_string()),
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
