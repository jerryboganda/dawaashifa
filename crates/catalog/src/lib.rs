//! Catalog, Drug Master, MRP enforcement, and four-signal product matching engine
//! for the Shifa pharmacy platform.

pub mod alias;
pub mod error;
pub mod matching;
pub mod models;
pub mod mrp;
pub mod phonetics;
pub mod service;
pub mod substitutions;

pub use alias::learn_alias;
pub use error::CatalogError;
pub use matching::match_product;
pub use models::*;
pub use mrp::validate_sale_price;
pub use phonetics::{encode_urdu_phonetic, normalize_query};
pub use service::CatalogService;
pub use substitutions::substitution_candidates;
