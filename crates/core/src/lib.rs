//! Core domain types, newtype identifiers, monetary representation,
//! tenant context, and error primitives for the Shifa platform.

pub mod context;
pub mod error;
pub mod id;
pub mod money;

pub use context::TenantContext;
pub use error::CoreError;
pub use id::*;
pub use money::Money;
