//! Append-only stock ledger, batch management, FEFO allocation,
//! reservations with TTL, inter-branch transfers, and cold-chain monitoring.

pub mod cold_chain;
pub mod error;
pub mod fefo;
pub mod models;
pub mod reservations;
pub mod service;
pub mod transfers;

pub use cold_chain::ColdChainService;
pub use error::InventoryError;
pub use fefo::allocate_fefo;
pub use models::*;
pub use reservations::{release_expired_reservations, reserve_stock, ReserveStockParams};
pub use service::InventoryService;
pub use transfers::TransferService;
