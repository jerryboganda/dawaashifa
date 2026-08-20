use crate::error::ApiError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use shifa_conversation::models::*;
use shifa_conversation::override_engine::{bulk_approve_drafts, override_message};
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, ConversationId, MessageId, TenantId};

#[derive(Debug, Deserialize)]
pub struct ConversationQueryParams {
    pub branch_id: Option<uuid::Uuid>,
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/conversations",
    params(
        ("branch_id" = Option<uuid::Uuid>, Query, description = "Filter by branch ID"),
        ("status" = Option<String>, Query, description = "Filter by status")
    ),
    responses(
        (status = 200, description = "List of conversations", body = Vec<ConversationDto>)
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn list_conversations(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<ConversationQueryParams>,
) -> Result<Json<Vec<ConversationDto>>, ApiError> {
    let list = state
        .conversation_service
        .list_conversations(
            &ctx,
            params.branch_id.map(BranchId::from),
            params.status.as_deref(),
        )
        .await?;
    Ok(Json(list))
}

#[utoipa::path(
    post,
    path = "/api/v1/conversations/inbound",
    request_body = InboundMessageRequest,
    responses(
        (status = 200, description = "Processed inbound message", body = ConversationDto)
    ),
    tag = "Conversations"
)]
pub async fn inbound_message(
    State(state): State<AppState>,
    Json(req): Json<InboundMessageRequest>,
) -> Result<Json<ConversationDto>, ApiError> {
    // For test / webhook simulation, tenant is pulled from first active tenant
    let tenant_row = sqlx::query("SELECT id FROM tenants WHERE status = 'ACTIVE' LIMIT 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::BadRequest(format!("No active tenant found: {}", e)))?;
    let tenant_id = TenantId::from(sqlx::Row::get::<uuid::Uuid, _>(&tenant_row, "id"));

    let conv = state
        .conversation_service
        .handle_inbound(tenant_id, req)
        .await?;
    Ok(Json(conv))
}

#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/messages",
    params(
        ("id" = uuid::Uuid, Path, description = "Conversation ID")
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Outbound message sent", body = MessageDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn send_message(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageDto>), ApiError> {
    let msg = state
        .conversation_service
        .send_outbound(&ctx, ConversationId::from(id), req)
        .await?;
    Ok((StatusCode::CREATED, Json(msg)))
}

#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/claim",
    params(
        ("id" = uuid::Uuid, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Conversation claimed successfully")
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn claim_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .conversation_service
        .claim(&ctx, ConversationId::from(id))
        .await?;
    Ok(Json(serde_json::json!({"status": "claimed"})))
}

#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/assign",
    params(
        ("id" = uuid::Uuid, Path, description = "Conversation ID")
    ),
    request_body = AssignConversationRequest,
    responses(
        (status = 200, description = "Conversation assigned")
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn assign_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<AssignConversationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .conversation_service
        .assign(&ctx, ConversationId::from(id), req.user_id)
        .await?;
    Ok(Json(serde_json::json!({"status": "assigned"})))
}

#[utoipa::path(
    post,
    path = "/api/v1/conversations/{id}/transfer",
    params(
        ("id" = uuid::Uuid, Path, description = "Conversation ID")
    ),
    request_body = TransferConversationRequest,
    responses(
        (status = 200, description = "Conversation transferred")
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn transfer_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<TransferConversationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .conversation_service
        .transfer(&ctx, ConversationId::from(id), req.branch_id)
        .await?;
    Ok(Json(serde_json::json!({"status": "transferred"})))
}

#[utoipa::path(
    patch,
    path = "/api/v1/messages/{id}",
    params(
        ("id" = uuid::Uuid, Path, description = "Message ID")
    ),
    request_body = OverrideMessageRequest,
    responses(
        (status = 200, description = "Draft message overridden", body = MessageDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn override_message_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<OverrideMessageRequest>,
) -> Result<Json<MessageDto>, ApiError> {
    let msg = override_message(&ctx, &state.pool, MessageId::from(id), &req.new_body).await?;
    Ok(Json(msg))
}

#[utoipa::path(
    post,
    path = "/api/v1/messages/bulk-approve/{conversation_id}",
    params(
        ("conversation_id" = uuid::Uuid, Path, description = "Conversation ID")
    ),
    responses(
        (status = 200, description = "Bulk approved non-Rx drafts")
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn bulk_approve_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(conversation_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count =
        bulk_approve_drafts(&ctx, &state.pool, ConversationId::from(conversation_id)).await?;
    Ok(Json(serde_json::json!({"approved_count": count})))
}

#[utoipa::path(
    post,
    path = "/api/v1/canned-replies",
    request_body = CreateCannedReplyRequest,
    responses(
        (status = 201, description = "Canned reply created", body = CannedReplyDto)
    ),
    security(("bearer_auth" = [])),
    tag = "Conversations"
)]
pub async fn create_canned_reply_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<CreateCannedReplyRequest>,
) -> Result<(StatusCode, Json<CannedReplyDto>), ApiError> {
    let reply = state
        .conversation_service
        .create_canned_reply(&ctx, req)
        .await?;
    Ok((StatusCode::CREATED, Json(reply)))
}
