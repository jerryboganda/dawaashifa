use crate::error::ApiError;
use axum::{
    body::Bytes,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use shifa_channel::webhook::{parse_inbound_webhook, verify_hub_signature};
use shifa_core::id::{ChannelId, TenantId};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct WebhookChallengeParams {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

#[utoipa::path(
    get,
    path = "/webhooks/whatsapp/{channel_id}",
    params(
        ("channel_id" = uuid::Uuid, Path, description = "Channel ID"),
        ("hub.mode" = Option<String>, Query, description = "Hub mode"),
        ("hub.verify_token" = Option<String>, Query, description = "Hub verify token"),
        ("hub.challenge" = Option<String>, Query, description = "Hub challenge")
    ),
    responses(
        (status = 200, description = "Webhook challenge verified", body = String),
        (status = 403, description = "Verification token mismatch")
    ),
    tag = "Webhooks"
)]
pub async fn verify_webhook_challenge(
    Path(_channel_id): Path<uuid::Uuid>,
    Query(params): Query<WebhookChallengeParams>,
) -> Result<Response, ApiError> {
    let expected_token =
        std::env::var("WA_VERIFY_TOKEN").unwrap_or_else(|_| "shifa_verify_token".to_string());

    if params.mode.as_deref() == Some("subscribe")
        && params.verify_token.as_deref() == Some(&expected_token)
    {
        if let Some(challenge) = params.challenge {
            return Ok((StatusCode::OK, challenge).into_response());
        }
    }

    Err(ApiError::Forbidden("Verify token mismatch".to_string()))
}

#[utoipa::path(
    post,
    path = "/webhooks/whatsapp/{channel_id}",
    params(
        ("channel_id" = uuid::Uuid, Path, description = "Channel ID")
    ),
    responses(
        (status = 200, description = "Inbound webhook acknowledged"),
        (status = 403, description = "Invalid signature")
    ),
    tag = "Webhooks"
)]
pub async fn handle_inbound_webhook(
    Path(channel_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app_secret =
        std::env::var("WA_APP_SECRET").unwrap_or_else(|_| "shifa_app_secret".to_string());

    // 1. Verify X-Hub-Signature-256 HMAC-SHA256 against app secret
    // Invariant: Reject on mismatch, do NOT log payload body
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Forbidden("Missing webhook signature".to_string()))?;

    verify_hub_signature(&body, signature, &app_secret)
        .map_err(|_| ApiError::Forbidden("Invalid webhook signature".to_string()))?;

    // 2. Parse payload asynchronously (fast 200 OK acknowledgment)
    if let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) {
        let tenant_id = TenantId::new(); // In prod, resolved from channel_id lookup
        let messages = parse_inbound_webhook(&payload, tenant_id, ChannelId::from(channel_id));
        info!("Acknowledged {} inbound WhatsApp messages", messages.len());
    }

    Ok(Json(serde_json::json!({"status": "acknowledged"})))
}
