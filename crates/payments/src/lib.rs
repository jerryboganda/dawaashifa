//! Payment gateway integrations, screenshot verification queue, TID ledger, and COD handling for the Shifa platform.

pub mod error;
pub mod gateways;
pub mod models;
pub mod ocr;
pub mod service;

pub use error::PaymentError;
pub use models::*;
pub use service::PaymentService;
