use crate::assignment::claim_conversation;
use crate::customer::resolve_or_create_customer;
use crate::error::ConversationError;
use crate::models::*;
use crate::routing::route_conversation;
use chrono::{Duration, Utc};
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, ConversationId, CustomerId, MessageId, TenantId, UserId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConversationService {
    pool: PgPool,
}

impl ConversationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Process inbound WhatsApp message per Doc 07 §4, §5, §6.
    pub async fn handle_inbound(
        &self,
        tenant_id: TenantId,
        req: InboundMessageRequest,
    ) -> Result<ConversationDto, ConversationError> {
        // 1. Resolve or create customer (race safe)
        let customer = resolve_or_create_customer(
            &self.pool,
            tenant_id,
            &req.msisdn,
            req.display_name.as_deref(),
        )
        .await?;

        // 2. Check for active conversation
        let active_conv = sqlx::query(
            "SELECT id, branch_id, status, assigned_to, is_rx_linked, unread_count, created_at, last_message_at
             FROM conversations
             WHERE tenant_id = $1 AND customer_id = $2
             ORDER BY updated_at DESC LIMIT 1"
        )
        .bind(tenant_id.0)
        .bind(customer.id.0)
        .fetch_optional(&self.pool)
        .await?;

        let (conv_id, branch_id, status, is_rx) = match active_conv {
            Some(row) => {
                let id: Uuid = row.get("id");
                let b_id: Option<Uuid> = row.get("branch_id");
                let st: String = row.get("status");
                let rx: bool = row.get("is_rx_linked");

                let new_status = if st == "RESOLVED" || st == "CLOSED" {
                    // Inbound message on RESOLVED / CLOSED reopens conversation as AWAITING_HUMAN
                    sqlx::query("UPDATE conversations SET status = 'AWAITING_HUMAN', updated_at = now() WHERE tenant_id = $1 AND id = $2")
                        .bind(tenant_id.0)
                        .bind(id)
                        .execute(&self.pool)
                        .await?;
                    "AWAITING_HUMAN".to_string()
                } else {
                    st
                };

                (
                    ConversationId::from(id),
                    b_id.map(BranchId::from),
                    new_status,
                    rx,
                )
            }
            None => {
                // 3. New conversation: route branch following 4-step precedence
                let routed_branch =
                    route_conversation(&self.pool, tenant_id, customer.id, req.branch_id).await?;

                let new_id = ConversationId::new();
                let initial_status = if customer.is_blocked {
                    "NEW"
                } else {
                    "AWAITING_HUMAN"
                };

                sqlx::query(
                    "INSERT INTO conversations (id, tenant_id, customer_id, branch_id, status, is_rx_linked, unread_count)
                     VALUES ($1, $2, $3, $4, $5, false, 1)"
                )
                .bind(new_id.0)
                .bind(tenant_id.0)
                .bind(customer.id.0)
                .bind(routed_branch.map(|b| b.0))
                .bind(initial_status)
                .execute(&self.pool)
                .await?;

                (new_id, routed_branch, initial_status.to_string(), false)
            }
        };

        // 4. Record inbound message
        let msg_id = MessageId::new();
        sqlx::query(
            "INSERT INTO messages (id, tenant_id, conversation_id, direction, sender_type, status, body)
             VALUES ($1, $2, $3, 'INBOUND', 'CUSTOMER', 'DELIVERED', $4)"
        )
        .bind(msg_id.0)
        .bind(tenant_id.0)
        .bind(conv_id.0)
        .bind(&req.text)
        .execute(&self.pool)
        .await?;

        // Update conversation last_message_at
        sqlx::query("UPDATE conversations SET last_message_at = now(), updated_at = now() WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id.0)
            .bind(conv_id.0)
            .execute(&self.pool)
            .await?;

        Ok(ConversationDto {
            id: conv_id,
            tenant_id,
            customer_id: customer.id,
            branch_id,
            status,
            assigned_to: None,
            is_rx_linked: is_rx,
            unread_count: 1,
            last_message_at: Some(Utc::now()),
            created_at: Utc::now(),
        })
    }

    /// Send outbound message respecting 24h WhatsApp service window per Doc 07 §8.
    pub async fn send_outbound(
        &self,
        ctx: &TenantContext,
        conversation_id: ConversationId,
        req: SendMessageRequest,
    ) -> Result<MessageDto, ConversationError> {
        ctx.require("inbox.reply")
            .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

        // Check last inbound message timestamp for 24h service window
        let last_inbound = sqlx::query(
            "SELECT created_at FROM messages
             WHERE tenant_id = $1 AND conversation_id = $2 AND direction = 'INBOUND'
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(ctx.tenant_id.0)
        .bind(conversation_id.0)
        .fetch_optional(&self.pool)
        .await?;

        let is_window_open = match last_inbound {
            Some(row) => {
                let in_time: chrono::DateTime<Utc> = row.get("created_at");
                Utc::now() - in_time < Duration::hours(24)
            }
            None => false,
        };

        if !is_window_open && !req.is_template {
            return Err(ConversationError::OutsideServiceWindow);
        }

        let msg_id = MessageId::new();
        sqlx::query(
            "INSERT INTO messages (id, tenant_id, conversation_id, direction, sender_type, sender_id, status, body)
             VALUES ($1, $2, $3, 'OUTBOUND', 'AGENT', $4, 'SENT', $5)"
        )
        .bind(msg_id.0)
        .bind(ctx.tenant_id.0)
        .bind(conversation_id.0)
        .bind(ctx.user_id.0)
        .bind(&req.body)
        .execute(&self.pool)
        .await?;

        Ok(MessageDto {
            id: msg_id,
            conversation_id,
            sender_type: "AGENT".into(),
            sender_id: Some(ctx.user_id.0),
            direction: "OUTBOUND".into(),
            status: "SENT".into(),
            body: req.body,
            original_body: None,
            overridden_by: None,
            created_at: Utc::now(),
        })
    }

    /// List conversations for inbox
    pub async fn list_conversations(
        &self,
        ctx: &TenantContext,
        branch_id: Option<BranchId>,
        status: Option<&str>,
    ) -> Result<Vec<ConversationDto>, ConversationError> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, customer_id, branch_id, status, assigned_to, is_rx_linked, unread_count, last_message_at, created_at
             FROM conversations
             WHERE tenant_id = $1
               AND ($2::uuid IS NULL OR branch_id = $2)
               AND ($3::text IS NULL OR status = $3)
             ORDER BY updated_at DESC"
        )
        .bind(ctx.tenant_id.0)
        .bind(branch_id.map(|b| b.0))
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        let dtos = rows
            .into_iter()
            .map(|r| ConversationDto {
                id: ConversationId::from(r.get::<Uuid, _>("id")),
                tenant_id: TenantId::from(r.get::<Uuid, _>("tenant_id")),
                customer_id: CustomerId::from(r.get::<Uuid, _>("customer_id")),
                branch_id: r.get::<Option<Uuid>, _>("branch_id").map(BranchId::from),
                status: r.get("status"),
                assigned_to: r.get::<Option<Uuid>, _>("assigned_to").map(UserId::from),
                is_rx_linked: r.get("is_rx_linked"),
                unread_count: r.get("unread_count"),
                last_message_at: r.get("last_message_at"),
                created_at: r.get("created_at"),
            })
            .collect();

        Ok(dtos)
    }

    /// Claim conversation atomically
    pub async fn claim(
        &self,
        ctx: &TenantContext,
        id: ConversationId,
    ) -> Result<(), ConversationError> {
        claim_conversation(ctx, &self.pool, id).await
    }

    /// Assign conversation
    pub async fn assign(
        &self,
        ctx: &TenantContext,
        id: ConversationId,
        user_id: UserId,
    ) -> Result<(), ConversationError> {
        ctx.require("inbox.assign")
            .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "UPDATE conversations
             SET assigned_to = $1, status = 'ASSIGNED', updated_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(user_id.0)
        .bind(ctx.tenant_id.0)
        .bind(id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Transfer conversation to another branch
    pub async fn transfer(
        &self,
        ctx: &TenantContext,
        id: ConversationId,
        branch_id: BranchId,
    ) -> Result<(), ConversationError> {
        ctx.require("inbox.assign")
            .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "UPDATE conversations
             SET branch_id = $1, assigned_to = NULL, status = 'AWAITING_HUMAN', updated_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(branch_id.0)
        .bind(ctx.tenant_id.0)
        .bind(id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark conversation resolved
    pub async fn resolve(
        &self,
        ctx: &TenantContext,
        id: ConversationId,
    ) -> Result<(), ConversationError> {
        ctx.require("inbox.reply")
            .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "UPDATE conversations
             SET status = 'RESOLVED', updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id.0)
        .bind(id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Escalate conversation
    pub async fn escalate(
        &self,
        ctx: &TenantContext,
        id: ConversationId,
    ) -> Result<(), ConversationError> {
        ctx.require("inbox.view")
            .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "UPDATE conversations
             SET status = 'ESCALATED', updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id.0)
        .bind(id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create canned reply
    pub async fn create_canned_reply(
        &self,
        ctx: &TenantContext,
        req: CreateCannedReplyRequest,
    ) -> Result<CannedReplyDto, ConversationError> {
        ctx.require("inbox.reply")
            .map_err(|e| ConversationError::Unauthorized(e.to_string()))?;

        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO canned_replies (id, tenant_id, branch_id, shortcode, title, body_en, body_ur, body_ur_latn)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(id)
        .bind(ctx.tenant_id.0)
        .bind(req.branch_id.map(|b| b.0))
        .bind(&req.shortcode)
        .bind(&req.title)
        .bind(&req.body_en)
        .bind(&req.body_ur)
        .bind(&req.body_ur_latn)
        .execute(&self.pool)
        .await?;

        Ok(CannedReplyDto {
            id,
            shortcode: req.shortcode,
            title: req.title,
            body_en: req.body_en,
            body_ur: req.body_ur,
            body_ur_latn: req.body_ur_latn,
        })
    }
}
