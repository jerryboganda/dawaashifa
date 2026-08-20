use crate::error::ConversationError;
use crate::models::MessageDto;
use shifa_core::context::TenantContext;
use shifa_core::id::{ConversationId, MessageId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Override draft message in PENDING_APPROVAL status per Doc 07 Â§8.
/// Preserves original_body and records overridden_by.
pub async fn override_message(
    ctx: &TenantContext,
    pool: &PgPool,
    message_id: MessageId,
    new_body: &str,
) -> Result<MessageDto, ConversationError> {
    ctx.require("inbox.override")
        .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

    let msg_row = sqlx::query(
        "SELECT conversation_id, body, original_body, status, direction
         FROM messages
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(ctx.tenant_id().0)
    .bind(message_id.0)
    .fetch_optional(pool)
    .await?;

    let (conv_id, current_body, orig_body, status, dir) = match msg_row {
        Some(r) => (
            r.get::<Uuid, _>("conversation_id"),
            r.get::<String, _>("body"),
            r.get::<Option<String>, _>("original_body"),
            r.get::<String, _>("status"),
            r.get::<String, _>("direction"),
        ),
        None => return Err(ConversationError::MessageNotFound(message_id)),
    };

    if status != "PENDING_APPROVAL" && status != "DRAFT" {
        return Err(ConversationError::InvalidMessageStatusTransition(
            status,
            "OVERRIDDEN".into(),
        ));
    }

    let preserved_original = orig_body.unwrap_or(current_body);

    sqlx::query(
        "UPDATE messages
         SET body = $1, original_body = $2, overridden_by = $3, updated_at = now()
         WHERE tenant_id = $4 AND id = $5",
    )
    .bind(new_body)
    .bind(&preserved_original)
    .bind(ctx.user_id().0)
    .bind(ctx.tenant_id().0)
    .bind(message_id.0)
    .execute(pool)
    .await?;

    // Record training event in audit_log
    sqlx::query(
        "INSERT INTO audit_log (tenant_id, actor_id, actor_type, entity_type, entity_id, action, before, after, reason)
         VALUES ($1, $2, 'USER', 'MESSAGE', $3, 'OVERRIDE_REPLY', $4, $5, 'Pharmacist / agent human override training signal')"
    )
    .bind(ctx.tenant_id().0)
    .bind(ctx.user_id().0)
    .bind(message_id.0)
    .bind(serde_json::json!({"original": preserved_original}))
    .bind(serde_json::json!({"corrected": new_body}))
    .execute(pool)
    .await?;

    Ok(MessageDto {
        id: message_id,
        conversation_id: ConversationId::from(conv_id),
        sender_type: "AGENT".into(),
        sender_id: Some(ctx.user_id().0),
        direction: dir,
        status,
        body: new_body.to_string(),
        original_body: Some(preserved_original),
        overridden_by: Some(ctx.user_id()),
        created_at: chrono::Utc::now(),
    })
}

/// Bulk approve drafts for non-Rx conversations only per Doc 07 Â§8 and Invariant I-6.
pub async fn bulk_approve_drafts(
    ctx: &TenantContext,
    pool: &PgPool,
    conversation_id: ConversationId,
) -> Result<u64, ConversationError> {
    ctx.require("inbox.reply")
        .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

    // Invariant I-6: Rx-linked conversations cannot be bulk-approved
    let conv =
        sqlx::query("SELECT is_rx_linked FROM conversations WHERE tenant_id = $1 AND id = $2")
            .bind(ctx.tenant_id().0)
            .bind(conversation_id.0)
            .fetch_optional(pool)
            .await?;

    match conv {
        Some(c) if c.get::<bool, _>("is_rx_linked") => {
            return Err(ConversationError::BulkApprovalRejectedForRx);
        }
        None => return Err(ConversationError::NotFound(conversation_id)),
        _ => (),
    }

    let res = sqlx::query(
        "UPDATE messages
         SET status = 'APPROVED', approved_by = $1, approved_at = now()
         WHERE tenant_id = $2 AND conversation_id = $3 AND status = 'PENDING_APPROVAL'",
    )
    .bind(ctx.user_id().0)
    .bind(ctx.tenant_id().0)
    .bind(conversation_id.0)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}
