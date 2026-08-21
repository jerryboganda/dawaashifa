use chrono::Utc;
use shifa_core::context::TenantContext;
use shifa_core::id::{TenantId, UserId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::AdminError;
use crate::models::*;

#[derive(Clone)]
pub struct AdminService {
    pool: PgPool,
}

impl AdminService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// List audit log events with multi-criteria filtering (Doc 16 §12, Invariant I-9)
    pub async fn list_audit_events(
        &self,
        ctx: &TenantContext,
        query: AuditQueryRequest,
    ) -> Result<Vec<AuditEventDto>, AdminError> {
        let limit = query.limit.unwrap_or(100).min(500);
        let offset = query.offset.unwrap_or(0);

        let rows = sqlx::query(
            "SELECT id, tenant_id, actor_id, actor_type, entity_type, entity_id, action, before, after, reason, ip, occurred_at
             FROM audit_log
             WHERE tenant_id = $1
               AND ($2::TEXT IS NULL OR entity_type = $2)
               AND ($3::UUID IS NULL OR entity_id = $3)
               AND ($4::UUID IS NULL OR actor_id = $4)
               AND ($5::TEXT IS NULL OR action = $5)
               AND ($6::TIMESTAMPTZ IS NULL OR occurred_at >= $6)
               AND ($7::TIMESTAMPTZ IS NULL OR occurred_at <= $7)
             ORDER BY occurred_at DESC
             LIMIT $8 OFFSET $9",
        )
        .bind(ctx.tenant_id().0)
        .bind(query.entity_type)
        .bind(query.entity_id)
        .bind(query.actor_id)
        .bind(query.action)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let events = rows
            .into_iter()
            .map(|r| {
                let aid: Option<Uuid> = r.get("actor_id");
                AuditEventDto {
                    id: r.get("id"),
                    tenant_id: TenantId::from(r.get::<Uuid, _>("tenant_id")),
                    actor_id: aid.map(UserId::from),
                    actor_type: r.get("actor_type"),
                    entity_type: r.get("entity_type"),
                    entity_id: r.get("entity_id"),
                    action: r.get("action"),
                    before: r.get("before"),
                    after: r.get("after"),
                    reason: r.get("reason"),
                    ip: r.get("ip"),
                    occurred_at: r.get("occurred_at"),
                }
            })
            .collect();

        Ok(events)
    }

    /// Export audit log to DRAP-compliant CSV format (Doc 16 §12)
    pub async fn export_audit_csv(
        &self,
        ctx: &TenantContext,
        query: AuditQueryRequest,
    ) -> Result<String, AdminError> {
        let events = self.list_audit_events(ctx, query).await?;
        let mut csv = String::from("id,timestamp_utc,actor_type,actor_id,entity_type,entity_id,action,reason,ip,before,after\n");

        for e in events {
            let actor_str = e
                .actor_id
                .map(|a| a.0.to_string())
                .unwrap_or_else(|| "N/A".into());
            let reason_str = e
                .reason
                .unwrap_or_default()
                .replace(',', ";")
                .replace('"', "'");
            let ip_str = e.ip.unwrap_or_default();
            let before_str = e
                .before
                .map(|b| b.to_string())
                .unwrap_or_default()
                .replace(',', ";")
                .replace('"', "'");
            let after_str = e
                .after
                .map(|a| a.to_string())
                .unwrap_or_default()
                .replace(',', ";")
                .replace('"', "'");

            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                e.id,
                e.occurred_at.to_rfc3339(),
                e.actor_type,
                actor_str,
                e.entity_type,
                e.entity_id,
                e.action,
                reason_str,
                ip_str,
                before_str,
                after_str
            ));
        }

        Ok(csv)
    }

    /// Get current tenant system settings
    pub async fn get_system_settings(
        &self,
        ctx: &TenantContext,
    ) -> Result<SystemSettingsDto, AdminError> {
        let row = sqlx::query(
            "SELECT id, name, legal_name, ntn, strn, status::text, settings, updated_at
             FROM tenants
             WHERE id = $1",
        )
        .bind(ctx.tenant_id().0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AdminError::TenantNotFound(ctx.tenant_id().0.to_string()))?;

        Ok(SystemSettingsDto {
            tenant_id: ctx.tenant_id(),
            name: row.get("name"),
            legal_name: row.get("legal_name"),
            ntn: row.get("ntn"),
            strn: row.get("strn"),
            status: row.get("status"),
            settings: row.get("settings"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Update tenant system settings
    pub async fn update_system_settings(
        &self,
        ctx: &TenantContext,
        req: UpdateSystemSettingsRequest,
    ) -> Result<SystemSettingsDto, AdminError> {
        let current = self.get_system_settings(ctx).await?;
        let legal_name = req.legal_name.unwrap_or(current.legal_name);
        let ntn = req.ntn.or(current.ntn);
        let strn = req.strn.or(current.strn);
        let settings = req.settings.unwrap_or(current.settings);

        let row = sqlx::query(
            "UPDATE tenants
             SET legal_name = $1, ntn = $2, strn = $3, settings = $4, updated_at = now()
             WHERE id = $5
             RETURNING id, name, legal_name, ntn, strn, status::text, settings, updated_at",
        )
        .bind(legal_name)
        .bind(ntn)
        .bind(strn)
        .bind(settings)
        .bind(ctx.tenant_id().0)
        .fetch_one(&self.pool)
        .await?;

        Ok(SystemSettingsDto {
            tenant_id: ctx.tenant_id(),
            name: row.get("name"),
            legal_name: row.get("legal_name"),
            ntn: row.get("ntn"),
            strn: row.get("strn"),
            status: row.get("status"),
            settings: row.get("settings"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Aggregate operational and regulatory metrics
    pub async fn get_operational_report(
        &self,
        ctx: &TenantContext,
    ) -> Result<OperationalReportDto, AdminError> {
        let tenant_id = ctx.tenant_id().0;

        let orders_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM orders WHERE tenant_id = $1 AND created_at >= CURRENT_DATE",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let rx_pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM prescriptions WHERE tenant_id = $1 AND status = 'PENDING_REVIEW'",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let payments_pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM payment_proofs WHERE tenant_id = $1 AND review_status = 'PENDING'",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let active_riders: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM riders WHERE tenant_id = $1 AND on_shift = true",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let revenue: Option<rust_decimal::Decimal> = sqlx::query_scalar(
            "SELECT sum(total) FROM orders WHERE tenant_id = $1 AND status NOT IN ('CANCELLED', 'RETURNED') AND created_at >= CURRENT_DATE",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(None);

        let fbr_pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM invoices WHERE tenant_id = $1 AND fbr_status = 'PENDING'",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(OperationalReportDto {
            today_orders_count: orders_count,
            rx_queue_depth: rx_pending,
            pending_payments_count: payments_pending,
            active_riders_count: active_riders,
            total_revenue_pkr: revenue.unwrap_or_default().to_string(),
            fbr_pending_invoices: fbr_pending,
            generated_at: Utc::now(),
        })
    }
}
