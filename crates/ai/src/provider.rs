use crate::error::AiError;
use crate::models::AiTask;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRequest {
    pub model: String,
    pub image_url: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub text: String,
    pub confidence: f32,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn chat(&self, task: AiTask, req: ChatRequest) -> Result<ChatResponse, AiError>;
    async fn vision(&self, task: AiTask, req: VisionRequest) -> Result<ChatResponse, AiError>;
    async fn transcribe(
        &self,
        task: AiTask,
        audio_url: &str,
        locale_hint: Option<&str>,
    ) -> Result<Transcript, AiError>;
    async fn embed(&self, task: AiTask, texts: &[String]) -> Result<Vec<Vec<f32>>, AiError>;
}

/// Task-level Circuit Breaker per Doc 08 §4: opens after 5 failures, half-opens after 30s.
#[derive(Debug)]
pub struct CircuitBreaker {
    pub failures: AtomicU32,
    pub last_failure_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            failures: AtomicU32::new(0),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn is_open(&self) -> bool {
        let count = self.failures.load(Ordering::SeqCst);
        if count >= 5 {
            let last = *self.last_failure_time.read().await;
            if let Some(t) = last {
                if Utc::now().signed_duration_since(t).num_seconds() > 30 {
                    // Half-open attempt
                    return false;
                }
            }
            return true;
        }
        false
    }

    pub async fn record_success(&self) {
        self.failures.store(0, Ordering::SeqCst);
        let mut last = self.last_failure_time.write().await;
        *last = None;
    }

    pub async fn record_failure(&self) {
        self.failures.fetch_add(1, Ordering::SeqCst);
        let mut last = self.last_failure_time.write().await;
        *last = Some(Utc::now());
    }
}

/// Mock AI Provider for deterministic testing and local offline operation
#[derive(Debug, Clone)]
pub struct MockAiProvider;

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn chat(&self, _task: AiTask, req: ChatRequest) -> Result<ChatResponse, AiError> {
        let last_msg = req
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let content = if last_msg.contains("panadol") || last_msg.contains("Panadol") {
            r#"{"intent": "PRODUCT_ENQUIRY", "entities": [{"entity_type": "BRAND", "value": "Panadol", "confidence": 0.95}], "confidence": 0.92}"#
        } else if last_msg.contains("human") || last_msg.contains("agent") {
            r#"{"intent": "HUMAN_REQUEST", "entities": [], "confidence": 0.99}"#
        } else if last_msg.contains("complaint") || last_msg.contains("kharab") {
            r#"{"intent": "COMPLAINT", "entities": [], "confidence": 0.98}"#
        } else if last_msg.contains("status") || last_msg.contains("kahan") {
            r#"{"intent": "ORDER_STATUS", "entities": [], "confidence": 0.90}"#
        } else {
            r#"{"intent": "GREETING", "entities": [], "confidence": 0.95}"#
        };

        Ok(ChatResponse {
            content: content.to_string(),
            total_tokens: 42,
        })
    }

    async fn vision(&self, _task: AiTask, _req: VisionRequest) -> Result<ChatResponse, AiError> {
        Ok(ChatResponse {
            content:
                r#"{"doctor": "Dr. Tariq", "items": [{"name": "Augmentin 625mg", "qty": 14}]}"#
                    .into(),
            total_tokens: 120,
        })
    }

    async fn transcribe(
        &self,
        _task: AiTask,
        _audio_url: &str,
        _locale_hint: Option<&str>,
    ) -> Result<Transcript, AiError> {
        Ok(Transcript {
            text: "mujhe panadol extra chahiye do dabbi".into(),
            confidence: 0.91,
        })
    }

    async fn embed(&self, _task: AiTask, texts: &[String]) -> Result<Vec<Vec<f32>>, AiError> {
        Ok(texts.iter().map(|_| vec![0.1; 128]).collect())
    }
}
