use crate::error::ApiError;
use crate::AppState;
use axum::{extract::State, http::StatusCode, Json};
use shifa_ai::models::*;
use shifa_core::context::TenantContext;

#[utoipa::path(
    post,
    path = "/api/v1/ai/analyse",
    request_body = AiAnalyseRequest,
    responses(
        (status = 200, description = "Message analysis results", body = AnalysisResult)
    ),
    security(("bearer_auth" = [])),
    tag = "AI"
)]
pub async fn analyse_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<AiAnalyseRequest>,
) -> Result<Json<AnalysisResult>, ApiError> {
    let result = state.ai_service.analyse_message(&ctx, req).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/ai/draft-reply",
    request_body = AiDraftReplyRequest,
    responses(
        (status = 200, description = "Draft reply generated", body = DraftReplyResult)
    ),
    security(("bearer_auth" = [])),
    tag = "AI"
)]
pub async fn draft_reply_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<AiDraftReplyRequest>,
) -> Result<Json<DraftReplyResult>, ApiError> {
    let result = state.ai_service.draft_reply(&ctx, req).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/ai/transcribe",
    request_body = AiTranscribeRequest,
    responses(
        (status = 200, description = "Audio transcribed", body = TranscriptionResult)
    ),
    security(("bearer_auth" = [])),
    tag = "AI"
)]
pub async fn transcribe_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<AiTranscribeRequest>,
) -> Result<Json<TranscriptionResult>, ApiError> {
    let result = state.ai_service.transcribe_voice_note(&ctx, req).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/ai/feedback",
    request_body = FeedbackEventRequest,
    responses(
        (status = 200, description = "Feedback recorded", body = ())
    ),
    security(("bearer_auth" = [])),
    tag = "AI"
)]
pub async fn feedback_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<FeedbackEventRequest>,
) -> Result<StatusCode, ApiError> {
    state.ai_service.record_feedback(&ctx, req).await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/api/v1/ai/health",
    responses(
        (status = 200, description = "Circuit breaker status", body = Vec<AiHealthStatus>)
    ),
    security(("bearer_auth" = [])),
    tag = "AI"
)]
pub async fn health_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<AiHealthStatus>>, ApiError> {
    let health = state.ai_service.get_health().await;
    Ok(Json(health))
}
