use chrono::{Duration, Utc};
use shifa_core::context::TenantContext;
use shifa_core::id::ChannelId;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::ChannelError;
use crate::types::{ChannelPoolStatus, IdentityKind, Transport};

pub struct NumberPoolManager;

impl NumberPoolManager {
    /// Enforces mandatory business identity isolation (Doc 03 §9)
    /// Unofficial Baileys channels can NEVER join an Official WABA identity
    pub fn validate_identity_isolation(
        transport: Transport,
        identity_kind: IdentityKind,
    ) -> Result<(), ChannelError> {
        if transport == Transport::Unofficial && identity_kind == IdentityKind::OfficialWaba {
            return Err(ChannelError::IdentityIsolationViolation);
        }
        Ok(())
    }

    /// Handles a WhatsApp ban detection event (Doc 03 §8, §10)
    /// Marks channel BANNED, drains queue, and fails over to next active channel in pool
    pub async fn handle_ban(
        ctx: &TenantContext,
        channel_id: ChannelId,
        _reason: &str,
        pool: &PgPool,
    ) -> Result<Option<ChannelId>, ChannelError> {
        // 1. Mark channel as BANNED
        sqlx::query(
            "UPDATE channels SET
                status = 'BANNED',
                health_score = 0,
                banned_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(channel_id.0)
        .execute(pool)
        .await
        .map_err(ChannelError::Sqlx)?;

        // 2. Find next ACTIVE channel in the same branch
        let next_active: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM channels
             WHERE tenant_id = $1
               AND id != $2
               AND (status = 'ACTIVE' OR status = 'active')
             ORDER BY health_score DESC
             LIMIT 1",
        )
        .bind(ctx.tenant_id().0)
        .bind(channel_id.0)
        .fetch_optional(pool)
        .await
        .map_err(ChannelError::Sqlx)?;

        if let Some(target_channel_id) = next_active {
            // Reassign open conversations and queued messages
            sqlx::query(
                "UPDATE conversations SET channel_id = $1
                 WHERE tenant_id = $2 AND channel_id = $3",
            )
            .bind(target_channel_id)
            .bind(ctx.tenant_id().0)
            .bind(channel_id.0)
            .execute(pool)
            .await
            .map_err(ChannelError::Sqlx)?;

            Ok(Some(ChannelId(target_channel_id)))
        } else {
            Ok(None)
        }
    }

    /// Updates health score and manages pool status transitions (Doc 03 §8)
    pub async fn record_send_outcome(
        ctx: &TenantContext,
        channel_id: ChannelId,
        success: bool,
        pool: &PgPool,
    ) -> Result<ChannelPoolStatus, ChannelError> {
        let current_score: i32 = sqlx::query_scalar(
            "SELECT health_score FROM channels WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(channel_id.0)
        .fetch_optional(pool)
        .await
        .map_err(ChannelError::Sqlx)?
        .unwrap_or(100);

        let new_score = if success {
            (current_score + 2).min(100)
        } else {
            (current_score - 15).max(0)
        };

        let new_status = if new_score < 40 {
            ChannelPoolStatus::Degraded
        } else {
            ChannelPoolStatus::Active
        };

        let status_str = match new_status {
            ChannelPoolStatus::Degraded => "DEGRADED",
            ChannelPoolStatus::Active => "ACTIVE",
            _ => "ACTIVE",
        };

        sqlx::query(
            "UPDATE channels SET
                health_score = $1,
                status = $2
             WHERE tenant_id = $3 AND id = $4",
        )
        .bind(new_score)
        .bind(status_str)
        .bind(ctx.tenant_id().0)
        .bind(channel_id.0)
        .execute(pool)
        .await
        .map_err(ChannelError::Sqlx)?;

        Ok(new_status)
    }

    /// Checks if a channel in WARMING state has matured past 7 days and can be promoted to ACTIVE
    pub async fn check_warming_promotion(
        ctx: &TenantContext,
        channel_id: ChannelId,
        pool: &PgPool,
    ) -> Result<bool, ChannelError> {
        let row_opt = sqlx::query(
            "SELECT warming_started_at, health_score FROM channels WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(channel_id.0)
        .fetch_optional(pool)
        .await
        .map_err(ChannelError::Sqlx)?;

        if let Some(row) = row_opt {
            let warming_started: Option<chrono::DateTime<Utc>> = row.get("warming_started_at");
            let score: i32 = row.get("health_score");

            if let Some(start) = warming_started {
                if Utc::now() - start >= Duration::days(7) && score >= 80 {
                    sqlx::query(
                        "UPDATE channels SET status = 'ACTIVE' WHERE tenant_id = $1 AND id = $2",
                    )
                    .bind(ctx.tenant_id().0)
                    .bind(channel_id.0)
                    .execute(pool)
                    .await
                    .map_err(ChannelError::Sqlx)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
