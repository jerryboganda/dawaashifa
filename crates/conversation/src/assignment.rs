use crate::error::ConversationError;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, ConversationId, UserId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentStrategy {
    Manual,
    RoundRobin,
    LeastBusy,
}

/// Atomically claim an unassigned conversation per Doc 07 §7.
/// First writer wins, second gets Err(AlreadyClaimed) -> 409 Conflict.
pub async fn claim_conversation(
    ctx: &TenantContext,
    pool: &PgPool,
    conversation_id: ConversationId,
) -> Result<(), ConversationError> {
    ctx.require("inbox.view")
        .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

    let res = sqlx::query(
        "UPDATE conversations
         SET assigned_to = $1, status = 'ASSIGNED', updated_at = now()
         WHERE tenant_id = $2 AND id = $3 AND assigned_to IS NULL",
    )
    .bind(ctx.user_id.0)
    .bind(ctx.tenant_id.0)
    .bind(conversation_id.0)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        // Find who already claimed it
        let current =
            sqlx::query("SELECT assigned_to FROM conversations WHERE tenant_id = $1 AND id = $2")
                .bind(ctx.tenant_id.0)
                .bind(conversation_id.0)
                .fetch_optional(pool)
                .await?;

        if let Some(r) = current {
            if let Some(u_id) = r.get::<Option<Uuid>, _>("assigned_to") {
                return Err(ConversationError::AlreadyClaimed(UserId::from(u_id)));
            }
        }
        return Err(ConversationError::NotFound(conversation_id));
    }

    Ok(())
}

/// Pick eligible user using LeastBusy strategy per Doc 07 §7.
/// Selects user with `inbox.view` having fewest open ASSIGNED conversations.
pub async fn assign_least_busy(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    branch_id: BranchId,
) -> Result<Option<UserId>, ConversationError> {
    let candidate = sqlx::query(
        "SELECT u.id, COUNT(c.id) as open_count
         FROM users u
         JOIN user_branches ub ON ub.user_id = u.id AND ub.tenant_id = u.tenant_id
         LEFT JOIN conversations c ON c.assigned_to = u.id AND c.status = 'ASSIGNED'
         WHERE u.tenant_id = $1 AND ub.branch_id = $2 AND u.status = 'ACTIVE'
         GROUP BY u.id
         ORDER BY open_count ASC, u.created_at ASC
         LIMIT 1",
    )
    .bind(tenant_id)
    .bind(branch_id.0)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = candidate {
        let u_id: Uuid = row.get("id");
        return Ok(Some(UserId::from(u_id)));
    }

    Ok(None)
}
