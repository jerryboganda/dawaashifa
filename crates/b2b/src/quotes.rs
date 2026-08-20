use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use shifa_core::id::{CustomerId, OrderId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::credit::CreditControl;
use crate::error::B2bError;
use crate::models::{
    CreateQuotationRequest, QuotationDto, QuotationItemDto, QuotationItemRequest,
    ReviseQuotationRequest,
};

pub struct QuoteEngine;

impl QuoteEngine {
    /// Generates next sequential quote number (e.g. Q-LHR-26-00001)
    pub fn generate_quote_no(branch_code: &str, seq: u32) -> String {
        let yy = Utc::now().year() % 100;
        format!("Q-{}-{:02}-{:05}", branch_code, yy, seq)
    }

    /// Validates quotation items against MRP cap (Doc 14 §5)
    pub async fn validate_mrp_cap(
        ctx: &TenantContext,
        items: &[QuotationItemRequest],
        pool: &PgPool,
    ) -> Result<(), B2bError> {
        for item in items {
            let unit_price = Decimal::from_str_exact(&item.unit_price)
                .map_err(|_| B2bError::Validation("Invalid unit price".into()))?;

            let mrp_row = sqlx::query("SELECT mrp FROM products WHERE tenant_id = $1 AND id = $2")
                .bind(ctx.tenant_id().0)
                .bind(item.product_id)
                .fetch_optional(pool)
                .await?
                .ok_or(B2bError::ProductNotFound(item.product_id))?;

            let mrp: Decimal = mrp_row.get("mrp");
            if unit_price > mrp {
                return Err(B2bError::NegotiatedPriceAboveMrp {
                    product_id: item.product_id,
                    price: unit_price,
                    mrp,
                });
            }
        }
        Ok(())
    }

    /// Creates a new draft quotation (Doc 14 §6)
    pub async fn create_quotation(
        ctx: &TenantContext,
        req: CreateQuotationRequest,
        pool: &PgPool,
    ) -> Result<QuotationDto, B2bError> {
        Self::validate_mrp_cap(ctx, &req.items, pool).await?;

        let quote_id = Uuid::now_v7();
        let quote_no = Self::generate_quote_no("LHR", 1);

        let mut subtotal = Decimal::ZERO;
        let mut total_discount = Decimal::ZERO;

        let mut item_dtos = Vec::new();

        let mut tx = pool.begin().await?;

        for item in &req.items {
            let item_id = Uuid::now_v7();
            let unit_price = Decimal::from_str_exact(&item.unit_price).unwrap_or(Decimal::ZERO);
            let discount = item
                .discount
                .as_deref()
                .and_then(|d| Decimal::from_str_exact(d).ok())
                .unwrap_or(Decimal::ZERO);
            let line_total = (unit_price * Decimal::from(item.qty)) - discount;

            subtotal += unit_price * Decimal::from(item.qty);
            total_discount += discount;

            sqlx::query(
                "INSERT INTO quotation_items (id, tenant_id, quotation_id, product_id, qty, unit_price, discount, line_total, lead_time_days, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            )
            .bind(item_id)
            .bind(ctx.tenant_id().0)
            .bind(quote_id)
            .bind(item.product_id)
            .bind(item.qty)
            .bind(unit_price)
            .bind(discount)
            .bind(line_total)
            .bind(item.lead_time_days.unwrap_or(0))
            .bind(item.notes.as_deref())
            .execute(&mut *tx)
            .await?;

            item_dtos.push(QuotationItemDto {
                id: item_id,
                quotation_id: quote_id,
                product_id: item.product_id,
                qty: item.qty,
                unit_price: unit_price.to_string(),
                discount: discount.to_string(),
                line_total: line_total.to_string(),
                lead_time_days: item.lead_time_days.unwrap_or(0),
                notes: item.notes.clone(),
            });
        }

        let total = subtotal - total_discount;

        sqlx::query(
            "INSERT INTO quotations (id, tenant_id, account_id, quote_no, version, status, valid_until, subtotal, discount, tax_amount, total, terms_text, prepared_by)
             VALUES ($1, $2, $3, $4, 1, 'DRAFT', $5, $6, $7, 0.0000, $8, $9, $10)"
        )
        .bind(quote_id)
        .bind(ctx.tenant_id().0)
        .bind(req.account_id)
        .bind(&quote_no)
        .bind(req.valid_until)
        .bind(subtotal)
        .bind(total_discount)
        .bind(total)
        .bind(req.terms_text.as_deref())
        .bind(ctx.user_id().0)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(QuotationDto {
            id: quote_id,
            tenant_id: ctx.tenant_id().0,
            account_id: req.account_id,
            quote_no,
            version: 1,
            parent_quote_id: None,
            status: "DRAFT".to_string(),
            valid_until: req.valid_until,
            subtotal: subtotal.to_string(),
            discount: total_discount.to_string(),
            tax_amount: "0.0000".to_string(),
            total: total.to_string(),
            terms_text: req.terms_text,
            prepared_by: ctx.user_id().0,
            approved_by: None,
            sent_at: None,
            responded_at: None,
            items: item_dtos,
            created_at: Utc::now(),
        })
    }

    /// Revises an existing quotation: increments version and marks parent as REVISED (Doc 14 §6)
    pub async fn revise_quotation(
        ctx: &TenantContext,
        parent_id: Uuid,
        req: ReviseQuotationRequest,
        pool: &PgPool,
    ) -> Result<QuotationDto, B2bError> {
        let parent_row = sqlx::query(
            "SELECT account_id, quote_no, version FROM quotations WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(parent_id)
        .fetch_optional(pool)
        .await?
        .ok_or(B2bError::QuoteNotFound(parent_id))?;

        let account_id: Uuid = parent_row.get("account_id");
        let quote_no: String = parent_row.get("quote_no");
        let old_version: i32 = parent_row.get("version");
        let new_version = old_version + 1;

        Self::validate_mrp_cap(ctx, &req.items, pool).await?;

        let new_id = Uuid::now_v7();
        let mut subtotal = Decimal::ZERO;
        let mut total_discount = Decimal::ZERO;
        let mut item_dtos = Vec::new();

        let mut tx = pool.begin().await?;

        // 1. Mark parent as REVISED
        sqlx::query("UPDATE quotations SET status = 'REVISED' WHERE tenant_id = $1 AND id = $2")
            .bind(ctx.tenant_id().0)
            .bind(parent_id)
            .execute(&mut *tx)
            .await?;

        // 2. Insert new revision items
        for item in &req.items {
            let item_id = Uuid::now_v7();
            let unit_price = Decimal::from_str_exact(&item.unit_price).unwrap_or(Decimal::ZERO);
            let discount = item
                .discount
                .as_deref()
                .and_then(|d| Decimal::from_str_exact(d).ok())
                .unwrap_or(Decimal::ZERO);
            let line_total = (unit_price * Decimal::from(item.qty)) - discount;

            subtotal += unit_price * Decimal::from(item.qty);
            total_discount += discount;

            sqlx::query(
                "INSERT INTO quotation_items (id, tenant_id, quotation_id, product_id, qty, unit_price, discount, line_total, lead_time_days, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            )
            .bind(item_id)
            .bind(ctx.tenant_id().0)
            .bind(new_id)
            .bind(item.product_id)
            .bind(item.qty)
            .bind(unit_price)
            .bind(discount)
            .bind(line_total)
            .bind(item.lead_time_days.unwrap_or(0))
            .bind(item.notes.as_deref())
            .execute(&mut *tx)
            .await?;

            item_dtos.push(QuotationItemDto {
                id: item_id,
                quotation_id: new_id,
                product_id: item.product_id,
                qty: item.qty,
                unit_price: unit_price.to_string(),
                discount: discount.to_string(),
                line_total: line_total.to_string(),
                lead_time_days: item.lead_time_days.unwrap_or(0),
                notes: item.notes.clone(),
            });
        }

        let total = subtotal - total_discount;

        sqlx::query(
            "INSERT INTO quotations (id, tenant_id, account_id, quote_no, version, parent_quote_id, status, valid_until, subtotal, discount, tax_amount, total, terms_text, prepared_by)
             VALUES ($1, $2, $3, $4, $5, $6, 'DRAFT', $7, $8, $9, 0.0000, $10, $11, $12)"
        )
        .bind(new_id)
        .bind(ctx.tenant_id().0)
        .bind(account_id)
        .bind(&quote_no)
        .bind(new_version)
        .bind(parent_id)
        .bind(req.valid_until)
        .bind(subtotal)
        .bind(total_discount)
        .bind(total)
        .bind(req.terms_text.as_deref())
        .bind(ctx.user_id().0)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(QuotationDto {
            id: new_id,
            tenant_id: ctx.tenant_id().0,
            account_id,
            quote_no,
            version: new_version,
            parent_quote_id: Some(parent_id),
            status: "DRAFT".to_string(),
            valid_until: req.valid_until,
            subtotal: subtotal.to_string(),
            discount: total_discount.to_string(),
            tax_amount: "0.0000".to_string(),
            total: total.to_string(),
            terms_text: req.terms_text,
            prepared_by: ctx.user_id().0,
            approved_by: None,
            sent_at: None,
            responded_at: None,
            items: item_dtos,
            created_at: Utc::now(),
        })
    }

    /// Approves high discount on quotation (Doc 14 §6)
    pub async fn approve_discount(
        ctx: &TenantContext,
        quote_id: Uuid,
        approver_limit: Decimal,
        pool: &PgPool,
    ) -> Result<(), B2bError> {
        let quote_row =
            sqlx::query("SELECT discount FROM quotations WHERE tenant_id = $1 AND id = $2")
                .bind(ctx.tenant_id().0)
                .bind(quote_id)
                .fetch_optional(pool)
                .await?
                .ok_or(B2bError::QuoteNotFound(quote_id))?;

        let discount: Decimal = quote_row.get("discount");
        if approver_limit < discount {
            return Err(B2bError::ApproverBelowLimit {
                limit: approver_limit,
                required: discount,
            });
        }

        sqlx::query("UPDATE quotations SET approved_by = $1 WHERE tenant_id = $2 AND id = $3")
            .bind(ctx.user_id().0)
            .bind(ctx.tenant_id().0)
            .bind(quote_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Accepts quote and converts directly to B2B Order in CONFIRMED state, bypassing retail carts (Doc 14 §6)
    pub async fn accept_and_convert_quote(
        ctx: &TenantContext,
        quote_id: Uuid,
        pool: &PgPool,
    ) -> Result<OrderId, B2bError> {
        let quote_row = sqlx::query(
            "SELECT account_id, status, valid_until, total FROM quotations WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(quote_id)
        .fetch_optional(pool)
        .await?
        .ok_or(B2bError::QuoteNotFound(quote_id))?;

        let account_id: Uuid = quote_row.get("account_id");
        let valid_until: DateTime<Utc> = quote_row.get("valid_until");
        let total: Decimal = quote_row.get("total");

        // 1. Expiry check (Doc 14 §6)
        if valid_until < Utc::now() {
            return Err(B2bError::QuoteExpired(quote_id, valid_until.to_rfc3339()));
        }

        // 2. Credit check (Doc 14 §8)
        CreditControl::evaluate_account_credit(ctx, account_id, total, pool).await?;

        // 3. Mark quotation as ACCEPTED
        sqlx::query("UPDATE quotations SET status = 'ACCEPTED', responded_at = now() WHERE tenant_id = $1 AND id = $2")
            .bind(ctx.tenant_id().0)
            .bind(quote_id)
            .execute(pool)
            .await?;

        // 4. Create B2B order landing in CONFIRMED status
        let order_id = OrderId::new();
        let customer_id = CustomerId::new();
        let branch_id: Uuid =
            sqlx::query_scalar("SELECT id FROM branches WHERE tenant_id = $1 LIMIT 1")
                .bind(ctx.tenant_id().0)
                .fetch_one(pool)
                .await
                .unwrap_or_else(|_| Uuid::now_v7());

        sqlx::query(
            "INSERT INTO orders (id, tenant_id, branch_id, customer_id, status, subtotal, discount, delivery_fee, tax, total_amount, payment_method, total_price)
             VALUES ($1, $2, $3, $4, 'CONFIRMED'::order_status, $5, 0, 0, 0, $5, 'CREDIT_TERMS', $5)"
        )
        .bind(order_id.0)
        .bind(ctx.tenant_id().0)
        .bind(branch_id)
        .bind(customer_id.0)
        .bind(total)
        .execute(pool)
        .await?;

        Ok(order_id)
    }
}
