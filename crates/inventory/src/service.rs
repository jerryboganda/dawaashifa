use crate::error::InventoryError;
use crate::models::*;
use shifa_core::context::TenantContext;
use shifa_core::id::{BatchId, BranchId, ProductId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InventoryService {
    pool: PgPool,
}

impl InventoryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record receipt of stock batches from supplier per Doc 06 §4.
    /// Invariant I-5: Inserts into `stock_movements`, trigger projection updates `stock_current`.
    pub async fn receive_stock(
        &self,
        ctx: &TenantContext,
        req: StockReceiptRequest,
    ) -> Result<BatchId, InventoryError> {
        ctx.require("inventory.receive")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        let batch_id = BatchId::new();

        // 1. Insert or reuse batch
        sqlx::query(
            "INSERT INTO batches (id, tenant_id, product_id, batch_number, expiry_date, supplier_id, is_quarantined)
             VALUES ($1, $2, $3, $4, $5, $6, false)
             ON CONFLICT (tenant_id, product_id, batch_number) DO NOTHING"
        )
        .bind(batch_id.0)
        .bind(ctx.tenant_id().0)
        .bind(req.product_id.0)
        .bind(&req.batch_number)
        .bind(req.expiry_date)
        .bind(req.supplier_id)
        .execute(&self.pool)
        .await?;

        let actual_batch = sqlx::query(
            "SELECT id FROM batches
             WHERE tenant_id = $1 AND product_id = $2 AND batch_number = $3",
        )
        .bind(ctx.tenant_id().0)
        .bind(req.product_id.0)
        .bind(&req.batch_number)
        .fetch_one(&self.pool)
        .await?;

        let actual_batch_id: Uuid = actual_batch.get("id");

        // 2. Insert RECEIPT movement (+qty)
        sqlx::query(
            "INSERT INTO stock_movements (id, tenant_id, branch_id, product_id, batch_id, movement_type, qty_delta, created_by)
             VALUES ($1, $2, $3, $4, $5, 'RECEIPT', $6, $7)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(req.branch_id.0)
        .bind(req.product_id.0)
        .bind(actual_batch_id)
        .bind(req.qty)
        .bind(ctx.user_id().0)
        .execute(&self.pool)
        .await?;

        Ok(BatchId::from(actual_batch_id))
    }

    /// Record stock adjustment with required reason code per Doc 06 §4.
    pub async fn adjust_stock(
        &self,
        ctx: &TenantContext,
        req: StockAdjustmentRequest,
    ) -> Result<(), InventoryError> {
        ctx.require("inventory.adjust")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "INSERT INTO stock_movements (id, tenant_id, branch_id, product_id, batch_id, movement_type, qty_delta, reason, created_by)
             VALUES ($1, $2, $3, $4, $5, 'ADJUSTMENT', $6, $7, $8)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(req.branch_id.0)
        .bind(req.product_id.0)
        .bind(req.batch_id.0)
        .bind(req.qty_delta)
        .bind(&req.reason)
        .bind(ctx.user_id().0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record stock write-off for expired stock
    pub async fn write_off_expired(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
        product_id: ProductId,
        batch_id: BatchId,
        qty: i32,
        reason: &str,
    ) -> Result<(), InventoryError> {
        ctx.require("inventory.adjust")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "INSERT INTO stock_movements (id, tenant_id, branch_id, product_id, batch_id, movement_type, qty_delta, reason, created_by)
             VALUES ($1, $2, $3, $4, $5, 'EXPIRY_WRITEOFF', $6, $7, $8)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(branch_id.0)
        .bind(product_id.0)
        .bind(batch_id.0)
        .bind(-qty)
        .bind(reason)
        .bind(ctx.user_id().0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List current stock for a branch
    pub async fn list_stock(
        &self,
        ctx: &TenantContext,
        branch_id: Option<BranchId>,
        product_id: Option<ProductId>,
    ) -> Result<Vec<StockCurrentDto>, InventoryError> {
        let rows = sqlx::query(
            "SELECT sc.branch_id, sc.product_id, sc.batch_id, sc.qty, b.batch_number, b.expiry_date, b.is_quarantined
             FROM stock_current sc
             JOIN batches b ON b.id = sc.batch_id AND b.tenant_id = sc.tenant_id
             WHERE sc.tenant_id = $1
               AND ($2::uuid IS NULL OR sc.branch_id = $2)
               AND ($3::uuid IS NULL OR sc.product_id = $3)
             ORDER BY sc.qty DESC"
        )
        .bind(ctx.tenant_id().0)
        .bind(branch_id.map(|b| b.0))
        .bind(product_id.map(|p| p.0))
        .fetch_all(&self.pool)
        .await?;

        let list = rows
            .into_iter()
            .map(|r| StockCurrentDto {
                branch_id: BranchId::from(r.get::<Uuid, _>("branch_id")),
                product_id: ProductId::from(r.get::<Uuid, _>("product_id")),
                batch_id: BatchId::from(r.get::<Uuid, _>("batch_id")),
                qty: r.get("qty"),
                batch_number: r.get("batch_number"),
                expiry_date: r.get("expiry_date"),
                is_quarantined: r.get("is_quarantined"),
            })
            .collect();

        Ok(list)
    }

    /// Get total available stock for a product across branches (excluding expired/quarantined)
    pub async fn get_available_stock(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
        product_id: ProductId,
    ) -> Result<i32, InventoryError> {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(sc.qty), 0)::integer as available
             FROM stock_current sc
             JOIN batches b ON b.id = sc.batch_id AND b.tenant_id = sc.tenant_id
             WHERE sc.tenant_id = $1
               AND sc.branch_id = $2
               AND sc.product_id = $3
               AND sc.qty > 0
               AND b.expiry_date > CURRENT_DATE
               AND b.is_quarantined = false",
        )
        .bind(ctx.tenant_id().0)
        .bind(branch_id.0)
        .bind(product_id.0)
        .fetch_one(&self.pool)
        .await?;

        let available: i32 = row.get("available");
        Ok(available)
    }
}
