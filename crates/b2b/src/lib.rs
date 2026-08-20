//! B2B module for medical device sales, business accounts, quotations,
//! purchase orders, credit control, accounts receivable, and consignment inventory.

pub mod ar;
pub mod consignment;
pub mod credit;
pub mod device;
pub mod error;
pub mod models;
pub mod po;
pub mod quotes;
pub mod service;

pub use ar::AccountsReceivable;
pub use consignment::ConsignmentManager;
pub use credit::CreditControl;
pub use device::DeviceTraceability;
pub use error::B2bError;
pub use models::*;
pub use po::PurchaseOrderEngine;
pub use quotes::QuoteEngine;
pub use service::B2bService;
