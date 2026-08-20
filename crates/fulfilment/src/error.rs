use shifa_core::id::{DeliveryId, PickingListId, RiderCashSessionId, RiderId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FulfilmentError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Delivery not found: {0}")]
    DeliveryNotFound(DeliveryId),

    #[error("Rider not found: {0}")]
    RiderNotFound(RiderId),

    #[error("Cash session not found: {0}")]
    CashSessionNotFound(RiderCashSessionId),

    #[error("Picking list not found: {0}")]
    PickingListNotFound(PickingListId),

    #[error("Invalid state transition from {current} to {requested}")]
    InvalidStateTransition { current: String, requested: String },

    #[error("Rider {rider_id} undeposited cash Rs {current_undeposited} exceeds COD ceiling Rs {limit}")]
    CashCeilingExceeded {
        rider_id: RiderId,
        limit: String,
        current_undeposited: String,
    },

    #[error("Rider {rider_id} has stale open cash session {session_id} (>24h) blocking COD assignment")]
    StaleCashSessionBlocked {
        rider_id: RiderId,
        session_id: RiderCashSessionId,
    },

    #[error("Delivery {0} exceeded maximum 2 reattempts and is marked RETURNED")]
    MaxReattemptsExceeded(DeliveryId),

    #[error("POD missing mandatory field: {0}")]
    PodMissingField(String),

    #[error("Session closure with non-zero variance requires documented note")]
    VarianceReasonRequired,

    #[error("Controlled substance delivery requires original prescription collection and recipient CNIC last 4")]
    ControlledSubstanceRequiresPrescriptionAndCnic,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Core error: {0}")]
    Core(#[from] shifa_core::error::CoreError),
}
