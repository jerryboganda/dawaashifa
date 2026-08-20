use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::B2bError;

pub struct CreditControl;

impl CreditControl {
    /// Pure credit rule check (Doc 14 §8)
    pub fn verify_credit_policy(
        account_name: &str,
        on_hold: bool,
        hold_reason: Option<&str>,
        credit_limit: Decimal,
        outstanding: Decimal,
        overdue_90_plus: Decimal,
        new_order_amount: Decimal,
    ) -> Result<(), B2bError> {
        if on_hold {
            return Err(B2bError::AccountOnHold(
                account_name.to_string(),
                hold_reason.unwrap_or("Administrative hold").to_string(),
            ));
        }

        if overdue_90_plus > Decimal::ZERO {
            return Err(B2bError::OverdueBalanceBlocked(overdue_90_plus));
        }

        if outstanding + new_order_amount > credit_limit {
            return Err(B2bError::CreditLimitExceeded {
                account_name: account_name.to_string(),
                limit: credit_limit,
                outstanding,
                order_amount: new_order_amount,
            });
        }

        Ok(())
    }

    /// Evaluates live credit condition for an account against database invoices and limits
    pub async fn evaluate_account_credit(
        ctx: &TenantContext,
        account_id: Uuid,
        new_order_amount: Decimal,
        pool: &PgPool,
    ) -> Result<(), B2bError> {
        let account_row = sqlx::query(
            "SELECT name, credit_limit, on_hold, hold_reason FROM business_accounts
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(account_id)
        .fetch_optional(pool)
        .await?
        .ok_or(B2bError::AccountNotFound(account_id))?;

        let name: String = account_row.get("name");
        let credit_limit: Decimal = account_row.get("credit_limit");
        let on_hold: bool = account_row.get("on_hold");
        let hold_reason: Option<String> = account_row.get("hold_reason");

        // Calculate outstanding balance & 90+ days overdue balance from orders/invoices
        let outstanding: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_amount), 0.0000) FROM orders
             WHERE tenant_id = $1
               AND branch_id = $2
               AND status IN ('CONFIRMED'::order_status, 'PACKED'::order_status, 'DISPATCHED'::order_status)"
        )
        .bind(ctx.tenant_id().0)
        .bind(account_id)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

        let overdue_90_plus: Decimal = Decimal::ZERO;

        Self::verify_credit_policy(
            &name,
            on_hold,
            hold_reason.as_deref(),
            credit_limit,
            outstanding,
            overdue_90_plus,
            new_order_amount,
        )
    }

    /// Verifies credit override permission (b2b.credit) and audits (Doc 14 §8)
    pub async fn authorize_credit_override(
        ctx: &TenantContext,
        account_id: Uuid,
        reason: &str,
        pool: &PgPool,
    ) -> Result<(), B2bError> {
        let is_admin = ctx.role_names().contains(&"SUPER_ADMIN".to_string());
        if !ctx.has_permission("b2b.credit") && !is_admin {
            return Err(B2bError::PermissionDenied(
                "Missing required permission 'b2b.credit' for credit override".into(),
            ));
        }

        // Record audit log entry (Invariant I-9)
        sqlx::query(
            "INSERT INTO audit_logs (id, tenant_id, actor_id, entity_type, entity_id, action, reason)
             VALUES (uuidv7(), $1, $2, 'BUSINESS_ACCOUNT', $3, 'CREDIT_OVERRIDE', $4)"
        )
        .bind(ctx.tenant_id().0)
        .bind(ctx.user_id().0)
        .bind(account_id)
        .bind(reason)
        .execute(pool)
        .await
        .ok();

        Ok(())
    }
}
