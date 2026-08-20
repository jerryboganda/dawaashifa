use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, InvoiceId, TaxCategoryId};
use shifa_tax::fbr::MockFbrReporter;
use shifa_tax::models::*;
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListInvoicesQuery {
    pub branch_id: Option<Uuid>,
    pub status: Option<String>,
    pub fbr_status: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct TaxReportQuery {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub branch_id: Option<Uuid>,
}

// ------------------------------------------------------------------------------------------------
// Invoices
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/invoices",
    params(
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch ID"),
        ("status" = Option<String>, Query, description = "Filter by invoice status (ISSUED, CANCELLED, REFUNDED)"),
        ("fbr_status" = Option<String>, Query, description = "Filter by FBR queue status (PENDING, SUBMITTING, ACCEPTED, REJECTED, FAILED)"),
        ("from" = Option<NaiveDate>, Query, description = "From date (YYYY-MM-DD)"),
        ("to" = Option<NaiveDate>, Query, description = "To date (YYYY-MM-DD)")
    ),
    responses(
        (status = 200, description = "List of invoices", body = Vec<InvoiceDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Invoices"
)]
pub async fn list_invoices(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<Vec<InvoiceDto>>, ApiError> {
    let branch_id = query.branch_id.map(BranchId::from);
    let status = query.status.and_then(|s| s.parse().ok());
    let fbr_status = query.fbr_status.and_then(|s| s.parse().ok());

    let invoices = state
        .tax_service
        .list_invoices(&ctx, branch_id, status, fbr_status, query.from, query.to)
        .await?;

    Ok(Json(invoices))
}

#[utoipa::path(
    get,
    path = "/api/v1/invoices/{id}",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice details", body = InvoiceDto),
        (status = 404, description = "Invoice not found")
    ),
    tag = "Invoices"
)]
pub async fn get_invoice(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<InvoiceDto>, ApiError> {
    let invoice = state
        .tax_service
        .get_invoice(&ctx, InvoiceId::from(id))
        .await?;

    Ok(Json(invoice))
}

#[utoipa::path(
    get,
    path = "/api/v1/invoices/{id}/pdf",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice PDF URL / payload", body = String),
        (status = 404, description = "Invoice not found")
    ),
    tag = "Invoices"
)]
pub async fn get_invoice_pdf(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let invoice = state
        .tax_service
        .get_invoice(&ctx, InvoiceId::from(id))
        .await?;

    let pdf_key = invoice
        .pdf_object_key
        .unwrap_or_else(|| format!("invoices/{}/invoice.pdf", invoice.invoice_no));

    Ok(Json(serde_json::json!({
        "invoice_no": invoice.invoice_no,
        "fiscal_invoice_no": invoice.fiscal_invoice_no,
        "pdf_url": format!("https://s3.shifa.pk/{}", pdf_key),
        "qr_code": invoice.fbr_qr_payload
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/invoices/{id}/resubmit",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    responses(
        (status = 200, description = "Invoice resubmitted to FBR", body = InvoiceDto),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invoice not found")
    ),
    tag = "Invoices"
)]
pub async fn resubmit_invoice(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<InvoiceDto>, ApiError> {
    let reporter = MockFbrReporter::new(shifa_tax::fbr::MockFbrBehavior::AlwaysAccept);
    let invoice = state
        .tax_service
        .resubmit_invoice(&ctx, InvoiceId::from(id), &reporter)
        .await?;

    Ok(Json(invoice))
}

#[utoipa::path(
    post,
    path = "/api/v1/invoices/{id}/credit-note",
    params(
        ("id" = Uuid, Path, description = "Invoice ID")
    ),
    request_body = CreateCreditNoteRequest,
    responses(
        (status = 201, description = "Credit note issued", body = InvoiceDto),
        (status = 400, description = "Credit note already issued"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Invoice not found")
    ),
    tag = "Invoices"
)]
pub async fn create_credit_note(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCreditNoteRequest>,
) -> Result<(StatusCode, Json<InvoiceDto>), ApiError> {
    let reporter = MockFbrReporter::new(shifa_tax::fbr::MockFbrBehavior::AlwaysAccept);
    let credit_note = state
        .tax_service
        .create_credit_note(&ctx, InvoiceId::from(id), req, &reporter)
        .await?;

    Ok((StatusCode::CREATED, Json(credit_note)))
}

// ------------------------------------------------------------------------------------------------
// Tax Categories
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/tax/categories",
    responses(
        (status = 200, description = "List of tax categories", body = Vec<TaxCategoryDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Tax"
)]
pub async fn list_tax_categories(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
) -> Result<Json<Vec<TaxCategoryDto>>, ApiError> {
    let categories = state.tax_service.list_tax_categories(&ctx).await?;
    Ok(Json(categories))
}

#[utoipa::path(
    post,
    path = "/api/v1/tax/categories",
    request_body = CreateTaxCategoryRequest,
    responses(
        (status = 201, description = "Tax category created", body = TaxCategoryDto),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Tax"
)]
pub async fn create_tax_category(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<CreateTaxCategoryRequest>,
) -> Result<(StatusCode, Json<TaxCategoryDto>), ApiError> {
    let category = state.tax_service.create_tax_category(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(category)))
}

#[utoipa::path(
    patch,
    path = "/api/v1/tax/categories/{id}",
    params(
        ("id" = Uuid, Path, description = "Tax category ID")
    ),
    request_body = PatchTaxCategoryRequest,
    responses(
        (status = 200, description = "Tax category updated with new rate period", body = TaxCategoryDto),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tax category not found")
    ),
    tag = "Tax"
)]
pub async fn patch_tax_category(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchTaxCategoryRequest>,
) -> Result<Json<TaxCategoryDto>, ApiError> {
    let category = state
        .tax_service
        .patch_tax_category(&ctx, TaxCategoryId::from(id), req)
        .await?;

    Ok(Json(category))
}

// ------------------------------------------------------------------------------------------------
// Reports & Monitoring
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/tax/report",
    params(
        ("from" = NaiveDate, Query, description = "From date (YYYY-MM-DD)"),
        ("to" = NaiveDate, Query, description = "To date (YYYY-MM-DD)"),
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch ID")
    ),
    responses(
        (status = 200, description = "Tax report", body = TaxReportDto),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Tax"
)]
pub async fn get_tax_report(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<TaxReportQuery>,
) -> Result<Json<TaxReportDto>, ApiError> {
    let branch_id = query.branch_id.map(BranchId::from);

    let report = state
        .tax_service
        .get_tax_report(&ctx, query.from, query.to, branch_id)
        .await?;

    Ok(Json(report))
}

#[utoipa::path(
    get,
    path = "/api/v1/fbr/queue-status",
    responses(
        (status = 200, description = "FBR submission queue status and health counts", body = FbrQueueStatusDto),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Tax"
)]
pub async fn get_fbr_queue_status(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
) -> Result<Json<FbrQueueStatusDto>, ApiError> {
    let status = state.tax_service.get_fbr_queue_status(&ctx).await?;
    Ok(Json(status))
}
