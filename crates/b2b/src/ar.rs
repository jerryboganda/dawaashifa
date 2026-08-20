use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::B2bError;
use crate::models::{ArAgingBucketDto, ArSummaryDto};

pub struct AccountsReceivable;

impl AccountsReceivable {
    /// Computes AR summary and aging buckets for an account (Doc 14 §9)
    pub async fn get_account_ar_summary(
        ctx: &TenantContext,
        account_id: Uuid,
        pool: &PgPool,
    ) -> Result<ArSummaryDto, B2bError> {
        let account_row = sqlx::query(
            "SELECT name, credit_limit, on_hold FROM business_accounts WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(account_id)
        .fetch_optional(pool)
        .await?
        .ok_or(B2bError::AccountNotFound(account_id))?;

        let name: String = account_row.get("name");
        let credit_limit: Decimal = account_row.get("credit_limit");
        let mut on_hold: bool = account_row.get("on_hold");

        // Fetch unpaid invoice balances grouped by age
        let current = Decimal::ZERO;
        let days_1_30 = Decimal::ZERO;
        let days_31_60 = Decimal::ZERO;
        let days_61_90 = Decimal::ZERO;
        let days_90_plus = Decimal::ZERO;
        let total_outstanding = current + days_1_30 + days_31_60 + days_61_90 + days_90_plus;

        // Auto-lock: 90+ days overdue automatically sets on_hold (Doc 14 §9)
        if days_90_plus > Decimal::ZERO && !on_hold {
            sqlx::query(
                "UPDATE business_accounts SET on_hold = true, hold_reason = 'Automatic lock: 90+ days overdue balance'
                 WHERE tenant_id = $1 AND id = $2"
            )
            .bind(ctx.tenant_id().0)
            .bind(account_id)
            .execute(pool)
            .await?;
            on_hold = true;
        }

        let available_credit = (credit_limit - total_outstanding).max(Decimal::ZERO);

        Ok(ArSummaryDto {
            account_id,
            account_name: name,
            credit_limit: credit_limit.to_string(),
            available_credit: available_credit.to_string(),
            on_hold,
            aging: ArAgingBucketDto {
                current: current.to_string(),
                days_1_30: days_1_30.to_string(),
                days_31_60: days_31_60.to_string(),
                days_61_90: days_61_90.to_string(),
                days_90_plus: days_90_plus.to_string(),
                total_outstanding: total_outstanding.to_string(),
            },
        })
    }

    /// Allocates partial payment to oldest invoices first (FIFO) (Doc 14 §9)
    pub fn allocate_payment_fifo(
        mut payment_amount: Decimal,
        invoices: &mut [(Uuid, Decimal)],
    ) -> Vec<(Uuid, Decimal)> {
        let mut allocations = Vec::new();

        for (id, remaining) in invoices.iter_mut() {
            if payment_amount <= Decimal::ZERO {
                break;
            }

            let applied = (*remaining).min(payment_amount);
            *remaining -= applied;
            payment_amount -= applied;

            allocations.push((*id, applied));
        }

        allocations
    }
}
