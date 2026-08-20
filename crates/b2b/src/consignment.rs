use chrono::Utc;
use shifa_core::context::TenantContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::B2bError;
use crate::models::{ConsignmentStockDto, PlaceConsignmentRequest};

pub struct ConsignmentManager;

impl ConsignmentManager {
    /// Places consignment stock at a hospital location (Transfer, NOT a sale) (Doc 14 §10)
    pub async fn place_stock(
        ctx: &TenantContext,
        req: PlaceConsignmentRequest,
        pool: &PgPool,
    ) -> Result<ConsignmentStockDto, B2bError> {
        let stock_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO consignment_stock (id, tenant_id, location_id, product_id, batch_id, serial_no, qty, placed_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, now())"
        )
        .bind(stock_id)
        .bind(ctx.tenant_id().0)
        .bind(req.location_id)
        .bind(req.product_id)
        .bind(req.batch_id)
        .bind(req.serial_no.as_deref())
        .bind(req.qty)
        .execute(pool)
        .await?;

        Ok(ConsignmentStockDto {
            id: stock_id,
            tenant_id: ctx.tenant_id().0,
            location_id: req.location_id,
            product_id: req.product_id,
            batch_id: req.batch_id,
            serial_no: req.serial_no,
            qty: req.qty,
            placed_at: Utc::now(),
            consumed_at: None,
            invoiced_at: None,
            discrepancy_flagged: false,
            discrepancy_reason: None,
            created_at: Utc::now(),
        })
    }

    /// Reconciles physical count against expected quantity without auto-adjustment (Doc 14 §10)
    pub async fn reconcile_stock(
        ctx: &TenantContext,
        stock_id: Uuid,
        physical_count: i32,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<ConsignmentStockDto, B2bError> {
        let stock_row = sqlx::query(
            "SELECT location_id, product_id, batch_id, serial_no, qty, placed_at, consumed_at, invoiced_at FROM consignment_stock
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(stock_id)
        .fetch_optional(pool)
        .await?
        .ok_or(B2bError::ConsignmentNotFound(stock_id))?;

        let expected_qty: i32 = stock_row.get("qty");
        let mut discrepancy = false;
        let mut reason = None;

        if physical_count != expected_qty {
            discrepancy = true;
            reason = Some(format!(
                "Discrepancy detected: physical count {} vs expected count {}. Notes: {}",
                physical_count,
                expected_qty,
                notes.unwrap_or("None")
            ));

            // Flag discrepancy, DO NOT auto-adjust qty (Doc 14 §10)
            sqlx::query(
                "UPDATE consignment_stock SET discrepancy_flagged = true, discrepancy_reason = $1
                 WHERE tenant_id = $2 AND id = $3",
            )
            .bind(&reason)
            .bind(ctx.tenant_id().0)
            .bind(stock_id)
            .execute(pool)
            .await?;
        }

        Ok(ConsignmentStockDto {
            id: stock_id,
            tenant_id: ctx.tenant_id().0,
            location_id: stock_row.get("location_id"),
            product_id: stock_row.get("product_id"),
            batch_id: stock_row.get("batch_id"),
            serial_no: stock_row.get("serial_no"),
            qty: expected_qty,
            placed_at: stock_row.get("placed_at"),
            consumed_at: stock_row.get("consumed_at"),
            invoiced_at: stock_row.get("invoiced_at"),
            discrepancy_flagged: discrepancy,
            discrepancy_reason: reason,
            created_at: Utc::now(),
        })
    }
}
