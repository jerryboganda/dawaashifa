use crate::error::InventoryError;
use chrono::{Duration, Utc};
use shifa_core::context::TenantContext;
use shifa_core::id::{BatchId, BranchId, ProductId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ReserveStockParams {
    pub order_id: Uuid,
    pub branch_id: BranchId,
    pub product_id: ProductId,
    pub batch_id: BatchId,
    pub qty: i32,
    pub ttl_minutes: i64,
}

/// Reserve stock with TTL on order confirmation per Doc 06 §6.
/// Inserts negative RESERVATION movement.
pub async fn reserve_stock(
    ctx: &TenantContext,
    pool: &PgPool,
    params: ReserveStockParams,
) -> Result<Uuid, InventoryError> {
    let reservation_id = Uuid::now_v7();
    let expires_at = Utc::now() + Duration::minutes(params.ttl_minutes);

    // 1. Insert reservation record
    sqlx::query(
        "INSERT INTO stock_reservations (id, tenant_id, order_id, branch_id, product_id, batch_id, qty, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(reservation_id)
    .bind(ctx.tenant_id().0)
    .bind(params.order_id)
    .bind(params.branch_id.0)
    .bind(params.product_id.0)
    .bind(params.batch_id.0)
    .bind(params.qty)
    .bind(expires_at)
    .execute(pool)
    .await?;

    // 2. Insert RESERVATION movement (negative delta) per Invariant I-5
    sqlx::query(
        "INSERT INTO stock_movements (id, tenant_id, branch_id, product_id, batch_id, movement_type, qty_delta, reference_id, created_by)
         VALUES ($1, $2, $3, $4, $5, 'RESERVATION', $6, $7, $8)"
    )
    .bind(Uuid::now_v7())
    .bind(ctx.tenant_id().0)
    .bind(params.branch_id.0)
    .bind(params.product_id.0)
    .bind(params.batch_id.0)
    .bind(-params.qty)
    .bind(params.order_id.to_string())
    .bind(ctx.user_id().0)
    .execute(pool)
    .await?;

    Ok(reservation_id)
}

/// Release expired reservations idempotently per Doc 06 §6.
/// Inserts compensating RELEASE movement (+qty).
pub async fn release_expired_reservations(pool: &PgPool) -> Result<usize, InventoryError> {
    let expired_rows = sqlx::query(
        "SELECT id, tenant_id, branch_id, product_id, batch_id, qty, order_id
         FROM stock_reservations
         WHERE expires_at < now() AND released_at IS NULL
         FOR UPDATE",
    )
    .fetch_all(pool)
    .await?;

    let mut count = 0;
    for row in expired_rows {
        let res_id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let branch_id: Uuid = row.get("branch_id");
        let product_id: Uuid = row.get("product_id");
        let batch_id: Uuid = row.get("batch_id");
        let qty: i32 = row.get("qty");
        let order_id: Uuid = row.get("order_id");

        // Mark released (idempotency guard)
        let res = sqlx::query(
            "UPDATE stock_reservations
             SET released_at = now()
             WHERE id = $1 AND released_at IS NULL",
        )
        .bind(res_id)
        .execute(pool)
        .await?;

        if res.rows_affected() > 0 {
            // Insert compensating RELEASE movement (+qty)
            sqlx::query(
                "INSERT INTO stock_movements (id, tenant_id, branch_id, product_id, batch_id, movement_type, qty_delta, reference_id)
                 VALUES ($1, $2, $3, $4, $5, 'RELEASE', $6, $7)"
            )
            .bind(Uuid::now_v7())
            .bind(tenant_id)
            .bind(branch_id)
            .bind(product_id)
            .bind(batch_id)
            .bind(qty)
            .bind(order_id.to_string())
            .execute(pool)
            .await?;

            count += 1;
        }
    }

    Ok(count)
}
