//! WhatsApp threaded conversations, assignment strategies, human override,
//! canned replies with strict variable checking, and SLA timers.

pub mod assignment;
pub mod canned;
pub mod customer;
pub mod error;
pub mod models;
pub mod override_engine;
pub mod routing;
pub mod service;
pub mod sla;

pub use assignment::{assign_least_busy, claim_conversation, AssignmentStrategy};
pub use canned::render_canned_reply;
pub use customer::resolve_or_create_customer;
pub use error::ConversationError;
pub use models::*;
pub use override_engine::{bulk_approve_drafts, override_message};
pub use routing::route_conversation;
pub use service::ConversationService;
pub use sla::{evaluate_sla_escalation, is_within_opening_hours};
