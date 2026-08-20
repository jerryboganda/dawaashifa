use crate::error::ApiError;
use crate::AppState;
use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use shifa_core::context::TenantContext;

/// Axum Extractor for TenantContext extracted strictly from verified JWT bearer claims.
/// Invariant: tenant_id comes only from JWT claims. Never from body, query, or path.
#[async_trait]
impl FromRequestParts<AppState> for TenantContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::Unauthorized(
                "Invalid Authorization header format, expected 'Bearer <token>'".to_string(),
            ));
        }

        let token = &auth_header[7..];

        let ctx = state
            .identity_service
            .extract_tenant_context(token)
            .await
            .map_err(|e| match e {
                shifa_identity::AuthError::SessionRevoked
                | shifa_identity::AuthError::SessionFamilyRevoked
                | shifa_identity::AuthError::InvalidToken => {
                    ApiError::Unauthorized("Invalid or expired session".to_string())
                }
                other => ApiError::from(other),
            })?;

        Ok(ctx)
    }
}
