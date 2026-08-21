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

    #[error("Prescription error: {0}")]
    Rx(#[from] shifa_prescription::RxError),

    #[error("Payment error: {0}")]
    Payment(#[from] shifa_payments::PaymentError),

    #[error("Fulfilment error: {0}")]
    Fulfilment(#[from] shifa_fulfilment::FulfilmentError),

    #[error("Tax error: {0}")]
    Tax(#[from] shifa_tax::TaxError),

    #[error("B2B error: {0}")]
    B2b(#[from] shifa_b2b::B2bError),

    #[error("Admin error: {0}")]
    Admin(#[from] shifa_admin::AdminError),

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
            ApiError::Payment(shifa_payments::PaymentError::InvalidSignature(msg)) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid signature: {}", msg),
            ),
            ApiError::Payment(shifa_payments::PaymentError::ReplayDetected(msg)) => (
                StatusCode::BAD_REQUEST,
                format!("Replay detected: {}", msg),
            ),
            ApiError::Payment(shifa_payments::PaymentError::AmountMismatch { expected, received }) => (
                StatusCode::BAD_REQUEST,
                format!("Amount mismatch: expected {}, received {}", expected, received),
            ),
            ApiError::Payment(shifa_payments::PaymentError::DuplicateTransactionId(tid)) => (
                StatusCode::CONFLICT,
                format!("Duplicate transaction ID: {}", tid),
            ),
            ApiError::Payment(shifa_payments::PaymentError::CodLimitExceeded { current, limit }) => (
                StatusCode::BAD_REQUEST,
                format!("COD limit exceeded: current {}, limit {}", current, limit),
            ),
            ApiError::Payment(shifa_payments::PaymentError::CustomerCodBlocked) => (
                StatusCode::FORBIDDEN,
                "Customer is blocked from COD due to excessive refusals".to_string(),
            ),
            ApiError::Payment(shifa_payments::PaymentError::PaymentNotFound(_)) => (
                StatusCode::NOT_FOUND,
                "Payment not found".to_string(),
            ),
            ApiError::Payment(shifa_payments::PaymentError::ProofNotFound(_)) => (
                StatusCode::NOT_FOUND,
                "Payment proof not found".to_string(),
            ),
            ApiError::Payment(shifa_payments::PaymentError::Unauthorized(msg)) => (
                StatusCode::FORBIDDEN,
                msg,
            ),
            ApiError::Payment(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Rx(shifa_prescription::RxError::IncompleteReview(line)) => (
                StatusCode::BAD_REQUEST,
                format!("Incomplete review: decision missing for line {} (Invariant I-3)", line),
            ),
            ApiError::Rx(shifa_prescription::RxError::AlreadyClaimed) => (
                StatusCode::CONFLICT,
                "Prescription is already claimed by another pharmacist".to_string(),
            ),
            ApiError::Rx(shifa_prescription::RxError::PrescriptionNotFound(_)) => (
                StatusCode::NOT_FOUND,
                "Prescription not found".to_string(),
            ),
            ApiError::Rx(shifa_prescription::RxError::Unauthorized(msg)) => (
                StatusCode::FORBIDDEN,
                msg,
            ),
            ApiError::Rx(err) => (StatusCode::BAD_REQUEST, err.to_string()),
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
            ApiError::Fulfilment(shifa_fulfilment::FulfilmentError::Unauthorized(msg)) => {
                (StatusCode::UNAUTHORIZED, msg)
            }
            ApiError::Fulfilment(shifa_fulfilment::FulfilmentError::Forbidden(msg)) => {
                (StatusCode::FORBIDDEN, msg)
            }
            ApiError::Fulfilment(shifa_fulfilment::FulfilmentError::NotFound(msg)) => {
                (StatusCode::NOT_FOUND, msg)
            }
            ApiError::Fulfilment(shifa_fulfilment::FulfilmentError::DeliveryNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Delivery {} not found", id))
            }
            ApiError::Fulfilment(shifa_fulfilment::FulfilmentError::RiderNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Rider {} not found", id))
            }
            ApiError::Fulfilment(shifa_fulfilment::FulfilmentError::CashSessionNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Cash session {} not found", id))
            }
            ApiError::Fulfilment(shifa_fulfilment::FulfilmentError::PickingListNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Picking list {} not found", id))
            }
            ApiError::Fulfilment(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Tax(shifa_tax::TaxError::CategoryNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Tax category {} not found", id))
            }
            ApiError::Tax(shifa_tax::TaxError::InvoiceNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Invoice {} not found", id))
            }
            ApiError::Tax(shifa_tax::TaxError::OrderNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Order {} not found", id))
            }
            ApiError::Tax(shifa_tax::TaxError::Unauthorized(msg)) => {
                (StatusCode::UNAUTHORIZED, msg)
            }
            ApiError::Tax(shifa_tax::TaxError::Forbidden(msg)) => {
                (StatusCode::FORBIDDEN, msg)
            }
            ApiError::Tax(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::B2b(shifa_b2b::B2bError::AccountNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Business account {} not found", id))
            }
            ApiError::B2b(shifa_b2b::B2bError::QuoteNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Quotation {} not found", id))
            }
            ApiError::B2b(shifa_b2b::B2bError::DeviceNotFound(serial)) => {
                (StatusCode::NOT_FOUND, format!("Device unit {} not found", serial))
            }
            ApiError::B2b(shifa_b2b::B2bError::DeviceSerialDuplicate(serial)) => {
                (StatusCode::CONFLICT, format!("Device serial {} already exists", serial))
            }
            ApiError::B2b(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApiError::Admin(shifa_admin::AdminError::PermissionDenied(msg)) => {
                (StatusCode::FORBIDDEN, msg)
            }
            ApiError::Admin(shifa_admin::AdminError::TenantNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Tenant {} not found", id))
            }
            ApiError::Admin(err) => (StatusCode::BAD_REQUEST, err.to_string()),
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
