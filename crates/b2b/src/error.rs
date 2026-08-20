use rust_decimal::Decimal;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum B2bError {
    #[error(
        "Negotiated price {price} is above MRP {mrp} for product {product_id} (MRP cap invariant)"
    )]
    NegotiatedPriceAboveMrp {
        product_id: Uuid,
        price: Decimal,
        mrp: Decimal,
    },

    #[error("Quotation '{0}' not found")]
    QuoteNotFound(Uuid),

    #[error("Quotation '{0}' has expired on {1} and cannot be converted or accepted")]
    QuoteExpired(Uuid, String),

    #[error("Quotation discount of Rs {discount} requires manager approval with sufficient limit")]
    DiscountRequiresApproval { discount: Decimal },

    #[error("Approver limit (Rs {limit}) is below the required approval amount (Rs {required})")]
    ApproverBelowLimit { limit: Decimal, required: Decimal },

    #[error("Credit limit exceeded for account '{account_name}': limit Rs {limit}, current outstanding Rs {outstanding}, order amount Rs {order_amount}")]
    CreditLimitExceeded {
        account_name: String,
        limit: Decimal,
        outstanding: Decimal,
        order_amount: Decimal,
    },

    #[error("Account '{0}' is on hold ({1})")]
    AccountOnHold(String, String),

    #[error(
        "Account has overdue balance over 90 days (Rs {0}); new orders/dispatches are blocked"
    )]
    OverdueBalanceBlocked(Decimal),

    #[error("Purchase order amount or item variance detected ({0}); fulfilment is blocked until resolved")]
    PoVarianceBlocked(String),

    #[error("Purchase order '{0}' not found")]
    PoNotFound(Uuid),

    #[error("Product '{0}' not found")]
    ProductNotFound(Uuid),

    #[error("Business account '{0}' not found")]
    AccountNotFound(Uuid),

    #[error("Business contact '{0}' not found")]
    ContactNotFound(Uuid),

    #[error("Consignment location or item '{0}' not found")]
    ConsignmentNotFound(Uuid),

    #[error("Device unit with serial number '{0}' already exists for this tenant")]
    DeviceSerialDuplicate(String),

    #[error("Device unit '{0}' not found")]
    DeviceNotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Core error: {0}")]
    Core(#[from] shifa_core::error::CoreError),
}
