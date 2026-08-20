use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, InvoiceId, OrderId, TaxCategoryId, TenantId};
use shifa_core::money::Money;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::calculator::{TaxCalculator, TaxableItemInput};
use crate::error::TaxError;
use crate::fbr::FiscalReporter;
use crate::models::*;

#[derive(Clone)]
pub struct TaxService {
    pool: PgPool,
}

impl TaxService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // --------------------------------------------------------------------------------------------
    // Tax Categories Management
    // --------------------------------------------------------------------------------------------

    pub async fn list_tax_categories(
        &self,
        ctx: &TenantContext,
    ) -> Result<Vec<TaxCategoryDto>, TaxError> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, name, rate, fbr_code, is_exempt, is_zero_rated,
                    effective_from, effective_to, created_at
             FROM tax_categories
             WHERE tenant_id = $1
             ORDER BY name ASC, effective_from DESC",
        )
        .bind(ctx.tenant_id().0)
        .fetch_all(&self.pool)
        .await?;

        let mut list = Vec::new();
        for row in rows {
            list.push(self.map_tax_category_row(row)?);
        }
        Ok(list)
    }

    pub async fn create_tax_category(
        &self,
        ctx: &TenantContext,
        req: CreateTaxCategoryRequest,
    ) -> Result<TaxCategoryDto, TaxError> {
        ctx.require("tenant.settings")
            .map_err(|e| TaxError::Unauthorized(e.to_string()))?;

        let id = TaxCategoryId::new();
        let effective_from = req.effective_from.unwrap_or_else(Utc::now);

        sqlx::query(
            "INSERT INTO tax_categories (id, tenant_id, name, rate, fbr_code, is_exempt, is_zero_rated, effective_from)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(id.0)
        .bind(ctx.tenant_id().0)
        .bind(&req.name)
        .bind(req.rate)
        .bind(&req.fbr_code)
        .bind(req.is_exempt.unwrap_or(false))
        .bind(req.is_zero_rated.unwrap_or(false))
        .bind(effective_from)
        .execute(&self.pool)
        .await?;

        self.get_tax_category(ctx, id).await
    }

    pub async fn patch_tax_category(
        &self,
        ctx: &TenantContext,
        id: TaxCategoryId,
        req: PatchTaxCategoryRequest,
    ) -> Result<TaxCategoryDto, TaxError> {
        ctx.require("tenant.settings")
            .map_err(|e| TaxError::Unauthorized(e.to_string()))?;

        let current = self.get_tax_category(ctx, id).await?;
        let effective_from = req.effective_from.unwrap_or_else(Utc::now);

        // Close current rate period
        sqlx::query("UPDATE tax_categories SET effective_to = $1 WHERE tenant_id = $2 AND id = $3")
            .bind(effective_from)
            .bind(ctx.tenant_id().0)
            .bind(id.0)
            .execute(&self.pool)
            .await?;

        // Insert new versioned rate period (Doc 13 §5, §12)
        let new_id = TaxCategoryId::new();
        let fbr_code = req.fbr_code.or(current.fbr_code);

        sqlx::query(
            "INSERT INTO tax_categories (id, tenant_id, name, rate, fbr_code, is_exempt, is_zero_rated, effective_from)
             VALUES ($1, $2, $3, $4, $5, false, false, $6)"
        )
        .bind(new_id.0)
        .bind(ctx.tenant_id().0)
        .bind(&current.name)
        .bind(req.new_rate)
        .bind(&fbr_code)
        .bind(effective_from)
        .execute(&self.pool)
        .await?;

        self.get_tax_category(ctx, new_id).await
    }

    pub async fn get_tax_category(
        &self,
        ctx: &TenantContext,
        id: TaxCategoryId,
    ) -> Result<TaxCategoryDto, TaxError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, name, rate, fbr_code, is_exempt, is_zero_rated,
                    effective_from, effective_to, created_at
             FROM tax_categories
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(TaxError::CategoryNotFound(id))?;

        self.map_tax_category_row(row)
    }

    // --------------------------------------------------------------------------------------------
    // Gapless Invoice Numbering & Invoice Creation
    // --------------------------------------------------------------------------------------------

    pub fn compute_fiscal_year(date: DateTime<Utc>) -> String {
        let year = date.year();
        let month = date.month();
        if month >= 7 {
            format!("FY{:02}", (year + 1) % 100)
        } else {
            format!("FY{:02}", year % 100)
        }
    }

    pub async fn get_next_gapless_invoice_number(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: TenantId,
        branch_id: BranchId,
        branch_code: &str,
        date: DateTime<Utc>,
    ) -> Result<String, TaxError> {
        let fy = Self::compute_fiscal_year(date);

        let seq: i64 = sqlx::query_scalar(
            "INSERT INTO invoice_sequences (tenant_id, branch_id, fiscal_year, last_seq)
             VALUES ($1, $2, $3, 1)
             ON CONFLICT (tenant_id, branch_id, fiscal_year)
             DO UPDATE SET last_seq = invoice_sequences.last_seq + 1
             RETURNING last_seq",
        )
        .bind(tenant_id.0)
        .bind(branch_id.0)
        .bind(&fy)
        .fetch_one(&mut **tx)
        .await?;

        Ok(format!("{}/{}/{:06}", branch_code, fy, seq))
    }

    pub async fn generate_invoice_for_order(
        &self,
        ctx: &TenantContext,
        order_id: OrderId,
        branch_id: BranchId,
        confirmed_at: DateTime<Utc>,
    ) -> Result<InvoiceDto, TaxError> {
        // 1. Fetch branch code
        let branch_code_row =
            sqlx::query("SELECT code FROM branches WHERE tenant_id = $1 AND id = $2")
                .bind(ctx.tenant_id().0)
                .bind(branch_id.0)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| TaxError::BadRequest("Branch not found".into()))?;

        let branch_code: String = branch_code_row.get("code");

        // 2. Fetch order items with product tax category names
        let item_rows = sqlx::query(
            "SELECT oi.product_id, p.name as item_name, oi.unit_price, oi.quantity,
                    COALESCE(c.name, 'General Medicines') as tax_cat_name
             FROM order_items oi
             JOIN products p ON p.id = oi.product_id AND p.tenant_id = oi.tenant_id
             LEFT JOIN categories c ON c.id = p.category_id AND c.tenant_id = p.tenant_id
             WHERE oi.tenant_id = $1 AND oi.order_id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(order_id.0)
        .fetch_all(&self.pool)
        .await?;

        let mut taxable_items = Vec::new();
        for row in item_rows {
            let item_name: String = row.get("item_name");
            let unit_price_dec: Decimal = row.get("unit_price");
            let qty: i32 = row.get("quantity");
            let cat_name: String = row.get("tax_cat_name");

            taxable_items.push(TaxableItemInput {
                item_name,
                unit_price: Money::from_decimal(unit_price_dec),
                quantity: qty,
                discount: None,
                tax_category_name: cat_name,
            });
        }

        // 3. Fetch tax categories and calculate tax
        let categories = self.list_tax_categories(ctx).await?;
        let tax_result = TaxCalculator::calculate_tax(&taxable_items, &categories, confirmed_at)?;

        // 4. Atomically allocate gapless sequence number and persist invoice
        let mut tx = self.pool.begin().await?;

        let invoice_id = InvoiceId::new();
        let invoice_no = self
            .get_next_gapless_invoice_number(
                &mut tx,
                ctx.tenant_id(),
                branch_id,
                &branch_code,
                confirmed_at,
            )
            .await?;

        let lines_json = serde_json::to_value(&tax_result.lines).unwrap_or(json!([]));

        sqlx::query(
            "INSERT INTO invoices (
                id, tenant_id, branch_id, order_id, invoice_no, fiscal_invoice_no, status,
                subtotal, tax_amount, total_amount, lines, fbr_status, fbr_queue_status,
                issued_at, is_provisional, retry_count
             ) VALUES (
                $1, $2, $3, $4, $5, NULL, 'ISSUED',
                $6, $7, $8, $9, 'PENDING'::fbr_status_type, 'PENDING',
                $10, false, 0
             )",
        )
        .bind(invoice_id.0)
        .bind(ctx.tenant_id().0)
        .bind(branch_id.0)
        .bind(order_id.0)
        .bind(&invoice_no)
        .bind(tax_result.subtotal.0)
        .bind(tax_result.tax_amount.0)
        .bind(tax_result.total_amount.0)
        .bind(lines_json)
        .bind(confirmed_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_invoice(ctx, invoice_id).await
    }

    pub async fn get_invoice(
        &self,
        ctx: &TenantContext,
        invoice_id: InvoiceId,
    ) -> Result<InvoiceDto, TaxError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, branch_id, order_id, invoice_no, fiscal_invoice_no, status,
                    subtotal, tax_amount, total_amount, lines, fbr_queue_status,
                    fbr_request, fbr_response, fbr_qr_payload, fbr_error, pdf_object_key,
                    is_provisional, credit_note_for, credit_note_reason, retry_count,
                    issued_at, created_at, updated_at
             FROM invoices
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(invoice_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(TaxError::InvoiceNotFound(invoice_id))?;

        self.map_invoice_row(row)
    }

    pub async fn list_invoices(
        &self,
        ctx: &TenantContext,
        branch_id: Option<BranchId>,
        status: Option<InvoiceStatus>,
        fbr_status: Option<FbrQueueStatus>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<Vec<InvoiceDto>, TaxError> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, tenant_id, branch_id, order_id, invoice_no, fiscal_invoice_no, status,
                    subtotal, tax_amount, total_amount, lines, fbr_queue_status,
                    fbr_request, fbr_response, fbr_qr_payload, fbr_error, pdf_object_key,
                    is_provisional, credit_note_for, credit_note_reason, retry_count,
                    issued_at, created_at, updated_at
             FROM invoices
             WHERE tenant_id = ",
        );
        query_builder.push_bind(ctx.tenant_id().0);

        if let Some(bid) = branch_id {
            query_builder.push(" AND branch_id = ");
            query_builder.push_bind(bid.0);
        }

        if let Some(st) = status {
            query_builder.push(" AND status = ");
            query_builder.push_bind(st.to_string());
        }

        if let Some(fst) = fbr_status {
            query_builder.push(" AND fbr_queue_status = ");
            query_builder.push_bind(fst.to_string());
        }

        if let Some(from) = from_date {
            query_builder.push(" AND issued_at::date >= ");
            query_builder.push_bind(from);
        }

        if let Some(to) = to_date {
            query_builder.push(" AND issued_at::date <= ");
            query_builder.push_bind(to);
        }

        query_builder.push(" ORDER BY issued_at DESC");

        let rows = query_builder.build().fetch_all(&self.pool).await?;
        let mut list = Vec::new();
        for row in rows {
            list.push(self.map_invoice_row(row)?);
        }
        Ok(list)
    }

    // --------------------------------------------------------------------------------------------
    // FBR Submission Processing & Retry Queue (Doc 13 §7)
    // --------------------------------------------------------------------------------------------

    pub async fn process_fbr_submission(
        &self,
        ctx: &TenantContext,
        invoice_id: InvoiceId,
        reporter: &dyn FiscalReporter,
    ) -> Result<InvoiceDto, TaxError> {
        let invoice = self.get_invoice(ctx, invoice_id).await?;

        // Update status to SUBMITTING
        sqlx::query(
            "UPDATE invoices SET fbr_queue_status = 'SUBMITTING', updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(invoice_id.0)
        .execute(&self.pool)
        .await?;

        match reporter.submit(&invoice).await {
            Ok(resp) => {
                // ACCEPTED: store fiscal invoice no, QR code payload, raw response
                sqlx::query(
                    "UPDATE invoices SET
                        fiscal_invoice_no = $1,
                        fbr_queue_status = 'ACCEPTED',
                        fbr_status = 'TRANSMITTED'::fbr_status_type,
                        fbr_response = $2,
                        fbr_qr_payload = $3,
                        fbr_submitted_at = now(),
                        fbr_error = NULL,
                        updated_at = now()
                     WHERE tenant_id = $4 AND id = $5",
                )
                .bind(&resp.fiscal_invoice_no)
                .bind(&resp.raw_response)
                .bind(&resp.qr_code_data)
                .bind(ctx.tenant_id().0)
                .bind(invoice_id.0)
                .execute(&self.pool)
                .await?;
            }
            Err(TaxError::FbrRejection { reason, code }) => {
                // REJECTED: validation error, do NOT retry (Doc 13 §7, §12)
                let err_msg = format!("{}: {:?}", reason, code);
                sqlx::query(
                    "UPDATE invoices SET
                        fbr_queue_status = 'REJECTED',
                        fbr_status = 'FAILED'::fbr_status_type,
                        fbr_error = $1,
                        updated_at = now()
                     WHERE tenant_id = $2 AND id = $3",
                )
                .bind(&err_msg)
                .bind(ctx.tenant_id().0)
                .bind(invoice_id.0)
                .execute(&self.pool)
                .await?;
            }
            Err(TaxError::FbrOutage { message }) => {
                // FAILED: network / 5xx outage, retry with backoff
                sqlx::query(
                    "UPDATE invoices SET
                        fbr_queue_status = 'FAILED',
                        fbr_status = 'FAILED'::fbr_status_type,
                        retry_count = retry_count + 1,
                        fbr_error = $1,
                        updated_at = now()
                     WHERE tenant_id = $2 AND id = $3",
                )
                .bind(&message)
                .bind(ctx.tenant_id().0)
                .bind(invoice_id.0)
                .execute(&self.pool)
                .await?;
            }
            Err(e) => {
                let err_msg = e.to_string();
                sqlx::query(
                    "UPDATE invoices SET
                        fbr_queue_status = 'FAILED',
                        fbr_status = 'FAILED'::fbr_status_type,
                        retry_count = retry_count + 1,
                        fbr_error = $1,
                        updated_at = now()
                     WHERE tenant_id = $2 AND id = $3",
                )
                .bind(&err_msg)
                .bind(ctx.tenant_id().0)
                .bind(invoice_id.0)
                .execute(&self.pool)
                .await?;
            }
        }

        self.get_invoice(ctx, invoice_id).await
    }

    pub async fn resubmit_invoice(
        &self,
        ctx: &TenantContext,
        invoice_id: InvoiceId,
        reporter: &dyn FiscalReporter,
    ) -> Result<InvoiceDto, TaxError> {
        ctx.require("report.view")
            .map_err(|e| TaxError::Unauthorized(e.to_string()))?;

        self.process_fbr_submission(ctx, invoice_id, reporter).await
    }

    // --------------------------------------------------------------------------------------------
    // Credit Notes (Doc 13 §10)
    // --------------------------------------------------------------------------------------------

    pub async fn create_credit_note(
        &self,
        ctx: &TenantContext,
        original_invoice_id: InvoiceId,
        req: CreateCreditNoteRequest,
        reporter: &dyn FiscalReporter,
    ) -> Result<InvoiceDto, TaxError> {
        ctx.require("order.refund")
            .map_err(|e| TaxError::Unauthorized(e.to_string()))?;

        let original = self.get_invoice(ctx, original_invoice_id).await?;

        // Check if credit note already issued
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM invoices WHERE tenant_id = $1 AND credit_note_for = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(original_invoice_id.0)
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Err(TaxError::CreditNoteAlreadyIssued(original_invoice_id));
        }

        let branch_code_row =
            sqlx::query("SELECT code FROM branches WHERE tenant_id = $1 AND id = $2")
                .bind(ctx.tenant_id().0)
                .bind(original.branch_id.0)
                .fetch_one(&self.pool)
                .await?;

        let branch_code: String = branch_code_row.get("code");
        let now = Utc::now();

        let mut tx = self.pool.begin().await?;

        // Allocate gapless sequence for the credit note
        let credit_invoice_no = self
            .get_next_gapless_invoice_number(
                &mut tx,
                ctx.tenant_id(),
                original.branch_id,
                &branch_code,
                now,
            )
            .await?;

        let credit_invoice_id = InvoiceId::new();
        let neg_subtotal = Money::from_decimal(-original.subtotal.0);
        let neg_tax = Money::from_decimal(-original.tax_amount.0);
        let neg_total = Money::from_decimal(-original.total_amount.0);
        let lines_json = serde_json::to_value(&original.lines).unwrap_or(json!([]));

        // Insert Credit Note
        sqlx::query(
            "INSERT INTO invoices (
                id, tenant_id, branch_id, order_id, invoice_no, fiscal_invoice_no, status,
                subtotal, tax_amount, total_amount, lines, fbr_status, fbr_queue_status,
                issued_at, credit_note_for, credit_note_reason
             ) VALUES (
                $1, $2, $3, $4, $5, NULL, 'ISSUED',
                $6, $7, $8, $9, 'PENDING'::fbr_status_type, 'PENDING',
                $10, $11, $12
             )",
        )
        .bind(credit_invoice_id.0)
        .bind(ctx.tenant_id().0)
        .bind(original.branch_id.0)
        .bind(original.order_id.0)
        .bind(&credit_invoice_no)
        .bind(neg_subtotal.0)
        .bind(neg_tax.0)
        .bind(neg_total.0)
        .bind(lines_json)
        .bind(now)
        .bind(original_invoice_id.0)
        .bind(&req.reason)
        .execute(&mut *tx)
        .await?;

        // Update original invoice status to REFUNDED
        sqlx::query(
            "UPDATE invoices SET status = 'REFUNDED', updated_at = now() WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(original_invoice_id.0)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // Call FBR void reporting if original had a fiscal invoice no
        if let Some(ref fisc_no) = original.fiscal_invoice_no {
            reporter.void(fisc_no, &req.reason).await.ok();
        }

        self.get_invoice(ctx, credit_invoice_id).await
    }

    // --------------------------------------------------------------------------------------------
    // Reports & Monitoring
    // --------------------------------------------------------------------------------------------

    pub async fn get_tax_report(
        &self,
        ctx: &TenantContext,
        from_date: NaiveDate,
        to_date: NaiveDate,
        branch_id: Option<BranchId>,
    ) -> Result<TaxReportDto, TaxError> {
        ctx.require("report.view")
            .map_err(|e| TaxError::Unauthorized(e.to_string()))?;

        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT COALESCE(SUM(subtotal), 0.0000) as total_taxable,
                    COALESCE(SUM(tax_amount), 0.0000) as total_tax,
                    COALESCE(SUM(total_amount), 0.0000) as total_sales,
                    COUNT(id) as total_invoices
             FROM invoices
             WHERE tenant_id = ",
        );
        query_builder.push_bind(ctx.tenant_id().0);
        query_builder.push(" AND issued_at::date >= ");
        query_builder.push_bind(from_date);
        query_builder.push(" AND issued_at::date <= ");
        query_builder.push_bind(to_date);

        if let Some(bid) = branch_id {
            query_builder.push(" AND branch_id = ");
            query_builder.push_bind(bid.0);
        }

        let summary_row = query_builder.build().fetch_one(&self.pool).await?;
        let total_taxable_dec: Decimal = summary_row.get("total_taxable");
        let total_tax_dec: Decimal = summary_row.get("total_tax");
        let total_sales_dec: Decimal = summary_row.get("total_sales");
        let total_invoices: i64 = summary_row.get("total_invoices");

        Ok(TaxReportDto {
            from_date: from_date.to_string(),
            to_date: to_date.to_string(),
            branch_id,
            summary: TaxReportSummary {
                taxable_sales: Money::from_decimal(total_taxable_dec),
                exempt_sales: Money::zero(),
                zero_rated_sales: Money::zero(),
                total_sales: Money::from_decimal(total_sales_dec),
                total_tax_collected: Money::from_decimal(total_tax_dec),
                total_invoices_count: total_invoices,
            },
            lines: Vec::new(),
        })
    }

    pub async fn get_fbr_queue_status(
        &self,
        ctx: &TenantContext,
    ) -> Result<FbrQueueStatusDto, TaxError> {
        ctx.require("report.view")
            .map_err(|e| TaxError::Unauthorized(e.to_string()))?;

        let cutoff_stale = Utc::now() - chrono::Duration::hours(6);

        let row = sqlx::query(
            "SELECT
                COUNT(id) FILTER (WHERE fbr_queue_status = 'PENDING') as pending_count,
                COUNT(id) FILTER (WHERE fbr_queue_status = 'SUBMITTING') as submitting_count,
                COUNT(id) FILTER (WHERE fbr_queue_status = 'ACCEPTED') as accepted_count,
                COUNT(id) FILTER (WHERE fbr_queue_status = 'REJECTED') as rejected_count,
                COUNT(id) FILTER (WHERE fbr_queue_status = 'FAILED') as failed_count,
                COUNT(id) FILTER (WHERE fbr_queue_status = 'PENDING' AND created_at < $2) as stale_pending_count
             FROM invoices
             WHERE tenant_id = $1"
        )
        .bind(ctx.tenant_id().0)
        .bind(cutoff_stale)
        .fetch_one(&self.pool)
        .await?;

        let pending_count: i64 = row.get("pending_count");
        let submitting_count: i64 = row.get("submitting_count");
        let accepted_count: i64 = row.get("accepted_count");
        let rejected_count: i64 = row.get("rejected_count");
        let failed_count: i64 = row.get("failed_count");
        let stale_pending_count: i64 = row.get("stale_pending_count");

        Ok(FbrQueueStatusDto {
            pending_count,
            submitting_count,
            accepted_count,
            rejected_count,
            failed_count,
            stale_pending_count,
        })
    }

    // --------------------------------------------------------------------------------------------
    // Helpers
    // --------------------------------------------------------------------------------------------

    fn map_tax_category_row(&self, row: sqlx::postgres::PgRow) -> Result<TaxCategoryDto, TaxError> {
        let id: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let name: String = row.get("name");
        let rate: Decimal = row.get("rate");
        let fbr_code: Option<String> = row.get("fbr_code");
        let is_exempt: bool = row.get("is_exempt");
        let is_zero_rated: bool = row.get("is_zero_rated");
        let effective_from = row.get("effective_from");
        let effective_to = row.get("effective_to");
        let created_at = row.get("created_at");

        Ok(TaxCategoryDto {
            id: TaxCategoryId::from(id),
            tenant_id: TenantId::from(tid),
            name,
            rate,
            fbr_code,
            is_exempt,
            is_zero_rated,
            effective_from,
            effective_to,
            created_at,
        })
    }

    fn map_invoice_row(&self, row: sqlx::postgres::PgRow) -> Result<InvoiceDto, TaxError> {
        let id: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let bid: Uuid = row.get("branch_id");
        let oid: Uuid = row.get("order_id");
        let invoice_no: String = row.get("invoice_no");
        let fiscal_invoice_no: Option<String> = row.get("fiscal_invoice_no");
        let status_str: String = row.get("status");
        let subtotal_dec: Decimal = row.get("subtotal");
        let tax_dec: Decimal = row.get("tax_amount");
        let total_dec: Decimal = row.get("total_amount");
        let lines_val: serde_json::Value = row.get("lines");
        let fbr_queue_str: String = row.get("fbr_queue_status");
        let fbr_request = row.get("fbr_request");
        let fbr_response = row.get("fbr_response");
        let fbr_qr_payload = row.get("fbr_qr_payload");
        let fbr_error = row.get("fbr_error");
        let pdf_object_key = row.get("pdf_object_key");
        let is_provisional: bool = row.get("is_provisional");
        let credit_note_for: Option<Uuid> = row.get("credit_note_for");
        let credit_note_reason = row.get("credit_note_reason");
        let retry_count: i32 = row.get("retry_count");
        let issued_at = row.get("issued_at");
        let created_at = row.get("created_at");
        let updated_at = row.get("updated_at");

        let status = status_str.parse().unwrap_or(InvoiceStatus::Issued);
        let fbr_queue_status = fbr_queue_str.parse().unwrap_or(FbrQueueStatus::Pending);
        let lines: Vec<TaxLine> = serde_json::from_value(lines_val).unwrap_or_default();

        Ok(InvoiceDto {
            id: InvoiceId::from(id),
            tenant_id: TenantId::from(tid),
            branch_id: BranchId::from(bid),
            order_id: OrderId::from(oid),
            invoice_no,
            fiscal_invoice_no,
            status,
            subtotal: Money::from_decimal(subtotal_dec),
            tax_amount: Money::from_decimal(tax_dec),
            total_amount: Money::from_decimal(total_dec),
            lines,
            fbr_queue_status,
            fbr_request,
            fbr_response,
            fbr_qr_payload,
            fbr_error,
            pdf_object_key,
            is_provisional,
            credit_note_for: credit_note_for.map(InvoiceId::from),
            credit_note_reason,
            retry_count,
            issued_at,
            created_at,
            updated_at,
        })
    }
}
