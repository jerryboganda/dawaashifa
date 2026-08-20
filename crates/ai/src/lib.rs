//! AI orchestration gateway, language pipeline, script detection,
//! Roman Urdu normaliser, confidence gating, and feedback loop.

pub mod error;
pub mod gating;
pub mod language;
pub mod models;
pub mod provider;
pub mod service;

pub use error::AiError;
pub use gating::{evaluate_gating, GatingEvaluation};
pub use language::{detect_script, normalise_roman_urdu};
pub use models::*;
pub use provider::{
    AiProvider, ChatMessage, ChatRequest, ChatResponse, CircuitBreaker, MockAiProvider,
};
pub use service::AiService;
