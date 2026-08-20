use crate::error::ApiError;
use crate::AppState;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;
use shifa_core::context::TenantContext;
use shifa_core::id::{OrderId, PaymentId, ProofId};
use shifa_payments::models::*;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ListProofsQueueQuery {
    pub severity: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListPaymentsQuery {
    pub order_id: Option<Uuid>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ReconciliationQuery {
    pub date: Option<String>,
    pub gateway: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/intent",
    request_body = IntentRequest,
    responses(
        (status = 200, description = "Payment intent created", body = PaymentIntent),
        (status = 400, description = "Invalid request or COD limit exceeded")
    ),
    tag = "Payments"
)]
pub async fn create_payment_intent(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<IntentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let intent = state.payment_service.create_intent(&ctx, req).await?;
    Ok(Json(intent))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/webhooks/{gateway}",
    params(
        ("gateway" = String, Path, description = "Gateway identifier (e.g. jazzcash, easypaisa, raast, safepay)")
    ),
    responses(
        (status = 200, description = "Webhook verified and payment confirmed", body = PaymentDto),
        (status = 400, description = "Invalid signature, replay, or amount mismatch")
    ),
    tag = "Payments"
)]
pub async fn handle_gateway_webhook(
    State(state): State<AppState>,
    Path(gateway): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let tenant_header = headers
        .get("x-tenant-id")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<Uuid>().ok())
        .unwrap_or_default();

    let ctx = TenantContext::from_authenticated_session(
        shifa_core::id::TenantId::from(tenant_header),
        shifa_core::id::UserId::new(),
        vec![],
        std::collections::HashSet::new(),
        vec!["SYSTEM".to_string()],
    );
    let payment = state
        .payment_service
        .handle_webhook(&ctx, &gateway, &headers, &body)
        .await?;

    Ok(Json(payment))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/proofs",
    request_body = CreateProofRequest,
    responses(
        (status = 200, description = "Screenshot proof submitted and queued for review", body = PaymentProofDto),
        (status = 400, description = "Invalid request")
    ),
    tag = "Payments"
)]
pub async fn create_payment_proof(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<CreateProofRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let proof = state.payment_service.create_proof(&ctx, req).await?;
    Ok(Json(proof))
}

#[utoipa::path(
    get,
    path = "/api/v1/payments/proofs/queue",
    params(
        ("severity" = Option<String>, Query, description = "Filter by fraud severity (CRITICAL, HIGH, MEDIUM, LOW)"),
        ("limit" = Option<i64>, Query, description = "Limit (default 50)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List of pending payment proofs in review queue", body = Vec<PaymentProofDto>)
    ),
    tag = "Payments"
)]
pub async fn list_proofs_queue(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(query): Query<ListProofsQueueQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let list = state
        .payment_service
        .list_proofs_queue(&ctx, query.severity, limit, offset)
        .await?;
    Ok(Json(list))
}

#[utoipa::path(
    get,
    path = "/api/v1/payments/proofs/{id}",
    params(
        ("id" = Uuid, Path, description = "Proof ID")
    ),
    responses(
        (status = 200, description = "Payment proof details with fraud flags and order", body = PaymentProofDto),
        (status = 404, description = "Proof not found")
    ),
    tag = "Payments"
)]
pub async fn get_payment_proof(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let proof = state
        .payment_service
        .get_proof(&ctx, ProofId::from(id))
        .await?;
    Ok(Json(proof))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/proofs/{id}/approve",
    params(
        ("id" = Uuid, Path, description = "Proof ID")
    ),
    request_body = ApproveProofRequest,
    responses(
        (status = 200, description = "Payment proof approved, TID recorded in ledger, payment and order confirmed", body = PaymentProofDto),
        (status = 403, description = "Missing payment.approve permission")
    ),
    tag = "Payments"
)]
pub async fn approve_payment_proof(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(req): Json<ApproveProofRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let proof = state
        .payment_service
        .approve_proof(&ctx, ProofId::from(id), req)
        .await?;
    Ok(Json(proof))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/proofs/{id}/reject",
    params(
        ("id" = Uuid, Path, description = "Proof ID")
    ),
    request_body = RejectProofRequest,
    responses(
        (status = 200, description = "Payment proof rejected with reason", body = PaymentProofDto),
        (status = 403, description = "Missing payment.reject permission")
    ),
    tag = "Payments"
)]
pub async fn reject_payment_proof(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(req): Json<RejectProofRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let proof = state
        .payment_service
        .reject_proof(&ctx, ProofId::from(id), req)
        .await?;
    Ok(Json(proof))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/{id}/refund",
    params(
        ("id" = Uuid, Path, description = "Payment ID")
    ),
    request_body = RefundRequest,
    responses(
        (status = 200, description = "Payment refunded", body = PaymentDto),
        (status = 403, description = "Missing payment.refund permission")
    ),
    tag = "Payments"
)]
pub async fn refund_payment(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<Uuid>,
    Json(req): Json<RefundRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let payment = state
        .payment_service
        .refund_payment(&ctx, PaymentId::from(id), req)
        .await?;
    Ok(Json(payment))
}

#[utoipa::path(
    get,
    path = "/api/v1/payments",
    params(
        ("order_id" = Option<Uuid>, Query, description = "Filter by order ID"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("limit" = Option<i64>, Query, description = "Limit (default 50)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses(
        (status = 200, description = "List of payments", body = Vec<PaymentDto>)
    ),
    tag = "Payments"
)]
pub async fn list_payments(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(query): Query<ListPaymentsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let status_filter = query.status.and_then(|s| s.parse().ok());
    let list = state
        .payment_service
        .list_payments(
            &ctx,
            query.order_id.map(OrderId::from),
            status_filter,
            limit,
            offset,
        )
        .await?;
    Ok(Json(list))
}

#[utoipa::path(
    get,
    path = "/api/v1/payments/reconciliation",
    params(
        ("date" = Option<String>, Query, description = "Date YYYY-MM-DD (default today)"),
        ("gateway" = Option<String>, Query, description = "Gateway name (default JAZZCASH)")
    ),
    responses(
        (status = 200, description = "Settlement reconciliation report with unmatched discrepancies", body = ReconciliationReportDto),
        (status = 403, description = "Missing report.view permission")
    ),
    tag = "Payments"
)]
pub async fn get_reconciliation_report(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(query): Query<ReconciliationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let report_date = query
        .date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let gateway = query.gateway.unwrap_or_else(|| "JAZZCASH".into());

    let report = state
        .payment_service
        .generate_reconciliation_report(&ctx, report_date, &gateway, vec![])
        .await?;

    Ok(Json(report))
}
