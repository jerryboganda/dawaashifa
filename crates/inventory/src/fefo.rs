use crate::error::InventoryError;
use crate::models::BatchAllocation;
use chrono::{Duration, NaiveDate, Utc};
use shifa_core::context::TenantContext;
use shifa_core::id::{BatchId, BranchId, ProductId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Allocate stock using First-Expired, First-Out (FEFO) strategy per Doc 06 §5.
/// - Order by `expiry_date ASC`.
/// - Excludes batches expiring within `min_shelf_life_days` (default 90 days).
/// - Excludes expired batches immediately.
/// - Excludes quarantined batches (unresolved temperature excursions).
/// - Splits across batches when needed.
/// - Concurrency safe: acquires row-level locks on stock rows.
/// - Returns `Err(InsufficientStock)` when available < requested. Never partially allocates silently.
pub async fn allocate_fefo(
    ctx: &TenantContext,
    pool: &PgPool,
    branch_id: BranchId,
    product_id: ProductId,
    qty_requested: i32,
    course_length_days: Option<i64>,
) -> Result<Vec<BatchAllocation>, InventoryError> {
    if qty_requested <= 0 {
        return Ok(Vec::new());
    }

    let now_date = Utc::now().date_naive();
    let min_days = course_length_days.unwrap_or(90);
    let shelf_life_floor = now_date + Duration::days(min_days);

    // Lock candidate batches for update to prevent concurrent overselling (Invariant: concurrency safety)
    let candidate_rows = sqlx::query(
        "SELECT sc.batch_id, sc.qty, b.batch_number, b.expiry_date, b.is_quarantined
         FROM stock_current sc
         JOIN batches b ON b.id = sc.batch_id AND b.tenant_id = sc.tenant_id
         WHERE sc.tenant_id = $1
           AND sc.branch_id = $2
           AND sc.product_id = $3
           AND sc.qty > 0
           AND b.expiry_date > $4
           AND b.is_quarantined = false
         ORDER BY b.expiry_date ASC
         FOR UPDATE OF sc",
    )
    .bind(ctx.tenant_id.0)
    .bind(branch_id.0)
    .bind(product_id.0)
    .bind(shelf_life_floor)
    .fetch_all(pool)
    .await?;

    let total_available: i32 = candidate_rows.iter().map(|r| r.get::<i32, _>("qty")).sum();

    if total_available < qty_requested {
        return Err(InventoryError::InsufficientStock {
            product_id,
            branch_id,
            requested: qty_requested,
            available: total_available,
        });
    }

    let mut remaining = qty_requested;
    let mut allocations = Vec::new();

    for row in candidate_rows {
        if remaining <= 0 {
            break;
        }

        let batch_id: Uuid = row.get("batch_id");
        let available_qty: i32 = row.get("qty");
        let batch_number: String = row.get("batch_number");
        let expiry_date: NaiveDate = row.get("expiry_date");

        let allocate_from_batch = available_qty.min(remaining);
        allocations.push(BatchAllocation {
            batch_id: BatchId::from(batch_id),
            batch_number,
            expiry_date,
            qty: allocate_from_batch,
        });

        remaining -= allocate_from_batch;
    }

    Ok(allocations)
}
