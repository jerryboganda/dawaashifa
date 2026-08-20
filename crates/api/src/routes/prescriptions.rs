use crate::error::ApiError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use shifa_core::context::TenantContext;
use shifa_core::id::PrescriptionId;
use shifa_prescription::models::*;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ListPrescriptionsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/api/v1/prescriptions",
    request_body = CreatePrescriptionRequest,
    responses(
        (status = 200, description = "Prescription received and queued", body = PrescriptionDto),
        (status = 400, description = "Invalid request")
    ),
    tag = "Prescriptions"
)]
pub async fn create_prescription(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<CreatePrescriptionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let rx = state
        .prescription_service
        .create_prescription(&ctx, req)
        .await?;
    Ok(Json(rx))
}

#[utoipa::path(
    get,
    path = "/api/v1/prescriptions",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("limit" = Option<i64>, Query, description = "Limit (default 50)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List of prescriptions", body = Vec<PrescriptionDto>)
    ),
    tag = "Prescriptions"
)]
pub async fn list_prescriptions(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(query): Query<ListPrescriptionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let list = state
        .prescription_service
        .list_prescriptions(&ctx, query.status.as_deref(), limit, offset)
        .await?;
    Ok(Json(list))
}

#[utoipa::path(
    get,
    path = "/api/v1/prescriptions/{id}",
    params(
        ("id" = Uuid, Path, description = "Prescription ID")
    ),
    responses(
        (status = 200, description = "Prescription full detail with lines and candidates", body = PrescriptionDto),
        (status = 404, description = "Not found")
    ),
    tag = "Prescriptions"
)]
pub async fn get_prescription(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let rx = state
        .prescription_service
        .get_prescription(&ctx, PrescriptionId::from(id))
        .await?;
    Ok(Json(rx))
}

#[utoipa::path(
    post,
    path = "/api/v1/prescriptions/{id}/extract",
    params(
        ("id" = Uuid, Path, description = "Prescription ID")
    ),
    responses(
        (status = 200, description = "Re-run extraction result", body = PrescriptionDto)
    ),
    tag = "Prescriptions"
)]
pub async fn extract_prescription(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let rx = state
        .prescription_service
        .extract_prescription(&ctx, PrescriptionId::from(id))
        .await?;
    Ok(Json(rx))
}

#[utoipa::path(
    post,
    path = "/api/v1/prescriptions/{id}/claim",
    params(
        ("id" = Uuid, Path, description = "Prescription ID")
    ),
    responses(
        (status = 200, description = "Claim prescription for review", body = PrescriptionDto),
        (status = 409, description = "Already claimed by another pharmacist")
    ),
    tag = "Prescriptions"
)]
pub async fn claim_prescription(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let rx = state
        .prescription_service
        .claim_prescription(&ctx, PrescriptionId::from(id))
        .await?;
    Ok(Json(rx))
}

#[utoipa::path(
    post,
    path = "/api/v1/prescriptions/{id}/approve",
    params(
        ("id" = Uuid, Path, description = "Prescription ID")
    ),
    request_body = ApprovePrescriptionRequest,
    responses(
        (status = 200, description = "Approval result recorded immutably", body = ApprovalResult),
        (status = 400, description = "Incomplete review / missing decision for line"),
        (status = 403, description = "Forbidden (requires rx.approve permission)")
    ),
    tag = "Prescriptions"
)]
pub async fn approve_prescription(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(req): Json<ApprovePrescriptionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let res = state
        .prescription_service
        .approve(&ctx, PrescriptionId::from(id), req)
        .await?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/api/v1/prescriptions/{id}/reject",
    params(
        ("id" = Uuid, Path, description = "Prescription ID")
    ),
    request_body = RejectPrescriptionRequest,
    responses(
        (status = 200, description = "Rejection recorded immutably", body = ApprovalResult),
        (status = 403, description = "Forbidden")
    ),
    tag = "Prescriptions"
)]
pub async fn reject_prescription(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(req): Json<RejectPrescriptionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let res = state
        .prescription_service
        .reject(&ctx, PrescriptionId::from(id), req)
        .await?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/api/v1/prescriptions/{id}/clarify",
    params(
        ("id" = Uuid, Path, description = "Prescription ID")
    ),
    request_body = ClarifyPrescriptionRequest,
    responses(
        (status = 200, description = "Clarification requested", body = PrescriptionDto),
        (status = 403, description = "Forbidden")
    ),
    tag = "Prescriptions"
)]
pub async fn clarify_prescription(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(req): Json<ClarifyPrescriptionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let rx = state
        .prescription_service
        .clarify(&ctx, PrescriptionId::from(id), req)
        .await?;
    Ok(Json(rx))
}

#[utoipa::path(
    get,
    path = "/api/v1/prescriptions/queue/stats",
    responses(
        (status = 200, description = "Prescription review queue metrics", body = QueueStatsDto)
    ),
    tag = "Prescriptions"
)]
pub async fn get_queue_stats(
    State(state): State<AppState>,
    ctx: TenantContext,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state.prescription_service.get_queue_stats(&ctx).await?;
    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/api/v1/prescriptions/{id}/audit",
    params(
        ("id" = Uuid, Path, description = "Prescription ID")
    ),
    responses(
        (status = 200, description = "Full immutable audit trail", body = Vec<RxAuditEntryDto>)
    ),
    tag = "Prescriptions"
)]
pub async fn get_audit_trail(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let trail = state
        .prescription_service
        .get_audit_trail(&ctx, PrescriptionId::from(id))
        .await?;
    Ok(Json(trail))
}
