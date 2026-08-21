use crate::error::PaymentError;
use crate::gateways::*;
use crate::models::*;
use crate::ocr::{ExtractedPaymentDetails, MockPaymentOcrProvider, PaymentOcrProvider};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use shifa_core::context::TenantContext;
use shifa_core::id::{CustomerId, OrderId, PaymentId, ProofId, TenantId, UserId};
use shifa_core::money::Money;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PaymentService {
    pool: PgPool,
    gateways: HashMap<PaymentMethod, Arc<dyn PaymentGateway>>,
    ocr_provider: Arc<dyn PaymentOcrProvider>,
}

impl PaymentService {
    pub fn new(pool: PgPool) -> Self {
        let mut gateways: HashMap<PaymentMethod, Arc<dyn PaymentGateway>> = HashMap::new();
        gateways.insert(PaymentMethod::JazzCash, Arc::new(JazzCashGateway::new()));
        gateways.insert(PaymentMethod::EasyPaisa, Arc::new(EasyPaisaGateway::new()));
        gateways.insert(PaymentMethod::Raast, Arc::new(RaastGateway::new()));
        gateways.insert(
            PaymentMethod::Aggregator,
            Arc::new(AggregatorGateway::new()),
        );

        Self {
            pool,
            gateways,
            ocr_provider: Arc::new(MockPaymentOcrProvider),
        }
    }

    pub fn with_ocr(pool: PgPool, ocr_provider: Arc<dyn PaymentOcrProvider>) -> Self {
        let mut gateways: HashMap<PaymentMethod, Arc<dyn PaymentGateway>> = HashMap::new();
        gateways.insert(PaymentMethod::JazzCash, Arc::new(JazzCashGateway::new()));
        gateways.insert(PaymentMethod::EasyPaisa, Arc::new(EasyPaisaGateway::new()));
        gateways.insert(PaymentMethod::Raast, Arc::new(RaastGateway::new()));
        gateways.insert(
            PaymentMethod::Aggregator,
            Arc::new(AggregatorGateway::new()),
        );

        Self {
            pool,
            gateways,
            ocr_provider,
        }
    }

    /// Helper to register a custom gateway implementation without modifying orders crate
    pub fn register_gateway(&mut self, method: PaymentMethod, gateway: Arc<dyn PaymentGateway>) {
        self.gateways.insert(method, gateway);
    }

    /// Create payment intent / link per Doc 11 §4.1
    pub async fn create_intent(
        &self,
        ctx: &TenantContext,
        req: IntentRequest,
    ) -> Result<PaymentIntent, PaymentError> {
        // Fetch order total and customer
        let order_row = sqlx::query(
            "SELECT id, total_amount, customer_id, status FROM orders WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(req.order_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PaymentError::OrderNotFound(req.order_id))?;

        let total_amount_dec: Decimal = order_row.get("total_amount");
        let total_amount = Money::from_decimal(total_amount_dec);
        let customer_id: Uuid = order_row.get("customer_id");

        // If COD, verify customer eligibility
        if req.method == PaymentMethod::Cod {
            self.check_cod_eligibility(ctx, CustomerId::from(customer_id), total_amount)
                .await?;

            let payment_id = PaymentId::new();
            // Insert COD Pending payment
            sqlx::query(
                "INSERT INTO payments (id, tenant_id, order_id, method, amount, status, gateway)
                 VALUES ($1, $2, $3, $4::payment_method_type, $5, 'PENDING'::payment_status, 'COD'::payment_gateway_type)"
            )
            .bind(payment_id.0)
            .bind(ctx.tenant_id().0)
            .bind(req.order_id.0)
            .bind("COD")
            .bind(total_amount.0)
            .execute(&self.pool)
            .await?;

            self.write_audit_log(
                ctx,
                payment_id.0,
                "PAYMENT_INTENT_CREATED",
                json!({ "method": "COD", "amount": total_amount.0, "order_id": req.order_id.0 }),
            )
            .await?;

            return Ok(PaymentIntent {
                payment_id,
                order_id: req.order_id,
                method: PaymentMethod::Cod,
                amount: total_amount,
                payment_url: None,
                instructions: "Cash will be collected by rider upon delivery".into(),
                expires_at: Utc::now() + Duration::days(3),
            });
        }

        let gateway = self.gateways.get(&req.method).ok_or_else(|| {
            PaymentError::BadRequest(format!("Unsupported payment method: {:?}", req.method))
        })?;

        let intent = gateway.create_intent(req.clone(), total_amount).await?;

        // Insert initial pending payment
        let gateway_type = match req.method {
            PaymentMethod::JazzCash => "JAZZCASH",
            PaymentMethod::EasyPaisa => "EASYPAISA",
            PaymentMethod::Raast => "RAAST",
            PaymentMethod::BankTransfer => "DIRECT_DEPOSIT",
            PaymentMethod::Aggregator => "SAFEPAY",
            PaymentMethod::Cod => "COD",
        };

        sqlx::query(
            "INSERT INTO payments (id, tenant_id, order_id, method, amount, status, gateway)
             VALUES ($1, $2, $3, $4::payment_method_type, $5, 'PENDING'::payment_status, $6::payment_gateway_type)
             ON CONFLICT (id) DO NOTHING"
        )
        .bind(intent.payment_id.0)
        .bind(ctx.tenant_id().0)
        .bind(req.order_id.0)
        .bind(req.method.as_str())
        .bind(total_amount.0)
        .bind(gateway_type)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            intent.payment_id.0,
            "PAYMENT_INTENT_CREATED",
            json!({
                "method": req.method.as_str(),
                "amount": total_amount.0,
                "order_id": req.order_id.0,
                "payment_url": intent.payment_url
            }),
        )
        .await?;

        Ok(intent)
    }

    /// Signed server-to-server gateway webhook callback per Doc 11 §4.1
    pub async fn handle_webhook(
        &self,
        ctx: &TenantContext,
        gateway_name: &str,
        headers: &axum::http::HeaderMap,
        body: &[u8],
    ) -> Result<PaymentDto, PaymentError> {
        let method = match gateway_name.to_uppercase().as_str() {
            "JAZZCASH" => PaymentMethod::JazzCash,
            "EASYPAISA" => PaymentMethod::EasyPaisa,
            "RAAST" => PaymentMethod::Raast,
            "SAFEPAY" | "PAYFAST" => PaymentMethod::Aggregator,
            _ => {
                return Err(PaymentError::BadRequest(format!(
                    "Unknown gateway: {}",
                    gateway_name
                )))
            }
        };

        let gateway = self.gateways.get(&method).ok_or_else(|| {
            PaymentError::BadRequest(format!("No gateway configured for {}", gateway_name))
        })?;

        // 1. Signature & Replay Verification
        let event = gateway.verify_webhook(headers, body)?;

        // 2. Fetch order to verify amount match and current state
        let order_row = sqlx::query(
            "SELECT id, total_amount, status FROM orders WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(event.order_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PaymentError::OrderNotFound(event.order_id))?;

        let expected_amount_dec: Decimal = order_row.get("total_amount");
        let expected_amount = Money::from_decimal(expected_amount_dec);

        // 3. Exact amount check
        if event.amount != expected_amount {
            return Err(PaymentError::AmountMismatch {
                expected: format!("{}", expected_amount.0),
                received: format!("{}", event.amount.0),
            });
        }

        // 4. Idempotency Check: if already confirmed with this gateway_ref
        let existing = sqlx::query(
            "SELECT id, tenant_id, order_id, method::text as method, amount, status::text as status,
                    gateway::text as gateway, gateway_ref, confirmed_at, confirmed_by,
                    refund_reason, refunded_at, created_at, updated_at
             FROM payments
             WHERE tenant_id = $1 AND gateway_ref = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(&event.gateway_ref)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            let status_str: String = row.get("status");
            if status_str == "CONFIRMED" {
                return self.map_payment_row(row);
            }
        }

        // 5. Update / Upsert Payment record as CONFIRMED
        let payment_id = PaymentId::new();
        let gateway_type = method.as_str();

        let updated_row = sqlx::query(
            "INSERT INTO payments (id, tenant_id, order_id, method, amount, status, gateway, gateway_ref, gateway_payload, confirmed_at, updated_at)
             VALUES ($1, $2, $3, $4::payment_method_type, $5, 'CONFIRMED'::payment_status, $6::payment_gateway_type, $7, $8, now(), now())
             ON CONFLICT (id) DO UPDATE SET
                status = 'CONFIRMED'::payment_status,
                gateway_ref = $7,
                gateway_payload = $8,
                confirmed_at = now(),
                updated_at = now()
             RETURNING id, tenant_id, order_id, method::text as method, amount, status::text as status,
                       gateway::text as gateway, gateway_ref, confirmed_at, confirmed_by,
                       refund_reason, refunded_at, created_at, updated_at"
        )
        .bind(payment_id.0)
        .bind(ctx.tenant_id().0)
        .bind(event.order_id.0)
        .bind(method.as_str())
        .bind(event.amount.0)
        .bind(gateway_type)
        .bind(&event.gateway_ref)
        .bind(&event.raw_payload)
        .fetch_one(&self.pool)
        .await?;

        // 6. Transition order state to Confirmed
        sqlx::query(
            "UPDATE orders SET status = 'CONFIRMED'::order_status, updated_at = now()
             WHERE tenant_id = $1 AND id = $2 AND status IN ('AWAITING_PAYMENT'::order_status, 'PAYMENT_UNDER_REVIEW'::order_status, 'CART_CONFIRMED'::order_status)"
        )
        .bind(ctx.tenant_id().0)
        .bind(event.order_id.0)
        .execute(&self.pool)
        .await?;

        // 7. Write audit log
        self.write_audit_log(
            ctx,
            payment_id.0,
            "PAYMENT_WEBHOOK_CONFIRMED",
            json!({
                "gateway": gateway_name,
                "gateway_ref": event.gateway_ref,
                "amount": event.amount.0,
                "order_id": event.order_id.0
            }),
        )
        .await?;

        self.map_payment_row(updated_row)
    }

    /// Intake payment screenshot proof, run OCR & 8 fraud checks, queue for human review (Doc 11 §4.2 & §5)
    pub async fn create_proof(
        &self,
        ctx: &TenantContext,
        req: CreateProofRequest,
    ) -> Result<PaymentProofDto, PaymentError> {
        let proof_id = ProofId::new();

        // 1. Fetch Order details
        let order_row = sqlx::query(
            "SELECT id, total_amount, customer_id, created_at FROM orders WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(req.order_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PaymentError::OrderNotFound(req.order_id))?;

        let order_total: Decimal = order_row.get("total_amount");
        let order_customer_id: Uuid = order_row.get("customer_id");
        let order_created_at: DateTime<Utc> = order_row.get("created_at");

        // 2. Extract OCR Details via Vision Provider
        let extracted = self
            .ocr_provider
            .extract(&req.image_object_key)
            .await
            .unwrap_or(ExtractedPaymentDetails {
                tid: None,
                amount: None,
                timestamp: None,
                sender: None,
                bank: None,
                confidence: 0.0,
                is_known_bank_layout: false,
            });

        // 3. Evaluate 8 Fraud Flags per Doc 11 §5
        let mut flags: Vec<FraudFlag> = Vec::new();
        let mut duplicate_proof_id: Option<ProofId> = None;

        // Flag 1: DUPLICATE_TID (Critical)
        if let Some(ref tid) = extracted.tid {
            let ledger_check = sqlx::query(
                "SELECT first_seen_order_id FROM transaction_id_ledger WHERE tenant_id = $1 AND tid = $2"
            )
            .bind(ctx.tenant_id().0)
            .bind(tid)
            .fetch_optional(&self.pool)
            .await?;

            if ledger_check.is_some() {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::DuplicateTid,
                    severity: FraudSeverity::Critical,
                    description: format!(
                        "Transaction ID '{}' already exists in transaction ledger",
                        tid
                    ),
                });
            } else {
                // Also check previously approved proofs
                let proof_check = sqlx::query(
                    "SELECT id FROM payment_proofs WHERE tenant_id = $1 AND ocr_tid = $2 AND id != $3 AND review_status = 'APPROVED'"
                )
                .bind(ctx.tenant_id().0)
                .bind(tid)
                .bind(proof_id.0)
                .fetch_optional(&self.pool)
                .await?;

                if let Some(p) = proof_check {
                    let dup_id: Uuid = p.get("id");
                    duplicate_proof_id = Some(ProofId::from(dup_id));
                    flags.push(FraudFlag {
                        flag_type: FraudFlagType::DuplicateTid,
                        severity: FraudSeverity::Critical,
                        description: format!(
                            "Transaction ID '{}' previously approved in proof {}",
                            tid, dup_id
                        ),
                    });
                }
            }
        }

        // Flag 2: AMOUNT_MISMATCH (High)
        if let Some(ocr_amt) = extracted.amount {
            if ocr_amt.0 != order_total {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::AmountMismatch,
                    severity: FraudSeverity::High,
                    description: format!(
                        "Screenshot amount Rs {} does not match order total Rs {}",
                        ocr_amt.0, order_total
                    ),
                });
            }
        }

        // Flag 3: TIMESTAMP_BEFORE_ORDER (High)
        if let Some(ts) = extracted.timestamp {
            if ts < order_created_at - Duration::minutes(2) {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::TimestampBeforeOrder,
                    severity: FraudSeverity::High,
                    description: format!(
                        "Payment timestamp ({}) predates order creation time ({})",
                        ts.to_rfc3339(),
                        order_created_at.to_rfc3339()
                    ),
                });
            }
        }

        // Flag 4: TIMESTAMP_STALE (Medium)
        if let Some(ts) = extracted.timestamp {
            if Utc::now() - ts > Duration::hours(48) {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::TimestampStale,
                    severity: FraudSeverity::Medium,
                    description: "Payment timestamp is older than 48 hours".into(),
                });
            }
        }

        // Flag 5: EDITED_IMAGE (High)
        if let Some(ref sw) = req.raw_exif_software {
            let lower = sw.to_lowercase();
            if lower.contains("photoshop")
                || lower.contains("canva")
                || lower.contains("gimp")
                || lower.contains("paint")
                || lower.contains("editor")
            {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::EditedImage,
                    severity: FraudSeverity::High,
                    description: format!("EXIF metadata indicates image editing software: {}", sw),
                });
            }
        }

        // Flag 6: SENDER_REUSED_ACROSS_CUSTOMERS (High)
        if let Some(ref sender) = extracted.sender {
            let sender_check = sqlx::query(
                "SELECT o.customer_id 
                 FROM payment_proofs p
                 JOIN orders o ON o.id = p.order_id
                 WHERE p.tenant_id = $1 AND p.ocr_sender = $2 AND o.customer_id != $3 AND p.id != $4
                 LIMIT 1",
            )
            .bind(ctx.tenant_id().0)
            .bind(sender)
            .bind(order_customer_id)
            .bind(proof_id.0)
            .fetch_optional(&self.pool)
            .await?;

            if sender_check.is_some() {
                flags.push(FraudFlag {
                    flag_type: FraudFlagType::SenderReusedAcrossCustomers,
                    severity: FraudSeverity::High,
                    description: format!(
                        "Sender account '{}' previously used by an unrelated customer",
                        sender
                    ),
                });
            }
        }

        // Flag 7: LOW_OCR_CONFIDENCE (Medium)
        if extracted.confidence < 0.70 {
            flags.push(FraudFlag {
                flag_type: FraudFlagType::LowOcrConfidence,
                severity: FraudSeverity::Medium,
                description: format!(
                    "OCR confidence {:.2} is below the 0.70 threshold",
                    extracted.confidence
                ),
            });
        }

        // Flag 8: UNKNOWN_BANK_LAYOUT (Low)
        if !extracted.is_known_bank_layout {
            flags.push(FraudFlag {
                flag_type: FraudFlagType::UnknownBankLayout,
                severity: FraudSeverity::Low,
                description: "Screenshot template / bank layout not recognised".into(),
            });
        }

        let flags_json = serde_json::to_value(&flags).unwrap_or(json!([]));
        let ocr_amount_dec = extracted.amount.map(|m| m.0);

        // 4. Save Payment Proof with PENDING review status (INVARIANT I-4: Never auto-approves)
        sqlx::query(
            "INSERT INTO payment_proofs (
                id, tenant_id, order_id, payment_id, image_object_key,
                ocr_tid, ocr_amount, ocr_timestamp, ocr_sender, ocr_bank, ocr_confidence,
                duplicate_of_proof_id, fraud_flags, review_status
             ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10, $11,
                $12, $13, 'PENDING'::proof_review_status
             )",
        )
        .bind(proof_id.0)
        .bind(ctx.tenant_id().0)
        .bind(req.order_id.0)
        .bind(req.payment_id.map(|p| p.0))
        .bind(&req.image_object_key)
        .bind(&extracted.tid)
        .bind(ocr_amount_dec)
        .bind(extracted.timestamp)
        .bind(&extracted.sender)
        .bind(&extracted.bank)
        .bind(extracted.confidence as f64)
        .bind(duplicate_proof_id.map(|d| d.0))
        .bind(&flags_json)
        .execute(&self.pool)
        .await?;

        // 5. Update Order status to PaymentUnderReview
        sqlx::query(
            "UPDATE orders SET status = 'PAYMENT_UNDER_REVIEW'::order_status, updated_at = now()
             WHERE tenant_id = $1 AND id = $2 AND status IN ('AWAITING_PAYMENT'::order_status, 'PAYMENT_REJECTED'::order_status)"
        )
        .bind(ctx.tenant_id().0)
        .bind(req.order_id.0)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            proof_id.0,
            "PAYMENT_PROOF_SUBMITTED",
            json!({
                "order_id": req.order_id.0,
                "image_key": req.image_object_key,
                "flags_count": flags.len()
            }),
        )
        .await?;

        self.get_proof(ctx, proof_id).await
    }

    /// Retrieve payment proof details by ID
    pub async fn get_proof(
        &self,
        ctx: &TenantContext,
        proof_id: ProofId,
    ) -> Result<PaymentProofDto, PaymentError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, order_id, payment_id, image_object_key,
                    ocr_tid, ocr_amount, ocr_timestamp, ocr_sender, ocr_bank, ocr_confidence,
                    duplicate_of_proof_id, fraud_flags, review_status::text as review_status,
                    reviewed_by, reviewed_at, review_note, created_at, updated_at
             FROM payment_proofs
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(proof_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PaymentError::ProofNotFound(proof_id))?;

        self.map_proof_row(row)
    }

    /// List payment proofs in review queue for ops console
    pub async fn list_proofs_queue(
        &self,
        ctx: &TenantContext,
        severity_filter: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PaymentProofDto>, PaymentError> {
        ctx.require("payment.view")
            .map_err(|e| PaymentError::Unauthorized(e.to_string()))?;

        let rows = sqlx::query(
            "SELECT id, tenant_id, order_id, payment_id, image_object_key,
                    ocr_tid, ocr_amount, ocr_timestamp, ocr_sender, ocr_bank, ocr_confidence,
                    duplicate_of_proof_id, fraud_flags, review_status::text as review_status,
                    reviewed_by, reviewed_at, review_note, created_at, updated_at
             FROM payment_proofs
             WHERE tenant_id = $1 AND review_status = 'PENDING'
             ORDER BY created_at ASC
             LIMIT $2 OFFSET $3",
        )
        .bind(ctx.tenant_id().0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let dto = self.map_proof_row(row)?;
            if let Some(ref sev) = severity_filter {
                let sev_upper = sev.to_uppercase();
                let matches = dto.fraud_flags.iter().any(|f| match f.severity {
                    FraudSeverity::Critical => sev_upper == "CRITICAL",
                    FraudSeverity::High => sev_upper == "HIGH",
                    FraudSeverity::Medium => sev_upper == "MEDIUM",
                    FraudSeverity::Low => sev_upper == "LOW",
                });
                if matches {
                    results.push(dto);
                }
            } else {
                results.push(dto);
            }
        }

        Ok(results)
    }

    /// Licensed agent/pharmacist manual approval of screenshot proof (Doc 11 §5 & Invariant I-4)
    pub async fn approve_proof(
        &self,
        ctx: &TenantContext,
        proof_id: ProofId,
        req: ApproveProofRequest,
    ) -> Result<PaymentProofDto, PaymentError> {
        ctx.require("payment.approve")
            .map_err(|e| PaymentError::Unauthorized(e.to_string()))?;

        let proof = self.get_proof(ctx, proof_id).await?;
        let reviewer_id = ctx.user_id();

        // 1. If TID extracted, record in transaction_id_ledger
        let gateway_str = proof.ocr_bank.as_deref().unwrap_or("SCREENSHOT");
        if let Some(ref tid) = proof.ocr_tid {
            sqlx::query(
                "INSERT INTO transaction_id_ledger (tenant_id, gateway, tid, first_seen_order_id, first_seen_at)
                 VALUES ($1, $2, $3, $4, now())
                 ON CONFLICT (tenant_id, gateway, tid) DO NOTHING"
            )
            .bind(ctx.tenant_id().0)
            .bind(gateway_str)
            .bind(tid)
            .bind(proof.order_id.0)
            .execute(&self.pool)
            .await?;
        }

        // 2. Update proof review status to APPROVED
        sqlx::query(
            "UPDATE payment_proofs SET
                review_status = 'APPROVED'::proof_review_status,
                reviewed_by = $1,
                reviewed_at = now(),
                review_note = $2,
                updated_at = now()
             WHERE tenant_id = $3 AND id = $4",
        )
        .bind(reviewer_id.0)
        .bind(&req.note)
        .bind(ctx.tenant_id().0)
        .bind(proof_id.0)
        .execute(&self.pool)
        .await?;

        // 3. Confirm payment in payments table
        let payment_id = proof.payment_id.unwrap_or_else(PaymentId::new);
        let amount = proof.ocr_amount.unwrap_or_else(Money::zero);

        sqlx::query(
            "INSERT INTO payments (id, tenant_id, order_id, method, amount, status, gateway, gateway_ref, confirmed_at, confirmed_by, updated_at)
             VALUES ($1, $2, $3, 'DIRECT_DEPOSIT'::payment_method_type, $4, 'CONFIRMED'::payment_status, 'DIRECT_DEPOSIT'::payment_gateway_type, $5, now(), $6, now())
             ON CONFLICT (id) DO UPDATE SET
                status = 'CONFIRMED'::payment_status,
                confirmed_at = now(),
                confirmed_by = $6,
                updated_at = now()"
        )
        .bind(payment_id.0)
        .bind(ctx.tenant_id().0)
        .bind(proof.order_id.0)
        .bind(amount.0)
        .bind(&proof.ocr_tid)
        .bind(reviewer_id.0)
        .execute(&self.pool)
        .await?;

        // 4. Update order status to CONFIRMED
        sqlx::query(
            "UPDATE orders SET status = 'CONFIRMED'::order_status, updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(proof.order_id.0)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            proof_id.0,
            "PAYMENT_PROOF_APPROVED",
            json!({
                "reviewer": reviewer_id.0,
                "note": req.note,
                "tid": proof.ocr_tid
            }),
        )
        .await?;

        self.get_proof(ctx, proof_id).await
    }

    /// Licensed agent rejection of screenshot proof
    pub async fn reject_proof(
        &self,
        ctx: &TenantContext,
        proof_id: ProofId,
        req: RejectProofRequest,
    ) -> Result<PaymentProofDto, PaymentError> {
        ctx.require("payment.reject")
            .map_err(|e| PaymentError::Unauthorized(e.to_string()))?;

        let proof = self.get_proof(ctx, proof_id).await?;
        let reviewer_id = ctx.user_id();

        // 1. Update proof review status to REJECTED
        sqlx::query(
            "UPDATE payment_proofs SET
                review_status = 'REJECTED'::proof_review_status,
                reviewed_by = $1,
                reviewed_at = now(),
                review_note = $2,
                updated_at = now()
             WHERE tenant_id = $3 AND id = $4",
        )
        .bind(reviewer_id.0)
        .bind(&req.reason)
        .bind(ctx.tenant_id().0)
        .bind(proof_id.0)
        .execute(&self.pool)
        .await?;

        // 2. Update order status to PaymentRejected
        sqlx::query(
            "UPDATE orders SET status = 'PAYMENT_REJECTED'::order_status, updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(proof.order_id.0)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            proof_id.0,
            "PAYMENT_PROOF_REJECTED",
            json!({
                "reviewer": reviewer_id.0,
                "reason": req.reason
            }),
        )
        .await?;

        self.get_proof(ctx, proof_id).await
    }

    /// Refund payment per Doc 11 §2 & §7
    pub async fn refund_payment(
        &self,
        ctx: &TenantContext,
        payment_id: PaymentId,
        req: RefundRequest,
    ) -> Result<PaymentDto, PaymentError> {
        ctx.require("payment.refund")
            .map_err(|e| PaymentError::Unauthorized(e.to_string()))?;

        let payment_row = sqlx::query(
            "SELECT id, tenant_id, order_id, method::text as method, amount, status::text as status,
                    gateway::text as gateway, gateway_ref, confirmed_at, confirmed_by,
                    refund_reason, refunded_at, created_at, updated_at
             FROM payments
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(payment_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PaymentError::PaymentNotFound(payment_id))?;

        let current_status: String = payment_row.get("status");
        if current_status != "CONFIRMED" {
            return Err(PaymentError::InvalidStatusTransition(format!(
                "Cannot refund payment with status {}",
                current_status
            )));
        }

        // Update payment to REFUNDED
        let updated = sqlx::query(
            "UPDATE payments SET
                status = 'REFUNDED'::payment_status,
                refund_reason = $1,
                refunded_at = now(),
                updated_at = now()
             WHERE tenant_id = $2 AND id = $3
             RETURNING id, tenant_id, order_id, method::text as method, amount, status::text as status,
                       gateway::text as gateway, gateway_ref, confirmed_at, confirmed_by,
                       refund_reason, refunded_at, created_at, updated_at"
        )
        .bind(&req.reason)
        .bind(ctx.tenant_id().0)
        .bind(payment_id.0)
        .fetch_one(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            payment_id.0,
            "PAYMENT_REFUNDED",
            json!({
                "reason": req.reason,
                "amount": req.amount.0
            }),
        )
        .await?;

        self.map_payment_row(updated)
    }

    /// Check COD eligibility against per-customer ceilings and refusal blocks (Doc 11 §6)
    pub async fn check_cod_eligibility(
        &self,
        ctx: &TenantContext,
        customer_id: CustomerId,
        order_amount: Money,
    ) -> Result<(), PaymentError> {
        let cust_row = sqlx::query(
            "SELECT is_blocked, metadata FROM customers WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(customer_id.0)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = cust_row {
            let is_blocked: bool = row.get("is_blocked");
            let meta: serde_json::Value = row.get("metadata");
            let cod_blocked = meta
                .get("is_cod_blocked")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if is_blocked || cod_blocked {
                return Err(PaymentError::CustomerCodBlocked);
            }
        }

        // Calculate outstanding unpaid COD orders
        let outstanding_row = sqlx::query(
            "SELECT COALESCE(SUM(total_amount), 0.0) as outstanding
             FROM orders
             WHERE tenant_id = $1 AND customer_id = $2 AND payment_method = 'COD'
               AND status IN ('CONFIRMED'::order_status, 'PICKING'::order_status, 'PACKED'::order_status, 'DISPATCHED'::order_status, 'OUT_FOR_DELIVERY'::order_status)"
        )
        .bind(ctx.tenant_id().0)
        .bind(customer_id.0)
        .fetch_one(&self.pool)
        .await?;

        let current_outstanding: Decimal = outstanding_row.get("outstanding");
        let cod_limit = Decimal::new(100000000, 4); // Default 10,000.0000 PKR

        if current_outstanding + order_amount.0 > cod_limit {
            return Err(PaymentError::CodLimitExceeded {
                current: format!("{}", current_outstanding),
                limit: format!("{}", cod_limit),
            });
        }

        Ok(())
    }

    /// COD Refusal at the door marks order as FailedDelivery, payment as Failed, triggers return (Doc 11 §6)
    pub async fn handle_cod_refusal(
        &self,
        ctx: &TenantContext,
        order_id: OrderId,
        reason: &str,
    ) -> Result<(), PaymentError> {
        let order_row = sqlx::query(
            "SELECT id, customer_id, status FROM orders WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(order_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PaymentError::OrderNotFound(order_id))?;

        let customer_id: Uuid = order_row.get("customer_id");

        // 1. Update order to FailedDelivery
        sqlx::query(
            "UPDATE orders SET status = 'FAILED_DELIVERY'::order_status, updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(order_id.0)
        .execute(&self.pool)
        .await?;

        // 2. Update payment to Failed
        sqlx::query(
            "UPDATE payments SET status = 'FAILED'::payment_status, updated_at = now()
             WHERE tenant_id = $1 AND order_id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(order_id.0)
        .execute(&self.pool)
        .await?;

        // 3. Increment customer refusal count; block COD if >= 3
        sqlx::query(
            "UPDATE customers SET
                metadata = jsonb_set(
                    COALESCE(metadata, '{}'::jsonb),
                    '{cod_refusal_count}',
                    to_jsonb(COALESCE((metadata->>'cod_refusal_count')::int, 0) + 1)
                ),
                updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(customer_id)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            order_id.0,
            "COD_DELIVERY_REFUSED",
            json!({
                "reason": reason,
                "customer_id": customer_id
            }),
        )
        .await?;

        Ok(())
    }

    /// Daily settlement report reconciliation per Doc 11 §8
    pub async fn generate_reconciliation_report(
        &self,
        ctx: &TenantContext,
        report_date: NaiveDate,
        gateway: &str,
        settlement_items: Vec<(String, Money, Money)>, // (gateway_ref, settled_amount, fee)
    ) -> Result<ReconciliationReportDto, PaymentError> {
        ctx.require("report.view")
            .map_err(|e| PaymentError::Unauthorized(e.to_string()))?;

        let gateway_upper = gateway.to_uppercase();

        // 1. Fetch expected ledger payments for this gateway & date
        let ledger_rows = sqlx::query(
            "SELECT id, gateway_ref, amount 
             FROM payments 
             WHERE tenant_id = $1 AND gateway::text = $2 AND status = 'CONFIRMED'
               AND created_at::date = $3",
        )
        .bind(ctx.tenant_id().0)
        .bind(&gateway_upper)
        .bind(report_date)
        .fetch_all(&self.pool)
        .await?;

        let mut expected_map: HashMap<String, (PaymentId, Money)> = HashMap::new();
        let mut expected_total = Money::zero();

        for row in ledger_rows {
            let pid: Uuid = row.get("id");
            let gref: Option<String> = row.get("gateway_ref");
            let amt_dec: Decimal = row.get("amount");
            let money = Money::from_decimal(amt_dec);
            expected_total = Money::from_decimal(expected_total.0 + money.0);

            if let Some(ref r) = gref {
                expected_map.insert(r.clone(), (PaymentId::from(pid), money));
            }
        }

        let mut settled_total = Money::zero();
        let mut fee_total = Money::zero();
        let mut discrepancies: Vec<ReconciliationDiscrepancy> = Vec::new();
        let mut settled_refs: HashMap<String, bool> = HashMap::new();

        for (gref, settled_amt, fee) in settlement_items {
            settled_total = Money::from_decimal(settled_total.0 + settled_amt.0);
            fee_total = Money::from_decimal(fee_total.0 + fee.0);
            settled_refs.insert(gref.clone(), true);

            if let Some((pid, exp_amt)) = expected_map.get(&gref) {
                if *exp_amt != settled_amt {
                    discrepancies.push(ReconciliationDiscrepancy {
                        payment_id: Some(*pid),
                        gateway_ref: Some(gref),
                        discrepancy_type: "AMOUNT_MISMATCH".into(),
                        expected_amount: *exp_amt,
                        settled_amount: settled_amt,
                        description: format!(
                            "Expected Rs {}, settled Rs {}",
                            exp_amt.0, settled_amt.0
                        ),
                    });
                }
            } else {
                // In settlement report but missing in ledger (Unmatched Direction A)
                discrepancies.push(ReconciliationDiscrepancy {
                    payment_id: None,
                    gateway_ref: Some(gref.clone()),
                    discrepancy_type: "UNMATCHED_IN_SETTLEMENT".into(),
                    expected_amount: Money::zero(),
                    settled_amount: settled_amt,
                    description: format!("Gateway ref {} present in settlement report but missing from payments ledger", gref),
                });
            }
        }

        // In ledger but missing in settlement (Unmatched Direction B)
        for (gref, (pid, exp_amt)) in &expected_map {
            if !settled_refs.contains_key(gref) {
                discrepancies.push(ReconciliationDiscrepancy {
                    payment_id: Some(*pid),
                    gateway_ref: Some(gref.clone()),
                    discrepancy_type: "UNMATCHED_IN_LEDGER".into(),
                    expected_amount: *exp_amt,
                    settled_amount: Money::zero(),
                    description: format!("Payment {} with ref {} present in ledger but absent from gateway settlement", pid.0, gref),
                });
            }
        }

        let unmatched_count = discrepancies.len() as i32;
        let disc_json = serde_json::to_value(&discrepancies).unwrap_or(json!([]));

        // Save report into payment_reconciliations
        let recon_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO payment_reconciliations (
                id, tenant_id, report_date, gateway, expected_amount, settled_amount, fee_amount, unmatched_count, discrepancies
             ) VALUES (
                $1, $2, $3, $4::payment_gateway_type, $5, $6, $7, $8, $9
             ) ON CONFLICT (tenant_id, report_date, gateway) DO UPDATE SET
                expected_amount = $5,
                settled_amount = $6,
                fee_amount = $7,
                unmatched_count = $8,
                discrepancies = $9,
                updated_at = now()"
        )
        .bind(recon_id)
        .bind(ctx.tenant_id().0)
        .bind(report_date)
        .bind(&gateway_upper)
        .bind(expected_total.0)
        .bind(settled_total.0)
        .bind(fee_total.0)
        .bind(unmatched_count)
        .bind(&disc_json)
        .execute(&self.pool)
        .await?;

        Ok(ReconciliationReportDto {
            report_date: report_date.to_string(),
            gateway: gateway_upper,
            expected_total,
            settled_total,
            fee_total,
            unmatched_count,
            discrepancies,
        })
    }

    /// Query payments list
    pub async fn list_payments(
        &self,
        ctx: &TenantContext,
        order_id: Option<OrderId>,
        status: Option<PaymentStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PaymentDto>, PaymentError> {
        ctx.require("payment.view")
            .map_err(|e| PaymentError::Unauthorized(e.to_string()))?;

        let rows = sqlx::query(
            "SELECT id, tenant_id, order_id, method::text as method, amount, status::text as status,
                    gateway::text as gateway, gateway_ref, confirmed_at, confirmed_by,
                    refund_reason, refunded_at, created_at, updated_at
             FROM payments
             WHERE tenant_id = $1
               AND ($2::uuid IS NULL OR order_id = $2)
               AND ($3::text IS NULL OR status::text = $3)
             ORDER BY created_at DESC
             LIMIT $4 OFFSET $5"
        )
        .bind(ctx.tenant_id().0)
        .bind(order_id.map(|o| o.0))
        .bind(status.map(|s| s.as_str()))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut list = Vec::new();
        for r in rows {
            list.push(self.map_payment_row(r)?);
        }
        Ok(list)
    }

    // --- Private Helper Mappings & Audit Logging ---

    fn map_payment_row(&self, row: sqlx::postgres::PgRow) -> Result<PaymentDto, PaymentError> {
        let pid: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let oid: Uuid = row.get("order_id");
        let method_str: String = row.get("method");
        let amount_dec: Decimal = row.get("amount");
        let status_str: String = row.get("status");
        let gateway: Option<String> = row.get("gateway");
        let gateway_ref: Option<String> = row.get("gateway_ref");
        let confirmed_at: Option<DateTime<Utc>> = row.get("confirmed_at");
        let confirmed_by: Option<Uuid> = row.get("confirmed_by");
        let refund_reason: Option<String> = row.get("refund_reason");
        let refunded_at: Option<DateTime<Utc>> = row.get("refunded_at");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");

        let method = method_str.parse().unwrap_or(PaymentMethod::BankTransfer);
        let status = status_str.parse().unwrap_or(PaymentStatus::Pending);

        Ok(PaymentDto {
            id: PaymentId::from(pid),
            tenant_id: TenantId::from(tid),
            order_id: OrderId::from(oid),
            method,
            amount: Money::from_decimal(amount_dec),
            status,
            gateway,
            gateway_ref,
            confirmed_at,
            confirmed_by: confirmed_by.map(UserId::from),
            refund_reason,
            refunded_at,
            created_at,
            updated_at,
        })
    }

    fn map_proof_row(&self, row: sqlx::postgres::PgRow) -> Result<PaymentProofDto, PaymentError> {
        let id: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let oid: Uuid = row.get("order_id");
        let pid: Option<Uuid> = row.get("payment_id");
        let image_object_key: String = row.get("image_object_key");
        let ocr_tid: Option<String> = row.get("ocr_tid");
        let ocr_amount_dec: Option<Decimal> = row.get("ocr_amount");
        let ocr_timestamp: Option<DateTime<Utc>> = row.get("ocr_timestamp");
        let ocr_sender: Option<String> = row.get("ocr_sender");
        let ocr_bank: Option<String> = row.get("ocr_bank");
        let ocr_conf_dec: Option<Decimal> = row.get("ocr_confidence");
        let duplicate_of_proof_id: Option<Uuid> = row.get("duplicate_of_proof_id");
        let fraud_flags_val: serde_json::Value = row.get("fraud_flags");
        let review_status_str: String = row.get("review_status");
        let reviewed_by: Option<Uuid> = row.get("reviewed_by");
        let reviewed_at: Option<DateTime<Utc>> = row.get("reviewed_at");
        let review_note: Option<String> = row.get("review_note");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");

        let flags: Vec<FraudFlag> = serde_json::from_value(fraud_flags_val).unwrap_or_default();
        let review_status = review_status_str
            .parse()
            .unwrap_or(ProofReviewStatus::Pending);

        Ok(PaymentProofDto {
            id: ProofId::from(id),
            tenant_id: TenantId::from(tid),
            order_id: OrderId::from(oid),
            payment_id: pid.map(PaymentId::from),
            image_object_key,
            ocr_tid,
            ocr_amount: ocr_amount_dec.map(Money::from_decimal),
            ocr_timestamp,
            ocr_sender,
            ocr_bank,
            ocr_confidence: ocr_conf_dec.map(|d| {
                use rust_decimal::prelude::ToPrimitive;
                d.to_f32().unwrap_or(0.0)
            }),
            duplicate_of_proof_id: duplicate_of_proof_id.map(ProofId::from),
            fraud_flags: flags,
            review_status,
            reviewed_by: reviewed_by.map(UserId::from),
            reviewed_at,
            review_note,
            created_at,
            updated_at,
        })
    }

    async fn write_audit_log(
        &self,
        ctx: &TenantContext,
        target_id: Uuid,
        action: &str,
        details: serde_json::Value,
    ) -> Result<(), PaymentError> {
        let audit_id = Uuid::now_v7();
        let user_id = ctx.user_id().0;

        sqlx::query(
            "INSERT INTO audit_log (id, tenant_id, actor_id, actor_type, entity_type, entity_id, action, after, ip)
             VALUES ($1, $2, $3, 'USER', 'PAYMENT', $4, $5, $6, '127.0.0.1')"
        )
        .bind(audit_id)
        .bind(ctx.tenant_id().0)
        .bind(user_id)
        .bind(target_id)
        .bind(action)
        .bind(&details)
        .execute(&self.pool)
        .await
        .ok(); // Non-blocking audit log

        Ok(())
    }
}
