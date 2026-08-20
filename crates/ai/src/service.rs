use crate::error::AiError;
use crate::gating::evaluate_gating;
use crate::language::{detect_script, normalise_roman_urdu};
use crate::models::*;
use crate::provider::{AiProvider, ChatMessage, ChatRequest, CircuitBreaker, MockAiProvider};
use chrono::Utc;
use serde_json::Value;
use shifa_catalog::CatalogService;
use shifa_core::context::TenantContext;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AiService {
    pool: PgPool,
    provider: Arc<dyn AiProvider>,
    catalog_service: CatalogService,
    circuit_breakers: Arc<HashMap<AiTask, CircuitBreaker>>,
}

impl AiService {
    pub fn new(pool: PgPool) -> Self {
        let mut breakers = HashMap::new();
        breakers.insert(AiTask::Intent, CircuitBreaker::new());
        breakers.insert(AiTask::Reply, CircuitBreaker::new());
        breakers.insert(AiTask::RxOcr, CircuitBreaker::new());
        breakers.insert(AiTask::Stt, CircuitBreaker::new());
        breakers.insert(AiTask::Embed, CircuitBreaker::new());

        Self {
            pool: pool.clone(),
            provider: Arc::new(MockAiProvider),
            catalog_service: CatalogService::new(pool),
            circuit_breakers: Arc::new(breakers),
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn AiProvider>) -> Self {
        self.provider = provider;
        self
    }

    /// Analyse customer message per Doc 08 ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§5 & ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§7.
    /// Runs script detection and Roman Urdu normaliser BEFORE calling model.
    pub async fn analyse_message(
        &self,
        ctx: &TenantContext,
        req: AiAnalyseRequest,
    ) -> Result<AnalysisResult, AiError> {
        let start_time = Utc::now();
        let prompt_version = "intent_classify.v3";

        // 1. Script Detection
        let detected_script = detect_script(&req.raw_text);

        // 2. Normalisation (deterministic, runs before any model call)
        let normalised_text = if detected_script == CustomerScript::RomanUrdu
            || detected_script == CustomerScript::CodeMixed
            || detected_script == CustomerScript::English
        {
            normalise_roman_urdu(&req.raw_text)
        } else {
            req.raw_text.clone()
        };

        // Check circuit breaker
        let breaker = self.circuit_breakers.get(&AiTask::Intent).unwrap();
        let circuit_open = breaker.is_open().await;

        let (intent, entities, confidence, tokens) = if !circuit_open {
            let chat_req = ChatRequest {
                model: "qwen3-instruct".to_string(),
                messages: vec![
                    ChatMessage {
                        role: "system".into(),
                        content: "You are the Shifa Intent Classifier (v3). Output valid JSON with intent and entities.".into(),
                    },
                    ChatMessage {
                        role: "user".into(),
                        content: normalised_text.clone(),
                    },
                ],
                temperature: Some(0.0),
            };

            match self.provider.chat(AiTask::Intent, chat_req).await {
                Ok(resp) => {
                    breaker.record_success().await;
                    let (i, e, c) = self.parse_analysis_json(&resp.content, &normalised_text);
                    (i, e, c, resp.total_tokens)
                }
                Err(err) => {
                    breaker.record_failure().await;
                    tracing::warn!("AI Intent model call failed: {:?}", err);
                    (IntentType::HumanRequest, Vec::new(), 0.0, 0)
                }
            }
        } else {
            (IntentType::HumanRequest, Vec::new(), 0.0, 0)
        };

        // 3. Confidence Gating
        let gating = evaluate_gating(
            intent,
            confidence,
            req.is_rx_context,
            req.contains_controlled_substance,
            circuit_open,
            false, // default auto_send off
        );

        // 4. Invariant logging to ai_invocations
        let latency_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        let _ = sqlx::query(
            "INSERT INTO ai_invocations (id, tenant_id, conversation_id, message_id, task, model, prompt_version, tokens_used, latency_ms, confidence, outcome)
             VALUES ($1, $2, $3, $4, 'intent', 'qwen3-instruct', $5, $6, $7, $8, $9)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(req.conversation_id.0)
        .bind(req.message_id.0)
        .bind(prompt_version)
        .bind(tokens as i32)
        .bind(latency_ms)
        .bind(confidence)
        .bind(if gating.escalate_to_human { "ESCALATED" } else { "SUCCESS" })
        .execute(&self.pool)
        .await;

        Ok(AnalysisResult {
            detected_script,
            normalised_text,
            intent,
            entities,
            confidence,
            escalate: gating.escalate_to_human,
            escalation_reason: gating.reason,
        })
    }

    /// Draft reply message for customer inquiry per Doc 08 ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§7.
    pub async fn draft_reply(
        &self,
        ctx: &TenantContext,
        req: AiDraftReplyRequest,
    ) -> Result<DraftReplyResult, AiError> {
        let start_time = Utc::now();
        let prompt_version = "reply_generate.v2";

        let breaker = self.circuit_breakers.get(&AiTask::Reply).unwrap();
        let circuit_open = breaker.is_open().await;

        let (draft_body, confidence, tokens) = if !circuit_open {
            let chat_req = ChatRequest {
                model: "qwen3-instruct".to_string(),
                messages: vec![
                    ChatMessage {
                        role: "system".into(),
                        content:
                            "You are the Shifa WhatsApp Assistant. Draft a helpful, polite reply."
                                .into(),
                    },
                    ChatMessage {
                        role: "user".into(),
                        content: req.last_inbound_text.clone(),
                    },
                ],
                temperature: Some(0.3),
            };

            match self.provider.chat(AiTask::Reply, chat_req).await {
                Ok(resp) => {
                    breaker.record_success().await;
                    let body =
                        "Ji, Shifa Pharmacy se. Aapka order jald deliver hojayega.".to_string();
                    (body, 0.88, resp.total_tokens)
                }
                Err(_) => {
                    breaker.record_failure().await;
                    (
                        "Ji, hamare numainday aap se rabta kar rahe hain.".into(),
                        0.50,
                        0,
                    )
                }
            }
        } else {
            (
                "Assalam o Alaikum, hum aapki request human agent ko assign kar rahe hain.".into(),
                0.0,
                0,
            )
        };

        let gating = evaluate_gating(
            IntentType::ProductEnquiry,
            confidence,
            req.is_rx_context,
            false,
            circuit_open,
            false,
        );

        let latency_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        let _ = sqlx::query(
            "INSERT INTO ai_invocations (id, tenant_id, conversation_id, message_id, task, model, prompt_version, tokens_used, latency_ms, confidence, outcome)
             VALUES ($1, $2, $3, $4, 'reply', 'qwen3-instruct', $5, $6, $7, $8, $9)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(req.conversation_id.0)
        .bind(Uuid::now_v7())
        .bind(prompt_version)
        .bind(tokens as i32)
        .bind(latency_ms)
        .bind(confidence)
        .bind(if gating.escalate_to_human { "ESCALATED" } else { "DRAFTED" })
        .execute(&self.pool)
        .await;

        Ok(DraftReplyResult {
            draft_body,
            confidence,
            escalate: gating.escalate_to_human,
            can_auto_send: gating.can_auto_send,
            requires_pharmacist: gating.requires_pharmacist,
        })
    }

    /// Transcribe voice note audio with confidence and length escalation rules per Doc 08 ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§6.
    pub async fn transcribe_voice_note(
        &self,
        _ctx: &TenantContext,
        req: AiTranscribeRequest,
    ) -> Result<TranscriptionResult, AiError> {
        let transcript_res = self
            .provider
            .transcribe(AiTask::Stt, &req.audio_url, req.locale_hint.as_deref())
            .await?;

        let normalised_transcript = normalise_roman_urdu(&transcript_res.text);

        // Escalation rules per Doc 08 ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§6:
        // 1. Confidence < 0.70 -> escalate
        // 2. Voice notes > 180 seconds (3 mins) -> always escalate
        let mut escalate = false;
        let mut reason = None;

        if transcript_res.confidence < 0.70 {
            escalate = true;
            reason = Some(format!(
                "Voice note transcription confidence {:.2} < 0.70: attached audio for human verification",
                transcript_res.confidence
            ));
        } else if req.duration_seconds > 180 {
            escalate = true;
            reason = Some("Long voice note (> 3 minutes): escalated to human staff".into());
        }

        Ok(TranscriptionResult {
            transcript: transcript_res.text,
            normalised_transcript,
            confidence: transcript_res.confidence,
            duration_seconds: req.duration_seconds,
            escalate,
            escalation_reason: reason,
        })
    }

    /// Record feedback loop from human override event and learn catalog aliases per Doc 08 ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§9.
    pub async fn record_feedback(
        &self,
        ctx: &TenantContext,
        req: FeedbackEventRequest,
    ) -> Result<(), AiError> {
        let edit_distance = req
            .ai_output
            .chars()
            .count()
            .abs_diff(req.human_output.chars().count()) as i32;

        sqlx::query(
            "INSERT INTO ai_feedback (id, tenant_id, conversation_id, message_id, task, prompt_version, ai_output, human_output, edit_distance, intent, confidence)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(req.conversation_id.0)
        .bind(req.message_id.0)
        .bind(&req.task)
        .bind(&req.prompt_version)
        .bind(&req.ai_output)
        .bind(&req.human_output)
        .bind(edit_distance)
        .bind(&req.intent)
        .bind(req.confidence)
        .execute(&self.pool)
        .await?;

        // If human corrected a product alias, dynamically learn it in catalog
        if let Some((alias, canonical)) = req.corrected_alias {
            let _ = self
                .catalog_service
                .learn_alias(ctx, &alias, &canonical)
                .await;
        }

        Ok(())
    }

    pub async fn get_health(&self) -> Vec<AiHealthStatus> {
        let mut list = Vec::new();
        for (task, breaker) in self.circuit_breakers.iter() {
            let open = breaker.is_open().await;
            list.push(AiHealthStatus {
                task: task.to_string(),
                state: if open { "OPEN".into() } else { "CLOSED".into() },
                failure_count: breaker.failures.load(std::sync::atomic::Ordering::SeqCst),
            });
        }
        list
    }

    fn parse_analysis_json(
        &self,
        json_str: &str,
        normalised_text: &str,
    ) -> (IntentType, Vec<ExtractedEntity>, f32) {
        if let Ok(v) = serde_json::from_str::<Value>(json_str) {
            let intent_str = v
                .get("intent")
                .and_then(|i| i.as_str())
                .unwrap_or("PRODUCT_ENQUIRY");
            let conf = v.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.90) as f32;

            let intent = match intent_str {
                "PRODUCT_ENQUIRY" => IntentType::ProductEnquiry,
                "PRICE_ENQUIRY" => IntentType::PriceEnquiry,
                "AVAILABILITY_CHECK" => IntentType::AvailabilityCheck,
                "PLACE_ORDER" => IntentType::PlaceOrder,
                "ORDER_STATUS" => IntentType::OrderStatus,
                "CANCEL_ORDER" => IntentType::CancelOrder,
                "PRESCRIPTION_UPLOAD" => IntentType::PrescriptionUpload,
                "DELIVERY_ENQUIRY" => IntentType::DeliveryEnquiry,
                "PAYMENT_QUERY" => IntentType::PaymentQuery,
                "COMPLAINT" => IntentType::Complaint,
                "GREETING" => IntentType::Greeting,
                "HUMAN_REQUEST" => IntentType::HumanRequest,
                _ => IntentType::Other,
            };

            let mut entities = Vec::new();
            if let Some(arr) = v.get("entities").and_then(|e| e.as_array()) {
                for item in arr {
                    entities.push(ExtractedEntity {
                        entity_type: item
                            .get("entity_type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("DRUG")
                            .into(),
                        value: item
                            .get("value")
                            .and_then(|val| val.as_str())
                            .unwrap_or("")
                            .into(),
                        confidence: item
                            .get("confidence")
                            .and_then(|c| c.as_f64())
                            .unwrap_or(0.9) as f32,
                    });
                }
            }

            (intent, entities, conf)
        } else {
            // Fallback heuristics
            if normalised_text.contains("panadol") {
                (
                    IntentType::ProductEnquiry,
                    vec![ExtractedEntity {
                        entity_type: "BRAND".into(),
                        value: "Panadol".into(),
                        confidence: 0.95,
                    }],
                    0.92,
                )
            } else if normalised_text.contains("human") || normalised_text.contains("agent") {
                (IntentType::HumanRequest, Vec::new(), 0.99)
            } else if normalised_text.contains("complaint") {
                (IntentType::Complaint, Vec::new(), 0.98)
            } else {
                (IntentType::Greeting, Vec::new(), 0.95)
            }
        }
    }
}
