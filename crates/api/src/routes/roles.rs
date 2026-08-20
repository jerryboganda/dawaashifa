use crate::error::ApiError;
use axum::Json;
use shifa_core::context::TenantContext;
use shifa_identity::models::RoleDto;

#[utoipa::path(
    get,
    path = "/api/v1/roles",
    responses(
        (status = 200, description = "List roles", body = Vec<RoleDto>),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Roles"
)]
pub async fn list_roles(ctx: TenantContext) -> Result<Json<Vec<RoleDto>>, ApiError> {
    ctx.require("user.view")?;
    Ok(Json(vec![]))
}

#[utoipa::path(
    get,
    path = "/api/v1/permissions",
    responses(
        (status = 200, description = "List all permissions", body = Vec<String>),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Roles"
)]
pub async fn list_permissions(ctx: TenantContext) -> Result<Json<Vec<String>>, ApiError> {
    ctx.require("user.view")?;
    let perms: Vec<String> = shifa_identity::roles::ALL_PERMISSIONS
        .iter()
        .map(|&s| s.to_string())
        .collect();
    Ok(Json(perms))
}
