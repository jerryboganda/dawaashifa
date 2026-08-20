//! WhatsApp channel abstraction, Meta Cloud API adapter, webhook receiver,
//! template registry, rate limiting, and media handling for the Shifa platform.

pub mod adapter;
pub mod cloud_api;
pub mod error;
pub mod rate_limit;
pub mod templates;
pub mod types;
pub mod webhook;

pub use adapter::ChannelAdapter;
pub use cloud_api::{CloudApiAdapter, CloudApiConfig};
pub use error::ChannelError;
pub use rate_limit::ChannelRateLimiter;
pub use templates::TemplateRegistry;
pub use types::*;
pub use webhook::{parse_inbound_webhook, verify_hub_signature};
