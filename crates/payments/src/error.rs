use shifa_core::id::{OrderId, PaymentId, ProofId};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PaymentError {
    #[error("Payment {0} not found")]
    PaymentNotFound(PaymentId),

    #[error("Payment proof {0} not found")]
    ProofNotFound(ProofId),

    #[error("Order {0} not found")]
    OrderNotFound(OrderId),

    #[error("Invalid payment status transition: {0}")]
    InvalidStatusTransition(String),

    #[error("Invalid webhook signature: {0}")]
    InvalidSignature(String),

    #[error("Webhook timestamp expired or replayed: {0}")]
    ReplayDetected(String),

    #[error("Webhook amount mismatch: expected {expected}, received {received}")]
    AmountMismatch { expected: String, received: String },

    #[error("Duplicate transaction ID detected: {0}")]
    DuplicateTransactionId(String),

    #[error("COD limit exceeded for customer: current outstanding {current}, limit {limit}")]
    CodLimitExceeded { current: String, limit: String },

    #[error("Customer is blocked from COD due to excessive refusals")]
    CustomerCodBlocked,

    #[error("Gateway error: {0}")]
    GatewayError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
