use shifa_core::id::{BatchId, BranchId, ProductId};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InventoryError {
    #[error("Insufficient stock for product {product_id} at branch {branch_id}: requested {requested}, available {available}")]
    InsufficientStock {
        product_id: ProductId,
        branch_id: BranchId,
        requested: i32,
        available: i32,
    },

    #[error("Negative stock constraint violation for batch {0}")]
    NegativeStock(BatchId),

    #[error("Batch with ID '{0}' not found")]
    BatchNotFound(BatchId),

    #[error("Transfer with ID '{0}' not found")]
    TransferNotFound(uuid::Uuid),

    #[error("Invalid transfer state transition from {0} to {1}")]
    InvalidTransferState(String, String),

    #[error("Transfer quantity mismatch: expected {expected}, received {received}")]
    TransferDiscrepancy { expected: i32, received: i32 },

    #[error("Branch {0} is not equipped for cold-chain storage")]
    ColdChainIncapable(BranchId),

    #[error("Batch {0} is currently quarantined due to temperature excursion")]
    BatchQuarantined(BatchId),

    #[error("Unauthorized inventory action: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    Db(#[from] shifa_db::DbError),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
