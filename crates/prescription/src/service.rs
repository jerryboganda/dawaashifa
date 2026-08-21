use crate::error::RxError;
use crate::extractor::{MockRxVlmProvider, RxVlmProvider};
use crate::models::*;
use crate::preprocessing::validate_and_preprocess_image;
use serde_json::json;
use shifa_catalog::service::CatalogService;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, ConversationId, CustomerId, PrescriptionId, ProductId, UserId};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PrescriptionService {
    pool: PgPool,
    catalog_service: CatalogService,
    vlm_provider: Arc<dyn RxVlmProvider>,
}

impl PrescriptionService {
    pub fn new(pool: PgPool) -> Self {
        let catalog_service = CatalogService::new(pool.clone());
        Self {
            pool,
            catalog_service,
            vlm_provider: Arc::new(MockRxVlmProvider),
        }
    }

    pub fn with_provider(pool: PgPool, provider: Arc<dyn RxVlmProvider>) -> Self {
        let catalog_service = CatalogService::new(pool.clone());
        Self {
            pool,
            catalog_service,
            vlm_provider: provider,
        }
    }

    /// Intake prescription from a WhatsApp message or customer upload per Doc 09 Â§5 & Â§6.
    pub async fn create_prescription(
        &self,
        ctx: &TenantContext,
        req: CreatePrescriptionRequest,
    ) -> Result<PrescriptionDto, RxError> {
        let prep = validate_and_preprocess_image(
            &req.image_object_key,
            req.image_width,
            req.image_height,
            req.image_bytes_len,
        )?;

        let rx_id = PrescriptionId::new();
        let channel = req.source_channel.unwrap_or_else(|| "WHATSAPP".into());

        sqlx::query(
            "INSERT INTO prescriptions (id, tenant_id, customer_id, conversation_id, branch_id, image_object_key, preprocessed_image_key, source_channel, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'RECEIVED')"
        )
        .bind(rx_id.0)
        .bind(ctx.tenant_id().0)
        .bind(req.customer_id.0)
        .bind(req.conversation_id.map(|c| c.0))
        .bind(req.branch_id.map(|b| b.0))
        .bind(&req.image_object_key)
        .bind(&prep.preprocessed_key)
        .bind(&channel)
        .execute(&self.pool)
        .await?;

        // Write initial audit log
        self.write_audit_log(
            ctx,
            rx_id,
            "PRESCRIPTION_RECEIVED",
            json!({
                "customer_id": req.customer_id.0,
                "image_key": req.image_object_key,
                "channel": channel
            }),
        )
        .await?;

        // Automatically trigger extraction pipeline
        self.extract_prescription(ctx, rx_id).await
    }

    /// Run or re-run VLM extraction and catalog matching per Doc 09 Â§7 & Â§8.
    pub async fn extract_prescription(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
    ) -> Result<PrescriptionDto, RxError> {
        let rx = self.get_prescription(ctx, rx_id).await?;

        // Mark as EXTRACTING
        sqlx::query(
            "UPDATE prescriptions SET status = 'EXTRACTING', updated_at = now() WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .execute(&self.pool)
        .await?;

        let extraction = match self
            .vlm_provider
            .extract_prescription(&rx.image_object_key)
            .await
        {
            Ok(ext) => ext,
            Err(e) => {
                tracing::warn!("VLM extraction failed for prescription {}: {}", rx_id, e);
                // Doc 09 Â§5: Extraction failure goes to PENDING_REVIEW with empty lines, not error
                sqlx::query(
                    "UPDATE prescriptions SET status = 'PENDING_REVIEW', updated_at = now() WHERE tenant_id = $1 AND id = $2"
                )
                .bind(ctx.tenant_id().0)
                .bind(rx_id.0)
                .execute(&self.pool)
                .await?;

                return self.get_prescription(ctx, rx_id).await;
            }
        };

        // Record OCR result
        sqlx::query(
            "INSERT INTO rx_ocr_results (id, tenant_id, prescription_id, model_name, model_version, raw_output, confidence_overall, processing_ms)
             VALUES ($1, $2, $3, 'shifa-vlm', 'rx_extract.v4', $4, $5, 120)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .bind(serde_json::to_value(&extraction).unwrap_or(json!({})))
        .bind(extraction.overall_confidence as f64)
        .execute(&self.pool)
        .await?;

        // Update prescription doctor & patient metadata
        sqlx::query(
            "UPDATE prescriptions
             SET doctor_name = $1, doctor_pmdc_no = $2, issued_date = $3, patient_name = $4, status = 'PENDING_REVIEW', updated_at = now()
             WHERE tenant_id = $5 AND id = $6"
        )
        .bind(&extraction.doctor_name)
        .bind(&extraction.doctor_pmdc_no)
        .bind(extraction.issued_date)
        .bind(&extraction.patient_name)
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .execute(&self.pool)
        .await?;

        // Process extracted lines and match against drug catalog
        for line in extraction.lines {
            let mut matched_pid = None;
            let mut match_conf = None;
            let mut match_meth: Option<String> = None;

            if let Some(ref drug) = line.drug_text {
                let drug_str: Option<&str> = Some(drug.as_str());
                let candidates = self
                    .catalog_service
                    .list_products(ctx, drug_str, 5, 0)
                    .await
                    .unwrap_or_default();
                if let Some(first) = candidates.first() {
                    matched_pid = Some(first.id);
                    match_conf = Some(line.confidence);
                    match_meth = Some("HYBRID_SEARCH".into());
                }
            }

            let qty: i32 = line
                .qty_text
                .as_deref()
                .and_then(|q| q.parse().ok())
                .unwrap_or(1);

            sqlx::query(
                "INSERT INTO rx_lines (id, tenant_id, prescription_id, line_no, ocr_text, matched_product_id, match_confidence, match_method, qty, dosage_instructions)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            )
            .bind(Uuid::now_v7())
            .bind(ctx.tenant_id().0)
            .bind(rx_id.0)
            .bind(line.line_no)
            .bind(&line.raw_text)
            .bind(matched_pid.map(|p| p.0))
            .bind(match_conf)
            .bind(match_meth)
            .bind(qty)
            .bind(&line.dosage_text)
            .execute(&self.pool)
            .await?;
        }

        self.get_prescription(ctx, rx_id).await
    }

    /// Claim a prescription from the review queue per Doc 09 Â§13.
    pub async fn claim_prescription(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
    ) -> Result<PrescriptionDto, RxError> {
        let updated = sqlx::query(
            "UPDATE prescriptions
             SET assigned_to = $1, status = 'UNDER_REVIEW', updated_at = now()
             WHERE tenant_id = $2 AND id = $3 AND (assigned_to IS NULL OR assigned_to = $1)",
        )
        .bind(ctx.user_id().0)
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .execute(&self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(RxError::AlreadyClaimed);
        }

        self.write_audit_log(
            ctx,
            rx_id,
            "PRESCRIPTION_CLAIMED",
            json!({ "claimed_by": ctx.user_id().0 }),
        )
        .await?;

        self.get_prescription(ctx, rx_id).await
    }

    /// Enforces licensed pharmacist approval gate (Invariant I-3 & Doc 09 Â§8).
    pub async fn approve(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
        req: ApprovePrescriptionRequest,
    ) -> Result<ApprovalResult, RxError> {
        // Invariant I-3: Only licensed pharmacist or super admin with rx.approve
        ctx.require("rx.approve")
            .map_err(|e| RxError::Unauthorized(e.to_string()))?;

        let rx = self.get_prescription(ctx, rx_id).await?;

        // Validate that EVERY line has an explicit decision (Doc 09 Â§8)
        let total_lines = rx.lines.len();
        if total_lines > 0 {
            let decided_line_numbers: std::collections::HashSet<i32> =
                req.decisions.iter().map(|d| d.line_no).collect();

            for line in &rx.lines {
                if !decided_line_numbers.contains(&line.line_no) {
                    return Err(RxError::IncompleteReview(line.line_no));
                }
            }
        }

        let mut approved_count = 0;
        let mut rejected_count = 0;
        let mut controlled_dispensed = 0;
        let mut substitutions_count = 0;

        for decision in &req.decisions {
            match &decision.action {
                LineAction::Accept => {
                    approved_count += 1;
                    sqlx::query(
                        "UPDATE rx_lines SET pharmacist_action = 'ACCEPTED' WHERE tenant_id = $1 AND prescription_id = $2 AND line_no = $3"
                    )
                    .bind(ctx.tenant_id().0)
                    .bind(rx_id.0)
                    .bind(decision.line_no)
                    .execute(&self.pool)
                    .await?;

                    // Check controlled substance
                    if let Some(line) = rx.lines.iter().find(|l| l.line_no == decision.line_no) {
                        if line.is_controlled {
                            if let Some(pid) = line.matched_product_id {
                                self.record_controlled_dispensing(ctx, rx_id, pid, line.qty, &rx)
                                    .await?;
                                controlled_dispensed += 1;
                            }
                        }
                    }
                }
                LineAction::Edit {
                    product_id,
                    qty,
                    dosage,
                } => {
                    approved_count += 1;
                    sqlx::query(
                        "UPDATE rx_lines
                         SET matched_product_id = $1, qty = $2, dosage_instructions = $3, pharmacist_action = 'EDITED'
                         WHERE tenant_id = $4 AND prescription_id = $5 AND line_no = $6"
                    )
                    .bind(product_id.0)
                    .bind(*qty)
                    .bind(dosage.as_deref())
                    .bind(ctx.tenant_id().0)
                    .bind(rx_id.0)
                    .bind(decision.line_no)
                    .execute(&self.pool)
                    .await?;

                    // Learn alias on edit (Doc 09 Â§11)
                    if let Some(line) = rx.lines.iter().find(|l| l.line_no == decision.line_no) {
                        if let Ok(prod) = self.catalog_service.get_product(ctx, *product_id).await {
                            let _ = self
                                .catalog_service
                                .learn_alias(ctx, &line.ocr_text, &prod.brand_name)
                                .await;
                        }
                    }
                }
                LineAction::Substitute { product_id, reason } => {
                    approved_count += 1;
                    substitutions_count += 1;
                    let line = rx.lines.iter().find(|l| l.line_no == decision.line_no);
                    let orig_pid = line
                        .and_then(|l| l.matched_product_id)
                        .unwrap_or(*product_id);

                    sqlx::query(
                        "UPDATE rx_lines
                         SET matched_product_id = $1, pharmacist_action = 'EDITED', pharmacist_note = $2
                         WHERE tenant_id = $3 AND prescription_id = $4 AND line_no = $5"
                    )
                    .bind(product_id.0)
                    .bind(reason)
                    .bind(ctx.tenant_id().0)
                    .bind(rx_id.0)
                    .bind(decision.line_no)
                    .execute(&self.pool)
                    .await?;

                    // Insert substitution record (Doc 09 Â§9)
                    sqlx::query(
                        "INSERT INTO rx_substitutions (id, tenant_id, prescription_id, original_product_id, substituted_product_id, pharmacist_id, reason, customer_informed)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, false)"
                    )
                    .bind(Uuid::now_v7())
                    .bind(ctx.tenant_id().0)
                    .bind(rx_id.0)
                    .bind(orig_pid.0)
                    .bind(product_id.0)
                    .bind(ctx.user_id().0)
                    .bind(reason)
                    .execute(&self.pool)
                    .await?;

                    // Learn alias on substitute (Doc 09 Â§11)
                    if let Some(l) = line {
                        if let Ok(prod) = self.catalog_service.get_product(ctx, *product_id).await {
                            let _ = self
                                .catalog_service
                                .learn_alias(ctx, &l.ocr_text, &prod.brand_name)
                                .await;
                        }
                    }
                }
                LineAction::Reject { reason } => {
                    rejected_count += 1;
                    sqlx::query(
                        "UPDATE rx_lines
                         SET pharmacist_action = 'REJECTED', pharmacist_note = $1
                         WHERE tenant_id = $2 AND prescription_id = $3 AND line_no = $4",
                    )
                    .bind(reason)
                    .bind(ctx.tenant_id().0)
                    .bind(rx_id.0)
                    .bind(decision.line_no)
                    .execute(&self.pool)
                    .await?;
                }
                LineAction::AddManual {
                    product_id,
                    qty,
                    dosage,
                } => {
                    approved_count += 1;
                    let next_line = (rx.lines.len() + 1) as i32;
                    sqlx::query(
                        "INSERT INTO rx_lines (id, tenant_id, prescription_id, line_no, ocr_text, matched_product_id, qty, dosage_instructions, pharmacist_action)
                         VALUES ($1, $2, $3, $4, 'MANUALLY_ADDED', $5, $6, $7, 'ADDED_MANUALLY')"
                    )
                    .bind(Uuid::now_v7())
                    .bind(ctx.tenant_id().0)
                    .bind(rx_id.0)
                    .bind(next_line)
                    .bind(product_id.0)
                    .bind(*qty)
                    .bind(dosage.as_deref())
                    .execute(&self.pool)
                    .await?;
                }
            }
        }

        let final_status = if rejected_count > 0 && approved_count > 0 {
            PrescriptionStatus::PartiallyApproved
        } else if approved_count > 0 {
            PrescriptionStatus::Approved
        } else {
            PrescriptionStatus::Rejected
        };

        // Update prescription status
        sqlx::query(
            "UPDATE prescriptions SET status = $1, updated_at = now() WHERE tenant_id = $2 AND id = $3"
        )
        .bind(final_status.as_str())
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .execute(&self.pool)
        .await?;

        // Insert immutable approval row (Doc 09 Â§8)
        let approval_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO pharmacist_approvals (id, tenant_id, prescription_id, user_id, decision, reason, ip, device)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(approval_id)
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .bind(ctx.user_id().0)
        .bind(if final_status == PrescriptionStatus::Rejected { "REJECTED" } else { "APPROVED" })
        .bind(req.note.as_deref())
        .bind(req.client_ip.as_deref())
        .bind(req.client_device.as_deref())
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            rx_id,
            "PRESCRIPTION_APPROVED",
            json!({
                "approval_id": approval_id,
                "status": final_status.as_str(),
                "approved_lines": approved_count,
                "rejected_lines": rejected_count,
                "controlled_dispensed": controlled_dispensed,
                "substitutions": substitutions_count,
                "pharmacist_id": ctx.user_id().0
            }),
        )
        .await?;

        Ok(ApprovalResult {
            prescription_id: rx_id,
            status: final_status,
            approved_lines_count: approved_count,
            rejected_lines_count: rejected_count,
            approval_id,
            controlled_substances_dispensed: controlled_dispensed,
            substitutions_count,
        })
    }

    /// Reject prescription outright per Doc 09 Â§13.
    pub async fn reject(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
        req: RejectPrescriptionRequest,
    ) -> Result<ApprovalResult, RxError> {
        ctx.require("rx.approve")
            .map_err(|e| RxError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "UPDATE prescriptions SET status = 'REJECTED', updated_at = now() WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .execute(&self.pool)
        .await?;

        let approval_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO pharmacist_approvals (id, tenant_id, prescription_id, user_id, decision, reason, ip, device)
             VALUES ($1, $2, $3, $4, 'REJECTED', $5, $6, $7)"
        )
        .bind(approval_id)
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .bind(ctx.user_id().0)
        .bind(&req.reason)
        .bind(req.client_ip.as_deref())
        .bind(req.client_device.as_deref())
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            rx_id,
            "PRESCRIPTION_REJECTED",
            json!({ "reason": req.reason, "pharmacist_id": ctx.user_id().0 }),
        )
        .await?;

        Ok(ApprovalResult {
            prescription_id: rx_id,
            status: PrescriptionStatus::Rejected,
            approved_lines_count: 0,
            rejected_lines_count: 0,
            approval_id,
            controlled_substances_dispensed: 0,
            substitutions_count: 0,
        })
    }

    /// Request clarification from customer per Doc 09 Â§5 & Â§13.
    pub async fn clarify(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
        req: ClarifyPrescriptionRequest,
    ) -> Result<PrescriptionDto, RxError> {
        ctx.require("rx.approve")
            .map_err(|e| RxError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "UPDATE prescriptions
             SET status = 'NEEDS_CLARIFICATION', clarification_notes = $1, updated_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(&req.question_to_customer)
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            rx_id,
            "PRESCRIPTION_CLARIFICATION_REQUESTED",
            json!({ "question": req.question_to_customer, "pharmacist_id": ctx.user_id().0 }),
        )
        .await?;

        self.get_prescription(ctx, rx_id).await
    }

    /// Queue metrics for ops dashboard per Doc 09 Â§13.
    pub async fn get_queue_stats(&self, ctx: &TenantContext) -> Result<QueueStatsDto, RxError> {
        let row = sqlx::query(
            "SELECT
                COUNT(*) FILTER (WHERE status IN ('PENDING_REVIEW', 'PENDING_OCR')) as pending_count,
                COUNT(*) FILTER (WHERE status IN ('UNDER_REVIEW', 'RX_UNDER_REVIEW')) as under_review_count,
                COUNT(*) FILTER (WHERE status = 'NEEDS_CLARIFICATION') as clarification_count,
                EXTRACT(EPOCH FROM (now() - MIN(received_at) FILTER (WHERE status IN ('PENDING_REVIEW', 'PENDING_OCR'))))::bigint as oldest_waiting
             FROM prescriptions
             WHERE tenant_id = $1"
        )
        .bind(ctx.tenant_id().0)
        .fetch_one(&self.pool)
        .await?;

        Ok(QueueStatsDto {
            total_pending: row.get("pending_count"),
            total_under_review: row.get("under_review_count"),
            total_needs_clarification: row.get("clarification_count"),
            oldest_waiting_seconds: row.get("oldest_waiting"),
        })
    }

    /// Fetch full prescription detail with lines and product info per Doc 09 Â§13.
    pub async fn get_prescription(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
    ) -> Result<PrescriptionDto, RxError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, customer_id, conversation_id, branch_id, image_object_key, preprocessed_image_key, source_channel, received_at, status::text, doctor_name, doctor_pmdc_no, issued_date, patient_name, assigned_to, clarification_notes, created_at, updated_at
             FROM prescriptions
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .fetch_optional(&self.pool)
        .await?;

        let r = match row {
            Some(r) => r,
            None => return Err(RxError::PrescriptionNotFound(rx_id)),
        };

        let status_str: String = r.get("status");
        let status: PrescriptionStatus = status_str.parse().unwrap_or(PrescriptionStatus::Received);

        let lines_rows = sqlx::query(
            "SELECT rl.id, rl.line_no, rl.ocr_text, rl.matched_product_id, rl.match_confidence, rl.match_method, rl.qty, rl.dosage_instructions, rl.pharmacist_action::text, rl.pharmacist_note, COALESCE(p.is_narcotic, false) as is_controlled, p.brand_name
             FROM rx_lines rl
             LEFT JOIN products p ON p.id = rl.matched_product_id
             WHERE rl.tenant_id = $1 AND rl.prescription_id = $2
             ORDER BY rl.line_no ASC"
        )
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .fetch_all(&self.pool)
        .await?;

        let lines = lines_rows
            .into_iter()
            .map(|lr| {
                let conf: Option<rust_decimal::Decimal> = lr.get("match_confidence");
                let mpid: Option<Uuid> = lr.get("matched_product_id");
                RxLineDto {
                    id: lr.get("id"),
                    line_no: lr.get("line_no"),
                    ocr_text: lr.get("ocr_text"),
                    matched_product_id: mpid.map(ProductId::from),
                    matched_brand_name: lr.get("brand_name"),
                    match_confidence: conf.and_then(|c| c.to_string().parse().ok()),
                    match_method: lr.get("match_method"),
                    qty: lr.get("qty"),
                    dosage_instructions: lr.get("dosage_instructions"),
                    pharmacist_action: lr.get("pharmacist_action"),
                    pharmacist_note: lr.get("pharmacist_note"),
                    is_controlled: lr.get("is_controlled"),
                }
            })
            .collect();

        let assigned_to: Option<Uuid> = r.get("assigned_to");
        let branch_id: Option<Uuid> = r.get("branch_id");
        let conv_id: Option<Uuid> = r.get("conversation_id");

        Ok(PrescriptionDto {
            id: rx_id,
            tenant_id: ctx.tenant_id(),
            customer_id: CustomerId::from(r.get::<Uuid, _>("customer_id")),
            conversation_id: conv_id.map(ConversationId::from),
            branch_id: branch_id.map(BranchId::from),
            image_object_key: r.get("image_object_key"),
            preprocessed_image_key: r.get("preprocessed_image_key"),
            source_channel: r.get("source_channel"),
            received_at: r.get("received_at"),
            status,
            doctor_name: r.get("doctor_name"),
            doctor_pmdc_no: r.get("doctor_pmdc_no"),
            issued_date: r.get("issued_date"),
            patient_name: r.get("patient_name"),
            assigned_to: assigned_to.map(UserId::from),
            clarification_notes: r.get("clarification_notes"),
            lines,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
    }

    /// List prescriptions with filters per Doc 09 Â§13.
    pub async fn list_prescriptions(
        &self,
        ctx: &TenantContext,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PrescriptionDto>, RxError> {
        let rows = if let Some(st) = status {
            sqlx::query(
                "SELECT id FROM prescriptions
                 WHERE tenant_id = $1 AND status::text = $2
                 ORDER BY received_at ASC
                 LIMIT $3 OFFSET $4",
            )
            .bind(ctx.tenant_id().0)
            .bind(st)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id FROM prescriptions
                 WHERE tenant_id = $1
                 ORDER BY received_at ASC
                 LIMIT $2 OFFSET $3",
            )
            .bind(ctx.tenant_id().0)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        let mut list = Vec::new();
        for r in rows {
            let rx_id = PrescriptionId::from(r.get::<Uuid, _>("id"));
            if let Ok(dto) = self.get_prescription(ctx, rx_id).await {
                list.push(dto);
            }
        }

        Ok(list)
    }

    /// Reconstruct immutable audit chain per Doc 09 §12 & §13.
    pub async fn get_audit_trail(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
    ) -> Result<Vec<RxAuditEntryDto>, RxError> {
        let rows = sqlx::query(
            "SELECT id, action, actor_id, occurred_at, COALESCE(after, json_build_object('reason', reason)::jsonb) as details
             FROM audit_log
             WHERE tenant_id = $1 AND entity_type = 'PRESCRIPTION' AND entity_id = $2
             ORDER BY occurred_at ASC",
        )
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .fetch_all(&self.pool)
        .await?;

        let entries = rows
            .into_iter()
            .map(|r| {
                let uid: Option<Uuid> = r.get("actor_id");
                RxAuditEntryDto {
                    id: r.get("id"),
                    action: r.get("action"),
                    actor_id: uid.map(UserId::from),
                    timestamp: r.get("occurred_at"),
                    details: r.get("details"),
                }
            })
            .collect();

        Ok(entries)
    }

    async fn record_controlled_dispensing(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
        product_id: ProductId,
        qty: i32,
        rx: &PrescriptionDto,
    ) -> Result<(), RxError> {
        sqlx::query(
            "INSERT INTO controlled_dispensing_register (id, tenant_id, prescription_id, product_id, pharmacist_id, quantity, prescriber_name, prescriber_pmdc_no, patient_name)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(rx_id.0)
        .bind(product_id.0)
        .bind(ctx.user_id().0)
        .bind(qty)
        .bind(rx.doctor_name.as_deref())
        .bind(rx.doctor_pmdc_no.as_deref())
        .bind(rx.patient_name.as_deref())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn write_audit_log(
        &self,
        ctx: &TenantContext,
        rx_id: PrescriptionId,
        action: &str,
        details: serde_json::Value,
    ) -> Result<(), RxError> {
        sqlx::query(
            "INSERT INTO audit_log (id, tenant_id, actor_id, actor_type, entity_type, entity_id, action, after, reason)
             VALUES ($1, $2, $3, 'PHARMACIST', 'PRESCRIPTION', $4, $5, $6, 'Pharmacist Prescription Action')"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(ctx.user_id().0)
        .bind(rx_id.0)
        .bind(action)
        .bind(details)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
