//! Identity, RBAC, users, branches, and session management for the Shifa platform.

pub mod error;
pub mod jwt;
pub mod models;
pub mod password;
pub mod roles;
pub mod service;

pub use error::AuthError;
pub use jwt::{create_access_token, generate_refresh_token, verify_access_token, Claims};
pub use models::*;
pub use password::{hash_password, validate_password_strength, verify_password};
pub use roles::{get_system_role_definitions, ALL_PERMISSIONS};
pub use service::IdentityService;
