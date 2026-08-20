use chrono::Utc;
use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::B2bError;
use crate::models::{CreatePurchaseOrderRequest, PurchaseOrderDto};

pub struct PurchaseOrderEngine;

impl PurchaseOrderEngine {
    /// Ingests purchase order document and matches against quotation (Doc 14 §7)
    pub async fn ingest_po(
        ctx: &TenantContext,
        req: CreatePurchaseOrderRequest,
        pool: &PgPool,
    ) -> Result<PurchaseOrderDto, B2bError> {
        let po_id = Uuid::now_v7();
        let po_amount = Decimal::from_str_exact(&req.amount)
            .map_err(|_| B2bError::Validation("Invalid PO amount".into()))?;

        let mut variance_detected = false;
        let mut variance_notes = None;
        let mut status = "PENDING_VERIFICATION".to_string();

        if let Some(quote_id) = req.quotation_id {
            let quote_row =
                sqlx::query("SELECT total FROM quotations WHERE tenant_id = $1 AND id = $2")
                    .bind(ctx.tenant_id().0)
                    .bind(quote_id)
                    .fetch_optional(pool)
                    .await?
                    .ok_or(B2bError::QuoteNotFound(quote_id))?;

            let quote_total: Decimal = quote_row.get("total");
            if quote_total != po_amount {
                variance_detected = true;
                variance_notes = Some(format!(
                    "PO amount (Rs {}) does not match Quote amount (Rs {})",
                    po_amount, quote_total
                ));
                status = "VARIANCE_BLOCKED".to_string();
            } else {
                status = "VERIFIED".to_string();
            }
        }

        sqlx::query(
            "INSERT INTO purchase_orders (id, tenant_id, account_id, quotation_id, po_number, po_document_key, received_at, verified_by, amount, variance_detected, variance_notes, status)
             VALUES ($1, $2, $3, $4, $5, $6, now(), $7, $8, $9, $10, $11)"
        )
        .bind(po_id)
        .bind(ctx.tenant_id().0)
        .bind(req.account_id)
        .bind(req.quotation_id)
        .bind(&req.po_number)
        .bind(req.po_document_key.as_deref())
        .bind(ctx.user_id().0)
        .bind(po_amount)
        .bind(variance_detected)
        .bind(variance_notes.as_deref())
        .bind(&status)
        .execute(pool)
        .await?;

        Ok(PurchaseOrderDto {
            id: po_id,
            tenant_id: ctx.tenant_id().0,
            account_id: req.account_id,
            quotation_id: req.quotation_id,
            po_number: req.po_number,
            po_document_key: req.po_document_key,
            received_at: Utc::now(),
            verified_by: Some(ctx.user_id().0),
            amount: po_amount.to_string(),
            variance_detected,
            variance_notes,
            status,
            created_at: Utc::now(),
        })
    }

    /// Verifies if fulfilment can proceed or is blocked by PO variance (Doc 14 §7)
    pub async fn verify_fulfilment_allowed(
        ctx: &TenantContext,
        po_id: Uuid,
        pool: &PgPool,
    ) -> Result<(), B2bError> {
        let po_row = sqlx::query(
            "SELECT status, variance_notes FROM purchase_orders WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(po_id)
        .fetch_optional(pool)
        .await?
        .ok_or(B2bError::PoNotFound(po_id))?;

        let status: String = po_row.get("status");
        let notes: Option<String> = po_row.get("variance_notes");

        if status == "VARIANCE_BLOCKED" {
            return Err(B2bError::PoVarianceBlocked(notes.unwrap_or_else(|| {
                "Purchase order has unverified variance".to_string()
            })));
        }

        Ok(())
    }
}
