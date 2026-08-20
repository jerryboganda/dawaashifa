use crate::error::ApiError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use shifa_core::context::TenantContext;
use shifa_core::id::{BatchId, BranchId, ProductId};
use shifa_inventory::models::*;

#[derive(Debug, Deserialize)]
pub struct StockQueryParams {
    pub branch_id: Option<uuid::Uuid>,
    pub product_id: Option<uuid::Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/v1/inventory/stock",
    params(
        ("branch_id" = Option<uuid::Uuid>, Query, description = "Filter by branch ID"),
        ("product_id" = Option<uuid::Uuid>, Query, description = "Filter by product ID")
    ),
    responses(
        (status = 200, description = "Current stock list", body = Vec<StockCurrentDto>)
    ),
    security(("bearer_auth" = [])),
    tag = "Inventory"
)]
pub async fn list_stock(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<StockQueryParams>,
) -> Result<Json<Vec<StockCurrentDto>>, ApiError> {
    let list = state
        .inventory_service
        .list_stock(
            &ctx,
            params.branch_id.map(BranchId::from),
            params.product_id.map(ProductId::from),
        )
        .await?;

    Ok(Json(list))
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/receipts",
    request_body = StockReceiptRequest,
    responses(
        (status = 201, description = "Stock received and movement recorded", body = BatchId)
    ),
    security(("bearer_auth" = [])),
    tag = "Inventory"
)]
pub async fn receive_stock(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<StockReceiptRequest>,
) -> Result<(StatusCode, Json<BatchId>), ApiError> {
    let batch_id = state.inventory_service.receive_stock(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(batch_id)))
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/adjustments",
    request_body = StockAdjustmentRequest,
    responses(
        (status = 200, description = "Stock adjusted")
    ),
    security(("bearer_auth" = [])),
    tag = "Inventory"
)]
pub async fn adjust_stock(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<StockAdjustmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.inventory_service.adjust_stock(&ctx, req).await?;
    Ok(Json(serde_json::json!({"status": "adjusted"})))
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/transfers",
    request_body = CreateTransferRequest,
    responses(
        (status = 201, description = "Transfer draft created", body = TransferDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Inventory"
)]
pub async fn create_transfer(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<CreateTransferRequest>,
) -> Result<(StatusCode, Json<TransferDto>), ApiError> {
    let transfer = state.transfer_service.create_transfer(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(transfer)))
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/transfers/{id}/dispatch",
    params(
        ("id" = uuid::Uuid, Path, description = "Transfer ID")
    ),
    responses(
        (status = 200, description = "Transfer dispatched", body = TransferDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Inventory"
)]
pub async fn dispatch_transfer(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<TransferDto>, ApiError> {
    let transfer = state.transfer_service.dispatch_transfer(&ctx, id).await?;
    Ok(Json(transfer))
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/cold-chain/logs",
    request_body = ColdChainLogRequest,
    responses(
        (status = 201, description = "Temperature logged")
    ),
    security(("bearer_auth" = [])),
    tag = "Inventory"
)]
pub async fn log_cold_chain(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<ColdChainLogRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let is_excursion = state
        .cold_chain_service
        .record_temperature(&ctx, req)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "recorded",
            "is_excursion": is_excursion
        })),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/cold-chain/{batch_id}/clear-excursion",
    params(
        ("batch_id" = uuid::Uuid, Path, description = "Batch ID")
    ),
    request_body = ClearExcursionRequest,
    responses(
        (status = 200, description = "Excursion cleared by pharmacist")
    ),
    security(("bearer_auth" = [])),
    tag = "Inventory"
)]
pub async fn clear_excursion(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(batch_id): Path<uuid::Uuid>,
    Json(req): Json<ClearExcursionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .cold_chain_service
        .clear_excursion(&ctx, BatchId::from(batch_id), req)
        .await?;
    Ok(Json(serde_json::json!({"status": "cleared"})))
}
