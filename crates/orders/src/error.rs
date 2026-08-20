use shifa_core::id::OrderId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrderError {
    #[error("Order {0} not found")]
    NotFound(OrderId),

    #[error("Invalid order state transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error(
        "Rx items present in order: must pass through AwaitingRx review (Invariant I-3 / I-6)"
    )]
    RxItemRequiresReview,

    #[error("Sale price {attempted} exceeds product MRP {mrp}")]
    AboveMrp { attempted: String, mrp: String },

    #[error("Item {0} not found in order")]
    ItemNotFound(uuid::Uuid),

    #[error("Reservation failed: insufficient stock available to confirm order")]
    ReservationFailed,

    #[error("Restock rejected: medicines require pharmacist certification before restock")]
    RestockRequiresCertification,

    #[error("Restock rejected: refrigerated items that left cold chain cannot be restocked")]
    ColdChainRestockForbidden,

    #[error("Unauthorized order action: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    Db(#[from] shifa_db::DbError),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
