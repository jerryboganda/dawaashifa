use crate::error::InventoryError;
use crate::models::{CreateTransferRequest, TransferDto};
use chrono::Utc;
use shifa_core::context::TenantContext;
use shifa_core::id::BranchId;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Inter-branch stock transfer workflow per Doc 06 §7:
/// State machine: DRAFT -> DISPATCHED -> IN_TRANSIT -> RECEIVED / DISCREPANCY
#[derive(Debug, Clone)]
pub struct TransferService {
    pool: PgPool,
}

impl TransferService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create new draft transfer
    pub async fn create_transfer(
        &self,
        ctx: &TenantContext,
        req: CreateTransferRequest,
    ) -> Result<TransferDto, InventoryError> {
        ctx.require("inventory.transfer")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        let transfer_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO stock_transfers (id, tenant_id, source_branch_id, target_branch_id, status, notes)
             VALUES ($1, $2, $3, $4, 'DRAFT', $5)"
        )
        .bind(transfer_id)
        .bind(ctx.tenant_id().0)
        .bind(req.source_branch_id.0)
        .bind(req.target_branch_id.0)
        .bind(req.note)
        .execute(&self.pool)
        .await?;

        for item in req.items {
            sqlx::query(
                "INSERT INTO stock_transfer_items (id, tenant_id, transfer_id, product_id, batch_id, requested_qty)
                 VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(Uuid::now_v7())
            .bind(ctx.tenant_id().0)
            .bind(transfer_id)
            .bind(item.product_id.0)
            .bind(item.batch_id.0)
            .bind(item.qty)
            .execute(&self.pool)
            .await?;
        }

        Ok(TransferDto {
            id: transfer_id,
            tenant_id: ctx.tenant_id(),
            source_branch_id: req.source_branch_id,
            target_branch_id: req.target_branch_id,
            status: "DRAFT".to_string(),
            created_at: Utc::now(),
        })
    }

    /// Dispatch transfer: deducts stock from source branch with `TRANSFER_OUT` movement.
    /// Invariant: In transit stock belongs to neither branch's available pool.
    pub async fn dispatch_transfer(
        &self,
        ctx: &TenantContext,
        transfer_id: Uuid,
    ) -> Result<TransferDto, InventoryError> {
        ctx.require("inventory.transfer")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        let transfer_row = sqlx::query(
            "SELECT source_branch_id, target_branch_id, status
             FROM stock_transfers
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await?;

        let (src_id, tgt_id, status) = match transfer_row {
            Some(r) => (
                r.get::<Uuid, _>("source_branch_id"),
                r.get::<Uuid, _>("target_branch_id"),
                r.get::<String, _>("status"),
            ),
            None => return Err(InventoryError::TransferNotFound(transfer_id)),
        };

        if status != "DRAFT" {
            return Err(InventoryError::InvalidTransferState(
                status,
                "DISPATCHED".into(),
            ));
        }

        let items = sqlx::query(
            "SELECT product_id, batch_id, requested_qty
             FROM stock_transfer_items
             WHERE tenant_id = $1 AND transfer_id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(transfer_id)
        .fetch_all(&self.pool)
        .await?;

        // Deduct from source branch with TRANSFER_OUT
        for item in items {
            let pid: Uuid = item.get("product_id");
            let bid: Uuid = item.get("batch_id");
            let qty: i32 = item.get("requested_qty");

            sqlx::query(
                "INSERT INTO stock_movements (id, tenant_id, branch_id, product_id, batch_id, movement_type, qty_delta, reference_id, created_by)
                 VALUES ($1, $2, $3, $4, $5, 'TRANSFER_OUT', $6, $7, $8)"
            )
            .bind(Uuid::now_v7())
            .bind(ctx.tenant_id().0)
            .bind(src_id)
            .bind(pid)
            .bind(bid)
            .bind(-qty)
            .bind(transfer_id.to_string())
            .bind(ctx.user_id().0)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query(
            "UPDATE stock_transfers
             SET status = 'IN_TRANSIT', dispatched_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(transfer_id)
        .execute(&self.pool)
        .await?;

        Ok(TransferDto {
            id: transfer_id,
            tenant_id: ctx.tenant_id(),
            source_branch_id: BranchId::from(src_id),
            target_branch_id: BranchId::from(tgt_id),
            status: "IN_TRANSIT".to_string(),
            created_at: Utc::now(),
        })
    }

    /// Receive transfer: adds stock to target branch with `TRANSFER_IN` movement.
    /// If quantity mismatch occurs, sets status to `DISCREPANCY`.
    pub async fn receive_transfer(
        &self,
        ctx: &TenantContext,
        transfer_id: Uuid,
        received_items: Vec<(Uuid, Uuid, i32)>, // (product_id, batch_id, received_qty)
    ) -> Result<TransferDto, InventoryError> {
        ctx.require("inventory.receive")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        let transfer_row = sqlx::query(
            "SELECT source_branch_id, target_branch_id, status
             FROM stock_transfers
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(transfer_id)
        .fetch_optional(&self.pool)
        .await?;

        let (src_id, tgt_id, status) = match transfer_row {
            Some(r) => (
                r.get::<Uuid, _>("source_branch_id"),
                r.get::<Uuid, _>("target_branch_id"),
                r.get::<String, _>("status"),
            ),
            None => return Err(InventoryError::TransferNotFound(transfer_id)),
        };

        if status != "IN_TRANSIT" && status != "DISPATCHED" {
            return Err(InventoryError::InvalidTransferState(
                status,
                "RECEIVED".into(),
            ));
        }

        let expected_items = sqlx::query(
            "SELECT product_id, batch_id, requested_qty
             FROM stock_transfer_items
             WHERE tenant_id = $1 AND transfer_id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(transfer_id)
        .fetch_all(&self.pool)
        .await?;

        let mut has_discrepancy = false;

        for expected in expected_items {
            let pid: Uuid = expected.get("product_id");
            let bid: Uuid = expected.get("batch_id");
            let exp_qty: i32 = expected.get("requested_qty");

            let rec_qty = received_items
                .iter()
                .find(|(p, b, _)| *p == pid && *b == bid)
                .map(|(_, _, q)| *q)
                .unwrap_or(0);

            if rec_qty != exp_qty {
                has_discrepancy = true;
            }

            // Insert TRANSFER_IN for the actual received quantity
            if rec_qty > 0 {
                sqlx::query(
                    "INSERT INTO stock_movements (id, tenant_id, branch_id, product_id, batch_id, movement_type, qty_delta, reference_id, created_by)
                     VALUES ($1, $2, $3, $4, $5, 'TRANSFER_IN', $6, $7, $8)"
                )
                .bind(Uuid::now_v7())
                .bind(ctx.tenant_id().0)
                .bind(tgt_id)
                .bind(pid)
                .bind(bid)
                .bind(rec_qty)
                .bind(transfer_id.to_string())
                .bind(ctx.user_id().0)
                .execute(&self.pool)
                .await?;
            }
        }

        let final_status = if has_discrepancy {
            "DISCREPANCY"
        } else {
            "RECEIVED"
        };

        sqlx::query(
            "UPDATE stock_transfers
             SET status = $1, received_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(final_status)
        .bind(ctx.tenant_id().0)
        .bind(transfer_id)
        .execute(&self.pool)
        .await?;

        Ok(TransferDto {
            id: transfer_id,
            tenant_id: ctx.tenant_id(),
            source_branch_id: BranchId::from(src_id),
            target_branch_id: BranchId::from(tgt_id),
            status: final_status.to_string(),
            created_at: Utc::now(),
        })
    }
}
