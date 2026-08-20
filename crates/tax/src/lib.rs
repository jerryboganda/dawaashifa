//! Fiscal invoicing, real-time FBR POS integration, per-category tax calculation, and credit notes for the Shifa platform.

pub mod calculator;
pub mod error;
pub mod fbr;
pub mod models;
pub mod service;

pub use calculator::{TaxCalculator, TaxableItemInput};
pub use error::TaxError;
pub use fbr::{generate_fbr_qr_payload, FiscalReporter, MockFbrBehavior, MockFbrReporter};
pub use models::*;
pub use service::TaxService;
