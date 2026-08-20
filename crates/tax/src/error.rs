use shifa_core::id::{InvoiceId, OrderId, TaxCategoryId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaxError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Tax category not found: {0}")]
    CategoryNotFound(TaxCategoryId),

    #[error("Invoice not found: {0}")]
    InvoiceNotFound(InvoiceId),

    #[error("Order not found: {0}")]
    OrderNotFound(OrderId),

    #[error("No active tax rate found for category '{category}' at timestamp '{date}'")]
    NoActiveRateForDate { category: String, date: String },

    #[error("FBR rejected invoice submission: {reason} (code: {code:?})")]
    FbrRejection {
        reason: String,
        code: Option<String>,
    },

    #[error("FBR service outage / network failure: {message}")]
    FbrOutage { message: String },

    #[error("Invoices are immutable once issued; edit not permitted for invoice {0}")]
    ImmutableInvoiceCannotBeEdited(InvoiceId),

    #[error("Credit note already issued for invoice {0}")]
    CreditNoteAlreadyIssued(InvoiceId),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Core error: {0}")]
    Core(#[from] shifa_core::error::CoreError),
}
