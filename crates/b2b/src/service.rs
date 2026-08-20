use chrono::Utc;
use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use shifa_core::id::OrderId;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::ar::AccountsReceivable;
use crate::consignment::ConsignmentManager;
use crate::device::DeviceTraceability;
use crate::error::B2bError;
use crate::models::*;
use crate::po::PurchaseOrderEngine;
use crate::quotes::QuoteEngine;

#[derive(Clone)]
pub struct B2bService {
    pool: PgPool,
}

impl B2bService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // --------------------------------------------------------------------------------------------
    // Business Accounts & Contacts
    // --------------------------------------------------------------------------------------------
    pub async fn create_account(
        &self,
        ctx: &TenantContext,
        req: CreateAccountRequest,
    ) -> Result<BusinessAccountDto, B2bError> {
        let id = Uuid::now_v7();
        let credit_limit = req
            .credit_limit
            .as_deref()
            .and_then(|c| Decimal::from_str_exact(c).ok())
            .unwrap_or(Decimal::ZERO);
        let terms = req.payment_terms_days.unwrap_or(30);
        let shipping = req
            .shipping_addresses
            .unwrap_or_else(|| serde_json::json!([]));
        let acc_type = req.account_type.unwrap_or_else(|| "HOSPITAL".to_string());

        sqlx::query(
            "INSERT INTO business_accounts (id, tenant_id, name, account_type, ntn, strn, billing_address, shipping_addresses, credit_limit, payment_terms_days, price_list_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind(id)
        .bind(ctx.tenant_id().0)
        .bind(&req.name)
        .bind(&acc_type)
        .bind(req.ntn.as_deref())
        .bind(req.strn.as_deref())
        .bind(&req.billing_address)
        .bind(&shipping)
        .bind(credit_limit)
        .bind(terms)
        .bind(req.price_list_id)
        .execute(&self.pool)
        .await?;

        Ok(BusinessAccountDto {
            id,
            tenant_id: ctx.tenant_id().0,
            name: req.name,
            account_type: acc_type,
            ntn: req.ntn,
            strn: req.strn,
            billing_address: req.billing_address,
            shipping_addresses: shipping,
            credit_limit: credit_limit.to_string(),
            payment_terms_days: terms,
            price_list_id: req.price_list_id,
            status: "ACTIVE".to_string(),
            on_hold: false,
            hold_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub async fn list_accounts(
        &self,
        ctx: &TenantContext,
    ) -> Result<Vec<BusinessAccountDto>, B2bError> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, name, account_type, ntn, strn, billing_address, shipping_addresses, credit_limit, payment_terms_days, price_list_id, status, on_hold, hold_reason, created_at, updated_at
             FROM business_accounts WHERE tenant_id = $1"
        )
        .bind(ctx.tenant_id().0)
        .fetch_all(&self.pool)
        .await?;

        let mut list = Vec::new();
        for r in rows {
            let credit_limit: Decimal = r.get("credit_limit");
            list.push(BusinessAccountDto {
                id: r.get("id"),
                tenant_id: ctx.tenant_id().0,
                name: r.get("name"),
                account_type: r.get("account_type"),
                ntn: r.get("ntn"),
                strn: r.get("strn"),
                billing_address: r.get("billing_address"),
                shipping_addresses: r.get("shipping_addresses"),
                credit_limit: credit_limit.to_string(),
                payment_terms_days: r.get("payment_terms_days"),
                price_list_id: r.get("price_list_id"),
                status: r.get("status"),
                on_hold: r.get("on_hold"),
                hold_reason: r.get("hold_reason"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            });
        }
        Ok(list)
    }

    pub async fn set_account_hold(
        &self,
        ctx: &TenantContext,
        account_id: Uuid,
        req: AccountHoldRequest,
    ) -> Result<(), B2bError> {
        sqlx::query(
            "UPDATE business_accounts SET on_hold = $1, hold_reason = $2, updated_at = now()
             WHERE tenant_id = $3 AND id = $4",
        )
        .bind(req.on_hold)
        .bind(&req.reason)
        .bind(ctx.tenant_id().0)
        .bind(account_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // --------------------------------------------------------------------------------------------
    // Quotations
    // --------------------------------------------------------------------------------------------
    pub async fn create_quotation(
        &self,
        ctx: &TenantContext,
        req: CreateQuotationRequest,
    ) -> Result<QuotationDto, B2bError> {
        QuoteEngine::create_quotation(ctx, req, &self.pool).await
    }

    pub async fn revise_quotation(
        &self,
        ctx: &TenantContext,
        parent_id: Uuid,
        req: ReviseQuotationRequest,
    ) -> Result<QuotationDto, B2bError> {
        QuoteEngine::revise_quotation(ctx, parent_id, req, &self.pool).await
    }

    pub async fn approve_quotation_discount(
        &self,
        ctx: &TenantContext,
        quote_id: Uuid,
        approver_limit: Decimal,
    ) -> Result<(), B2bError> {
        QuoteEngine::approve_discount(ctx, quote_id, approver_limit, &self.pool).await
    }

    pub async fn accept_quotation(
        &self,
        ctx: &TenantContext,
        quote_id: Uuid,
    ) -> Result<OrderId, B2bError> {
        QuoteEngine::accept_and_convert_quote(ctx, quote_id, &self.pool).await
    }

    // --------------------------------------------------------------------------------------------
    // Purchase Orders
    // --------------------------------------------------------------------------------------------
    pub async fn ingest_purchase_order(
        &self,
        ctx: &TenantContext,
        req: CreatePurchaseOrderRequest,
    ) -> Result<PurchaseOrderDto, B2bError> {
        PurchaseOrderEngine::ingest_po(ctx, req, &self.pool).await
    }

    // --------------------------------------------------------------------------------------------
    // Accounts Receivable
    // --------------------------------------------------------------------------------------------
    pub async fn get_ar_summary(
        &self,
        ctx: &TenantContext,
        account_id: Uuid,
    ) -> Result<ArSummaryDto, B2bError> {
        AccountsReceivable::get_account_ar_summary(ctx, account_id, &self.pool).await
    }

    // --------------------------------------------------------------------------------------------
    // Consignment Stock
    // --------------------------------------------------------------------------------------------
    pub async fn place_consignment(
        &self,
        ctx: &TenantContext,
        req: PlaceConsignmentRequest,
    ) -> Result<ConsignmentStockDto, B2bError> {
        ConsignmentManager::place_stock(ctx, req, &self.pool).await
    }

    pub async fn reconcile_consignment(
        &self,
        ctx: &TenantContext,
        stock_id: Uuid,
        req: ReconcileConsignmentRequest,
    ) -> Result<ConsignmentStockDto, B2bError> {
        ConsignmentManager::reconcile_stock(
            ctx,
            stock_id,
            req.physical_count,
            req.notes.as_deref(),
            &self.pool,
        )
        .await
    }

    // --------------------------------------------------------------------------------------------
    // Device Traceability
    // --------------------------------------------------------------------------------------------
    pub async fn register_device(
        &self,
        ctx: &TenantContext,
        req: RegisterDeviceRequest,
    ) -> Result<DeviceUnitDto, B2bError> {
        DeviceTraceability::register_device(ctx, req, &self.pool).await
    }

    pub async fn query_recall(
        &self,
        ctx: &TenantContext,
        product_id: Option<Uuid>,
        batch_id: Option<Uuid>,
    ) -> Result<RecallQueryResponse, B2bError> {
        DeviceTraceability::query_recall(ctx, product_id, batch_id, &self.pool).await
    }
}
