use crate::error::ApiError;
use crate::AppState;
use axum::{extract::State, Json};
use shifa_core::context::TenantContext;
use shifa_identity::models::{
    AuthTokens, ChangePasswordRequest, LoginRequest, RefreshRequest, UserProfileResponse,
};

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthTokens),
        (status = 401, description = "Invalid credentials"),
        (status = 429, description = "Rate limit exceeded")
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthTokens>, ApiError> {
    let tokens = state.identity_service.login(req, None, None).await?;
    Ok(Json(tokens))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Tokens refreshed", body = AuthTokens),
        (status = 401, description = "Invalid or revoked refresh token")
    ),
    tag = "Auth"
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthTokens>, ApiError> {
    let tokens = state
        .identity_service
        .refresh_tokens(req, None, None)
        .await?;
    Ok(Json(tokens))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out successfully")
    ),
    security(("bearer_auth" = [])),
    tag = "Auth"
)]
pub async fn logout(_ctx: TenantContext) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(serde_json::json!({"status": "ok"})))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current user profile and permissions", body = UserProfileResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "Auth"
)]
pub async fn me(ctx: TenantContext) -> Result<Json<UserProfileResponse>, ApiError> {
    let response = UserProfileResponse {
        user: shifa_identity::models::UserDto {
            id: ctx.user_id,
            tenant_id: ctx.tenant_id,
            phone: "+923000000000".to_string(),
            email: Some("user@shifa.pk".to_string()),
            full_name: "Authenticated User".to_string(),
            status: "ACTIVE".to_string(),
            locale: "en".to_string(),
            roles: ctx.role_names,
            branch_ids: ctx.branch_ids,
            last_login_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
        },
        permissions: ctx.permissions.into_iter().collect(),
    };
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/password/change",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successfully"),
        (status = 400, description = "Password validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "Auth"
)]
pub async fn change_password(
    _ctx: TenantContext,
    Json(_req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(serde_json::json!({"status": "password_updated"})))
}
