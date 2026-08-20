use crate::error::InventoryError;
use crate::models::{ClearExcursionRequest, ColdChainLogRequest};
use shifa_core::context::TenantContext;
use shifa_core::id::BatchId;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Cold chain temperature monitoring and quarantine enforcement per Doc 06 §9.
/// - Products requiring cold chain may only be held at branches with `cold_chain_capable = true`.
/// - Acceptable range: 2.0°C to 8.0°C.
/// - Readings outside range set `is_excursion = true` and quarantine batch.
/// - Clearing excursion requires `rx.approve` permission with documented decision note.
#[derive(Debug, Clone)]
pub struct ColdChainService {
    pool: PgPool,
}

impl ColdChainService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record temperature reading for a batch at a branch
    pub async fn record_temperature(
        &self,
        ctx: &TenantContext,
        req: ColdChainLogRequest,
    ) -> Result<bool, InventoryError> {
        ctx.require("inventory.view")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        // 1. Verify branch is cold-chain capable
        let branch =
            sqlx::query("SELECT cold_chain_capable FROM branches WHERE tenant_id = $1 AND id = $2")
                .bind(ctx.tenant_id().0)
                .bind(req.branch_id.0)
                .fetch_optional(&self.pool)
                .await?;

        match branch {
            Some(b) if b.get::<bool, _>("cold_chain_capable") => (),
            _ => return Err(InventoryError::ColdChainIncapable(req.branch_id)),
        }

        // 2. Check temperature range (2.0C - 8.0C)
        let is_excursion = req.temperature_c < 2.0 || req.temperature_c > 8.0;

        sqlx::query(
            "INSERT INTO cold_chain_logs (id, tenant_id, branch_id, batch_id, temperature_c, is_excursion, note, recorded_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id().0)
        .bind(req.branch_id.0)
        .bind(req.batch_id.0)
        .bind(req.temperature_c)
        .bind(is_excursion)
        .bind(&req.note)
        .bind(ctx.user_id().0)
        .execute(&self.pool)
        .await?;

        // 3. If excursion detected, quarantine batch
        if is_excursion {
            sqlx::query(
                "UPDATE batches
                 SET is_quarantined = true
                 WHERE tenant_id = $1 AND id = $2",
            )
            .bind(ctx.tenant_id().0)
            .bind(req.batch_id.0)
            .execute(&self.pool)
            .await?;
        }

        Ok(is_excursion)
    }

    /// Clear excursion: Pharmacist reviews and releases batch from quarantine.
    /// Invariant: Requires `rx.approve` permission.
    pub async fn clear_excursion(
        &self,
        ctx: &TenantContext,
        batch_id: BatchId,
        req: ClearExcursionRequest,
    ) -> Result<(), InventoryError> {
        ctx.require("rx.approve")
            .map_err(|e| InventoryError::Unauthorized(e.to_string()))?;

        sqlx::query(
            "UPDATE batches
             SET is_quarantined = false
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(batch_id.0)
        .execute(&self.pool)
        .await?;

        // Audit log of pharmacist excursion clearance
        sqlx::query(
            "INSERT INTO audit_log (tenant_id, actor_id, actor_type, entity_type, entity_id, action, after, reason)
             VALUES ($1, $2, 'USER', 'BATCH', $3, 'CLEAR_EXCURSION', $4, $5)"
        )
        .bind(ctx.tenant_id().0)
        .bind(ctx.user_id().0)
        .bind(batch_id.0)
        .bind(serde_json::json!({"decision": req.decision_note}))
        .bind("Pharmacist cleared temperature excursion")
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
