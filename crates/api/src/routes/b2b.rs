use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use shifa_b2b::models::*;
use shifa_core::context::TenantContext;
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct RecallQueryParams {
    pub product_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
}

pub fn b2b_routes() -> Router<AppState> {
    Router::new()
        .route("/accounts", get(list_accounts).post(create_account))
        .route("/accounts/:id/hold", post(set_account_hold))
        .route("/accounts/:id/ar-summary", get(get_ar_summary))
        .route("/quotations", post(create_quotation))
        .route("/quotations/:id/revise", post(revise_quotation))
        .route("/quotations/:id/accept", post(accept_quotation))
        .route("/purchase-orders", post(ingest_purchase_order))
        .route("/consignment/stock", post(place_consignment))
        .route("/consignment/:id/reconcile", post(reconcile_consignment))
        .route("/devices", post(register_device))
        .route("/devices/recall", get(query_device_recall))
}

#[utoipa::path(
    get,
    path = "/api/v1/b2b/accounts",
    tag = "B2B",
    responses(
        (status = 200, description = "List business accounts", body = Vec<BusinessAccountDto>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_accounts(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
) -> Result<impl IntoResponse, ApiError> {
    let accounts = state.b2b_service.list_accounts(&ctx).await?;
    Ok(Json(accounts))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/accounts",
    tag = "B2B",
    request_body = CreateAccountRequest,
    responses(
        (status = 201, description = "Business account created", body = BusinessAccountDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_account(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let account = state.b2b_service.create_account(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/accounts/{id}/hold",
    tag = "B2B",
    params(
        ("id" = Uuid, Path, description = "Account ID")
    ),
    request_body = AccountHoldRequest,
    responses(
        (status = 200, description = "Account hold status updated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_account_hold(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<AccountHoldRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state.b2b_service.set_account_hold(&ctx, id, req).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/v1/b2b/accounts/{id}/ar-summary",
    tag = "B2B",
    params(
        ("id" = Uuid, Path, description = "Account ID")
    ),
    responses(
        (status = 200, description = "AR Summary and Aging", body = ArSummaryDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_ar_summary(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let summary = state.b2b_service.get_ar_summary(&ctx, id).await?;
    Ok(Json(summary))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/quotations",
    tag = "B2B",
    request_body = CreateQuotationRequest,
    responses(
        (status = 201, description = "Quotation created", body = QuotationDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_quotation(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<CreateQuotationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let quote = state.b2b_service.create_quotation(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(quote)))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/quotations/{id}/revise",
    tag = "B2B",
    params(
        ("id" = Uuid, Path, description = "Parent Quotation ID")
    ),
    request_body = ReviseQuotationRequest,
    responses(
        (status = 201, description = "Quotation revised to new version", body = QuotationDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn revise_quotation(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReviseQuotationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let quote = state.b2b_service.revise_quotation(&ctx, id, req).await?;
    Ok((StatusCode::CREATED, Json(quote)))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/quotations/{id}/accept",
    tag = "B2B",
    params(
        ("id" = Uuid, Path, description = "Quotation ID")
    ),
    responses(
        (status = 200, description = "Quotation accepted and order created")
    ),
    security(("bearer_auth" = []))
)]
pub async fn accept_quotation(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let order_id = state.b2b_service.accept_quotation(&ctx, id).await?;
    Ok(Json(serde_json::json!({ "order_id": order_id.0 })))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/purchase-orders",
    tag = "B2B",
    request_body = CreatePurchaseOrderRequest,
    responses(
        (status = 201, description = "Purchase order uploaded and matched", body = PurchaseOrderDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn ingest_purchase_order(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<CreatePurchaseOrderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let po = state.b2b_service.ingest_purchase_order(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(po)))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/consignment/stock",
    tag = "B2B",
    request_body = PlaceConsignmentRequest,
    responses(
        (status = 201, description = "Consignment stock placed", body = ConsignmentStockDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn place_consignment(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<PlaceConsignmentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let stock = state.b2b_service.place_consignment(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(stock)))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/consignment/{id}/reconcile",
    tag = "B2B",
    params(
        ("id" = Uuid, Path, description = "Consignment Stock ID")
    ),
    request_body = ReconcileConsignmentRequest,
    responses(
        (status = 200, description = "Consignment stock reconciled", body = ConsignmentStockDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reconcile_consignment(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReconcileConsignmentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let stock = state
        .b2b_service
        .reconcile_consignment(&ctx, id, req)
        .await?;
    Ok(Json(stock))
}

#[utoipa::path(
    post,
    path = "/api/v1/b2b/devices",
    tag = "B2B",
    request_body = RegisterDeviceRequest,
    responses(
        (status = 201, description = "Device unit registered", body = DeviceUnitDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn register_device(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let device = state.b2b_service.register_device(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(device)))
}

#[utoipa::path(
    get,
    path = "/api/v1/b2b/devices/recall",
    tag = "B2B",
    params(
        RecallQueryParams
    ),
    responses(
        (status = 200, description = "Device recall query result", body = RecallQueryResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn query_device_recall(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(params): Query<RecallQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let res = state
        .b2b_service
        .query_recall(&ctx, params.product_id, params.batch_id)
        .await?;
    Ok(Json(res))
}
