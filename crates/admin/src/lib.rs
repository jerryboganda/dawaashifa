//! System settings, immutable audit log explorer, and operational reporting for the Shifa platform.

pub mod error;
pub mod models;
pub mod service;

pub use error::AdminError;
pub use models::*;
pub use service::AdminService;
