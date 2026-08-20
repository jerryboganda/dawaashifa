//! Rider assignment, delivery dispatch, proof of delivery (POD), and daily cash reconciliation for the Shifa platform.

pub mod assignment;
pub mod error;
pub mod models;
pub mod service;

pub use assignment::AssignmentEngine;
pub use error::FulfilmentError;
pub use models::*;
pub use service::FulfilmentService;
