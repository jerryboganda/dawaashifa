//! Order lifecycle, cart management, exhaustive state machine,
//! branch routing across shared stock, and COD payment handling.

pub mod error;
pub mod models;
pub mod numbering;
pub mod pricing;
pub mod routing;
pub mod service;
pub mod state_machine;

pub use error::OrderError;
pub use models::*;
pub use numbering::generate_order_number;
pub use pricing::{calculate_line_total, calculate_order_total, validate_item_price};
pub use routing::{compute_routing, RoutingRequest, RoutingResult, SplitFulfilmentPolicy};
pub use service::OrderService;
pub use state_machine::{can_transition, OrderStatus};
