use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, RiderCashSessionId, RiderId};
use shifa_core::money::Money;
use sqlx::{PgPool, Row};

use crate::error::FulfilmentError;
use crate::models::{RiderDto, RiderStatus};

pub struct AssignmentEngine;

impl AssignmentEngine {
    /// Validates financial safety constraints before assigning a COD order to a rider (Doc 12 §5, §7):
    /// 1. Rider must not have an open cash session >24 hours old.
    /// 2. Undeposited cash in current open session + proposed order total must not exceed the branch COD ceiling.
    pub async fn validate_cod_assignment_eligibility(
        pool: &PgPool,
        ctx: &TenantContext,
        rider_id: RiderId,
        _branch_id: BranchId,
        order_amount: Money,
        cod_ceiling: Money,
    ) -> Result<(), FulfilmentError> {
        // 1. Check for stale open cash sessions (> 24 hours old)
        let cutoff = Utc::now() - Duration::hours(24);
        let stale_row = sqlx::query(
            "SELECT id FROM rider_cash_sessions
             WHERE tenant_id = $1 AND rider_id = $2 AND status != 'RECONCILED' AND opened_at < $3
             LIMIT 1"
        )
        .bind(ctx.tenant_id().0)
        .bind(rider_id.0)
        .bind(cutoff)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = stale_row {
            let session_id: uuid::Uuid = row.get("id");
            return Err(FulfilmentError::StaleCashSessionBlocked {
                rider_id,
                session_id: RiderCashSessionId::from(session_id),
            });
        }

        // 2. Compute current undeposited cash in active open sessions
        let undeposited_dec: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(expected_amount), 0.0000)
             FROM rider_cash_sessions
             WHERE tenant_id = $1 AND rider_id = $2 AND status != 'RECONCILED'"
        )
        .bind(ctx.tenant_id().0)
        .bind(rider_id.0)
        .fetch_one(pool)
        .await?;

        let current_undeposited = Money::from_decimal(undeposited_dec);
        let projected = Money::from_decimal(current_undeposited.0 + order_amount.0);

        if projected.0 > cod_ceiling.0 {
            return Err(FulfilmentError::CashCeilingExceeded {
                rider_id,
                limit: format!("{}", cod_ceiling.0),
                current_undeposited: format!("{}", current_undeposited.0),
            });
        }

        Ok(())
    }

    /// Ranks available riders for auto-assignment according to Doc 12 §5:
    /// currently on shift -> fewest active deliveries -> lowest recent decline count
    pub async fn rank_candidates(
        pool: &PgPool,
        ctx: &TenantContext,
        branch_id: BranchId,
    ) -> Result<Vec<RiderDto>, FulfilmentError> {
        let rows = sqlx::query(
            "SELECT r.id, r.tenant_id, r.branch_id, r.user_id, r.vehicle_type, r.cnic, r.licence_no,
                    r.status::text as status, r.on_shift, r.decline_count, r.shift_started_at, r.shift_ended_at,
                    r.created_at, r.updated_at,
                    COUNT(d.id) FILTER (WHERE d.status IN ('ASSIGNED', 'ACCEPTED', 'PICKED_UP', 'IN_TRANSIT', 'OUT_FOR_DELIVERY')) as active_deliveries
             FROM riders r
             LEFT JOIN deliveries d ON d.rider_id = r.id AND d.tenant_id = r.tenant_id
             WHERE r.tenant_id = $1 AND r.branch_id = $2 AND r.status != 'SUSPENDED'
             GROUP BY r.id
             ORDER BY r.on_shift DESC, active_deliveries ASC, r.decline_count ASC, r.created_at ASC"
        )
        .bind(ctx.tenant_id().0)
        .bind(branch_id.0)
        .fetch_all(pool)
        .await?;

        let mut candidates = Vec::new();
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            let tid: uuid::Uuid = row.get("tenant_id");
            let bid: uuid::Uuid = row.get("branch_id");
            let uid: uuid::Uuid = row.get("user_id");
            let vehicle_type: String = row.get("vehicle_type");
            let cnic: String = row.get("cnic");
            let licence_no: String = row.get("licence_no");
            let status_str: String = row.get("status");
            let on_shift: bool = row.get("on_shift");
            let decline_count: i32 = row.get("decline_count");
            let shift_started_at = row.get("shift_started_at");
            let shift_ended_at = row.get("shift_ended_at");
            let created_at = row.get("created_at");
            let updated_at = row.get("updated_at");

            let status = status_str.parse().unwrap_or(RiderStatus::Available);

            candidates.push(RiderDto {
                id: RiderId::from(id),
                tenant_id: TenantId::from(tid),
                branch_id: BranchId::from(bid),
                user_id: UserId::from(uid),
                vehicle_type,
                cnic,
                licence_no,
                status,
                on_shift,
                decline_count,
                shift_started_at,
                shift_ended_at,
                created_at,
                updated_at,
            });
        }

        Ok(candidates)
    }
}
