use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use shifa_admin::models::*;
use shifa_core::context::TenantContext;

use crate::error::ApiError;
use crate::AppState;

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/audit", get(list_audit_events))
        .route("/audit/export", get(export_audit_csv))
        .route("/settings", get(get_settings).patch(update_settings))
        .route("/reports", get(get_reports))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/audit",
    tag = "Admin",
    params(AuditQueryRequest),
    responses(
        (status = 200, description = "List audit log events", body = Vec<AuditEventDto>)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_audit_events(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<AuditQueryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let events = state.admin_service.list_audit_events(&ctx, query).await?;
    Ok(Json(events))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/audit/export",
    tag = "Admin",
    params(AuditQueryRequest),
    responses(
        (status = 200, description = "Export audit log CSV", body = String)
    ),
    security(("bearer_auth" = []))
)]
pub async fn export_audit_csv(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Query(query): Query<AuditQueryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let csv = state.admin_service.export_audit_csv(&ctx, query).await?;
    let headers = [
        (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"audit_trail.csv\"",
        ),
    ];
    Ok((headers, csv))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/settings",
    tag = "Admin",
    responses(
        (status = 200, description = "Get system settings", body = SystemSettingsDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_settings(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = state.admin_service.get_system_settings(&ctx).await?;
    Ok(Json(settings))
}

#[utoipa::path(
    patch,
    path = "/api/v1/admin/settings",
    tag = "Admin",
    request_body = UpdateSystemSettingsRequest,
    responses(
        (status = 200, description = "Update system settings", body = SystemSettingsDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_settings(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
    Json(req): Json<UpdateSystemSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = state
        .admin_service
        .update_system_settings(&ctx, req)
        .await?;
    Ok((StatusCode::OK, Json(settings)))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/reports",
    tag = "Admin",
    responses(
        (status = 200, description = "Get operational metrics", body = OperationalReportDto)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_reports(
    State(state): State<AppState>,
    Extension(ctx): Extension<TenantContext>,
) -> Result<impl IntoResponse, ApiError> {
    let report = state.admin_service.get_operational_report(&ctx).await?;
    Ok(Json(report))
}
