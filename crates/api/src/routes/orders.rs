use crate::error::ApiError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, OrderId};
use shifa_orders::models::*;

#[derive(Debug, Deserialize)]
pub struct OrderQueryParams {
    pub branch_id: Option<uuid::Uuid>,
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/orders",
    params(
        ("branch_id" = Option<uuid::Uuid>, Query, description = "Filter by branch ID"),
        ("status" = Option<String>, Query, description = "Filter by order status")
    ),
    responses(
        (status = 200, description = "List of orders", body = Vec<OrderDto>)
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn list_orders(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<OrderQueryParams>,
) -> Result<Json<Vec<OrderDto>>, ApiError> {
    let list = state
        .order_service
        .list_orders(
            &ctx,
            params.branch_id.map(BranchId::from),
            params.status.as_deref(),
        )
        .await?;
    Ok(Json(list))
}

#[utoipa::path(
    post,
    path = "/api/v1/orders",
    request_body = CreateDraftOrderRequest,
    responses(
        (status = 201, description = "Draft order created", body = OrderDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn create_order(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<CreateDraftOrderRequest>,
) -> Result<(StatusCode, Json<OrderDto>), ApiError> {
    let order = state.order_service.create_draft_order(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(order)))
}

#[utoipa::path(
    get,
    path = "/api/v1/orders/{id}",
    params(
        ("id" = uuid::Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order details", body = OrderDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn get_order(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<OrderDto>, ApiError> {
    let order = state
        .order_service
        .get_order(&ctx, OrderId::from(id))
        .await?;
    Ok(Json(order))
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/{id}/items",
    params(
        ("id" = uuid::Uuid, Path, description = "Order ID")
    ),
    request_body = AddOrderItemRequest,
    responses(
        (status = 200, description = "Item added to order", body = OrderDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn add_item(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<AddOrderItemRequest>,
) -> Result<Json<OrderDto>, ApiError> {
    let order = state
        .order_service
        .add_order_item(&ctx, OrderId::from(id), req)
        .await?;
    Ok(Json(order))
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/{id}/confirm-cart",
    params(
        ("id" = uuid::Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Cart confirmed", body = OrderDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn confirm_cart(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<OrderDto>, ApiError> {
    let order = state
        .order_service
        .confirm_cart(&ctx, OrderId::from(id))
        .await?;
    Ok(Json(order))
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/{id}/transition",
    params(
        ("id" = uuid::Uuid, Path, description = "Order ID")
    ),
    request_body = TransitionOrderRequest,
    responses(
        (status = 200, description = "Order state transitioned", body = OrderDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Orders"
)]
pub async fn transition_order(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<TransitionOrderRequest>,
) -> Result<Json<OrderDto>, ApiError> {
    let order = state
        .order_service
        .transition_order(&ctx, OrderId::from(id), req)
        .await?;
    Ok(Json(order))
}
