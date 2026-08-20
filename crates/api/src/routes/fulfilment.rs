use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, DeliveryId, PickingListId, RiderCashSessionId, RiderId};
use shifa_fulfilment::models::*;
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListPickingListsQuery {
    pub branch_id: Option<Uuid>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListRidersQuery {
    pub branch_id: Option<Uuid>,
    pub status: Option<String>,
    pub on_shift: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListDeliveriesQuery {
    pub branch_id: Option<Uuid>,
    pub rider_id: Option<Uuid>,
    pub status: Option<String>,
    pub date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct ListCashSessionsQuery {
    pub rider_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VarianceReportQuery {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub branch_id: Option<Uuid>,
}

// ------------------------------------------------------------------------------------------------
// Picking Lists
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/fulfilment/picking-lists",
    params(
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch ID"),
        ("status" = Option<String>, Query, description = "Filter by picking list status")
    ),
    responses(
        (status = 200, description = "List of picking lists", body = Vec<PickingListDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Fulfilment"
)]
pub async fn list_picking_lists(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<ListPickingListsQuery>,
) -> Result<Json<Vec<PickingListDto>>, ApiError> {
    let branch_id = query.branch_id.map(BranchId::from);
    let status = query.status.and_then(|s| s.parse().ok());

    let lists = state
        .fulfilment_service
        .list_picking_lists(&ctx, branch_id, status)
        .await?;

    Ok(Json(lists))
}

#[utoipa::path(
    post,
    path = "/api/v1/fulfilment/picking-lists/{id}/complete",
    params(
        ("id" = Uuid, Path, description = "Picking list ID")
    ),
    responses(
        (status = 200, description = "Picking list marked completed", body = PickingListDto),
        (status = 404, description = "Picking list not found")
    ),
    tag = "Fulfilment"
)]
pub async fn complete_picking_list(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<PickingListDto>, ApiError> {
    let picking_list = state
        .fulfilment_service
        .complete_picking_list(&ctx, PickingListId::from(id))
        .await?;

    Ok(Json(picking_list))
}

// ------------------------------------------------------------------------------------------------
// Rider Management
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/riders",
    params(
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch ID"),
        ("status" = Option<String>, Query, description = "Filter by rider status"),
        ("on_shift" = Option<bool>, Query, description = "Filter by on_shift flag")
    ),
    responses(
        (status = 200, description = "List of riders", body = Vec<RiderDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden for rider role")
    ),
    tag = "Fulfilment"
)]
pub async fn list_riders(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<ListRidersQuery>,
) -> Result<Json<Vec<RiderDto>>, ApiError> {
    let branch_id = query.branch_id.map(BranchId::from);
    let status = query.status.and_then(|s| s.parse().ok());

    let riders = state
        .fulfilment_service
        .list_riders(&ctx, branch_id, status, query.on_shift)
        .await?;

    Ok(Json(riders))
}

#[utoipa::path(
    post,
    path = "/api/v1/riders",
    request_body = CreateRiderRequest,
    responses(
        (status = 201, description = "Rider registered", body = RiderDto),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Fulfilment"
)]
pub async fn create_rider(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<CreateRiderRequest>,
) -> Result<(StatusCode, Json<RiderDto>), ApiError> {
    let rider = state.fulfilment_service.create_rider(&ctx, req).await?;
    Ok((StatusCode::CREATED, Json(rider)))
}

#[utoipa::path(
    post,
    path = "/api/v1/riders/{id}/shift/start",
    params(
        ("id" = Uuid, Path, description = "Rider ID")
    ),
    responses(
        (status = 200, description = "Rider shift started", body = RiderDto),
        (status = 404, description = "Rider not found")
    ),
    tag = "Fulfilment"
)]
pub async fn start_shift(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<RiderDto>, ApiError> {
    let rider = state
        .fulfilment_service
        .start_shift(&ctx, RiderId::from(id))
        .await?;

    Ok(Json(rider))
}

#[utoipa::path(
    post,
    path = "/api/v1/riders/{id}/shift/end",
    params(
        ("id" = Uuid, Path, description = "Rider ID")
    ),
    responses(
        (status = 200, description = "Rider shift ended", body = RiderDto),
        (status = 404, description = "Rider not found")
    ),
    tag = "Fulfilment"
)]
pub async fn end_shift(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<RiderDto>, ApiError> {
    let rider = state
        .fulfilment_service
        .end_shift(&ctx, RiderId::from(id))
        .await?;

    Ok(Json(rider))
}

// ------------------------------------------------------------------------------------------------
// Deliveries
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/deliveries",
    params(
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch ID"),
        ("rider_id" = Option<Uuid>, Query, description = "Filter by rider ID"),
        ("status" = Option<String>, Query, description = "Filter by delivery status"),
        ("date" = Option<NaiveDate>, Query, description = "Filter by date (YYYY-MM-DD)")
    ),
    responses(
        (status = 200, description = "List of deliveries", body = Vec<DeliveryDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Fulfilment"
)]
pub async fn list_deliveries(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<ListDeliveriesQuery>,
) -> Result<Json<Vec<DeliveryDto>>, ApiError> {
    let branch_id = query.branch_id.map(BranchId::from);
    let rider_id = query.rider_id.map(RiderId::from);
    let status = query.status.and_then(|s| s.parse().ok());

    let deliveries = state
        .fulfilment_service
        .list_deliveries(&ctx, branch_id, rider_id, status, query.date)
        .await?;

    Ok(Json(deliveries))
}

#[utoipa::path(
    post,
    path = "/api/v1/deliveries/{id}/assign",
    params(
        ("id" = Uuid, Path, description = "Delivery ID")
    ),
    request_body = AssignDeliveryRequest,
    responses(
        (status = 200, description = "Delivery assigned to rider", body = DeliveryDto),
        (status = 400, description = "Cash ceiling exceeded or stale session blocked"),
        (status = 404, description = "Delivery or rider not found")
    ),
    tag = "Fulfilment"
)]
pub async fn assign_delivery(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignDeliveryRequest>,
) -> Result<Json<DeliveryDto>, ApiError> {
    let delivery = state
        .fulfilment_service
        .assign_delivery(&ctx, DeliveryId::from(id), req.rider_id)
        .await?;

    Ok(Json(delivery))
}

#[utoipa::path(
    post,
    path = "/api/v1/deliveries/{id}/accept",
    params(
        ("id" = Uuid, Path, description = "Delivery ID")
    ),
    responses(
        (status = 200, description = "Delivery accepted by rider", body = DeliveryDto),
        (status = 403, description = "Forbidden for unassigned rider")
    ),
    tag = "Fulfilment"
)]
pub async fn accept_delivery(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeliveryDto>, ApiError> {
    let delivery = state
        .fulfilment_service
        .accept_delivery(&ctx, DeliveryId::from(id))
        .await?;

    Ok(Json(delivery))
}

#[utoipa::path(
    post,
    path = "/api/v1/deliveries/{id}/decline",
    params(
        ("id" = Uuid, Path, description = "Delivery ID")
    ),
    request_body = DeclineDeliveryRequest,
    responses(
        (status = 200, description = "Delivery declined and returned to unassigned", body = DeliveryDto),
        (status = 403, description = "Forbidden for unassigned rider")
    ),
    tag = "Fulfilment"
)]
pub async fn decline_delivery(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<DeclineDeliveryRequest>,
) -> Result<Json<DeliveryDto>, ApiError> {
    let delivery = state
        .fulfilment_service
        .decline_delivery(&ctx, DeliveryId::from(id), req)
        .await?;

    Ok(Json(delivery))
}

#[utoipa::path(
    post,
    path = "/api/v1/deliveries/{id}/pickup",
    params(
        ("id" = Uuid, Path, description = "Delivery ID")
    ),
    responses(
        (status = 200, description = "Delivery picked up from pharmacy", body = DeliveryDto),
        (status = 403, description = "Forbidden for unassigned rider")
    ),
    tag = "Fulfilment"
)]
pub async fn pickup_delivery(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeliveryDto>, ApiError> {
    let delivery = state
        .fulfilment_service
        .pickup_delivery(&ctx, DeliveryId::from(id))
        .await?;

    Ok(Json(delivery))
}

#[utoipa::path(
    post,
    path = "/api/v1/deliveries/{id}/deliver",
    params(
        ("id" = Uuid, Path, description = "Delivery ID")
    ),
    request_body = DeliverRequest,
    responses(
        (status = 200, description = "Delivery completed with proof of delivery", body = DeliveryDto),
        (status = 400, description = "Missing mandatory POD field or controlled substance requirements")
    ),
    tag = "Fulfilment"
)]
pub async fn complete_delivery(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<DeliverRequest>,
) -> Result<Json<DeliveryDto>, ApiError> {
    let delivery = state
        .fulfilment_service
        .complete_delivery(&ctx, DeliveryId::from(id), req)
        .await?;

    Ok(Json(delivery))
}

#[utoipa::path(
    post,
    path = "/api/v1/deliveries/{id}/fail",
    params(
        ("id" = Uuid, Path, description = "Delivery ID")
    ),
    request_body = FailDeliveryRequest,
    responses(
        (status = 200, description = "Delivery marked failed or returned", body = DeliveryDto),
        (status = 404, description = "Delivery not found")
    ),
    tag = "Fulfilment"
)]
pub async fn fail_delivery(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<FailDeliveryRequest>,
) -> Result<Json<DeliveryDto>, ApiError> {
    let delivery = state
        .fulfilment_service
        .fail_delivery(&ctx, DeliveryId::from(id), req)
        .await?;

    Ok(Json(delivery))
}

// ------------------------------------------------------------------------------------------------
// Cash Sessions & Daily Reconciliation
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/cash-sessions",
    params(
        ("rider_id" = Option<Uuid>, Query, description = "Filter by rider ID"),
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch ID"),
        ("status" = Option<String>, Query, description = "Filter by cash session status")
    ),
    responses(
        (status = 200, description = "List of rider cash sessions", body = Vec<RiderCashSessionDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Fulfilment"
)]
pub async fn list_cash_sessions(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<ListCashSessionsQuery>,
) -> Result<Json<Vec<RiderCashSessionDto>>, ApiError> {
    let rider_id = query.rider_id.map(RiderId::from);
    let branch_id = query.branch_id.map(BranchId::from);
    let status = query.status.and_then(|s| s.parse().ok());

    let sessions = state
        .fulfilment_service
        .list_cash_sessions(&ctx, rider_id, branch_id, status)
        .await?;

    Ok(Json(sessions))
}

#[utoipa::path(
    post,
    path = "/api/v1/cash-sessions/{id}/declare",
    params(
        ("id" = Uuid, Path, description = "Cash session ID")
    ),
    request_body = DeclareCashRequest,
    responses(
        (status = 200, description = "Rider cash declaration recorded", body = RiderCashSessionDto),
        (status = 404, description = "Cash session not found")
    ),
    tag = "Fulfilment"
)]
pub async fn declare_cash(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<DeclareCashRequest>,
) -> Result<Json<RiderCashSessionDto>, ApiError> {
    let session = state
        .fulfilment_service
        .declare_cash(&ctx, RiderCashSessionId::from(id), req)
        .await?;

    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/api/v1/cash-sessions/{id}/reconcile",
    params(
        ("id" = Uuid, Path, description = "Cash session ID")
    ),
    request_body = ReconcileCashSessionRequest,
    responses(
        (status = 200, description = "Cash session reconciled and closed", body = RiderCashSessionDto),
        (status = 400, description = "Variance requires documented note"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Fulfilment"
)]
pub async fn reconcile_cash_session(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReconcileCashSessionRequest>,
) -> Result<Json<RiderCashSessionDto>, ApiError> {
    let session = state
        .fulfilment_service
        .reconcile_cash_session(&ctx, RiderCashSessionId::from(id), req)
        .await?;

    Ok(Json(session))
}

#[utoipa::path(
    get,
    path = "/api/v1/cash-sessions/variance-report",
    params(
        ("start_date" = NaiveDate, Query, description = "Start date (YYYY-MM-DD)"),
        ("end_date" = NaiveDate, Query, description = "End date (YYYY-MM-DD)"),
        ("branch_id" = Option<Uuid>, Query, description = "Filter by branch ID")
    ),
    responses(
        (status = 200, description = "Cash variance report", body = VarianceReportDto),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Fulfilment"
)]
pub async fn get_variance_report(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<VarianceReportQuery>,
) -> Result<Json<VarianceReportDto>, ApiError> {
    let branch_id = query.branch_id.map(BranchId::from);

    let report = state
        .fulfilment_service
        .get_variance_report(&ctx, query.start_date, query.end_date, branch_id)
        .await?;

    Ok(Json(report))
}

// ------------------------------------------------------------------------------------------------
// Public Tracking (Zero PII, Unauthenticated)
// ------------------------------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/track/{token}",
    params(
        ("token" = String, Path, description = "Public tracking token")
    ),
    responses(
        (status = 200, description = "Customer tracking status without PII", body = PublicTrackingDto),
        (status = 404, description = "Invalid or expired tracking token")
    ),
    tag = "Fulfilment"
)]
pub async fn get_public_tracking(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let tracking = state.fulfilment_service.get_public_tracking(&token).await?;
    Ok(Json(tracking))
}
