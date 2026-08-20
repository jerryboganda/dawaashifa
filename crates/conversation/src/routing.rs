use crate::error::ConversationError;
use shifa_core::id::{BranchId, CustomerId, TenantId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Route a new conversation to a branch following the 4-step precedence in Doc 07 §6.
pub async fn route_conversation(
    pool: &PgPool,
    tenant_id: TenantId,
    customer_id: CustomerId,
    explicit_branch: Option<BranchId>,
) -> Result<Option<BranchId>, ConversationError> {
    // 1. Explicit branch if customer messaged a branch-specific number
    if let Some(branch_id) = explicit_branch {
        return Ok(Some(branch_id));
    }

    // 2. Customer's last-ordered branch, if within 60 days
    let last_order_branch = sqlx::query(
        "SELECT branch_id FROM orders
         WHERE tenant_id = $1 AND customer_id = $2 AND created_at > now() - interval '60 days'
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(tenant_id.0)
    .bind(customer_id.0)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = last_order_branch {
        let b_id: Uuid = row.get("branch_id");
        return Ok(Some(BranchId::from(b_id)));
    }

    // 3. Customer's default branch
    let cust_row =
        sqlx::query("SELECT default_branch_id FROM customers WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id.0)
            .bind(customer_id.0)
            .fetch_optional(pool)
            .await?;

    if let Some(row) = cust_row {
        if let Some(def_id) = row.get::<Option<Uuid>, _>("default_branch_id") {
            return Ok(Some(BranchId::from(def_id)));
        }
    }

    // 4. Tenant default branch (first active branch)
    let default_branch = sqlx::query(
        "SELECT id FROM branches WHERE tenant_id = $1 AND status = 'ACTIVE' ORDER BY created_at ASC LIMIT 1"
    )
    .bind(tenant_id.0)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = default_branch {
        let b_id: Uuid = row.get("id");
        return Ok(Some(BranchId::from(b_id)));
    }

    Ok(None)
}
