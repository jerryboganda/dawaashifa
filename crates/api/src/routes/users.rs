use crate::error::ApiError;
use crate::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use shifa_core::context::TenantContext;
use shifa_core::id::UserId;
use shifa_identity::models::{
    AssignBranchesRequest, AssignRolesRequest, CreateUserRequest, UpdateUserRequest, UserDto,
};

#[utoipa::path(
    get,
    path = "/api/v1/users",
    responses(
        (status = 200, description = "List users", body = Vec<UserDto>),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn list_users(ctx: TenantContext) -> Result<Json<Vec<UserDto>>, ApiError> {
    ctx.require("user.view")?;
    Ok(Json(vec![]))
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = UserDto),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn create_user(
    ctx: TenantContext,
    State(_state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<UserDto>, ApiError> {
    ctx.require("user.create")?;
    let new_user = UserDto {
        id: UserId::new(),
        tenant_id: ctx.tenant_id,
        phone: req.phone,
        email: req.email,
        full_name: req.full_name,
        status: "ACTIVE".to_string(),
        locale: "en".to_string(),
        roles: req.role_names,
        branch_ids: req.branch_ids,
        last_login_at: None,
        created_at: chrono::Utc::now(),
    };
    Ok(Json(new_user))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}",
    request_body = UpdateUserRequest,
    params(
        ("id" = uuid::Uuid, Path, description = "Target user ID")
    ),
    responses(
        (status = 200, description = "User updated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn update_user(
    ctx: TenantContext,
    Path(_id): Path<uuid::Uuid>,
    Json(_req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ctx.require("user.edit")?;
    Ok(Json(serde_json::json!({"status": "updated"})))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/roles",
    request_body = AssignRolesRequest,
    params(
        ("id" = uuid::Uuid, Path, description = "Target user ID")
    ),
    responses(
        (status = 200, description = "Roles assigned"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn assign_roles(
    ctx: TenantContext,
    Path(_id): Path<uuid::Uuid>,
    Json(_req): Json<AssignRolesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ctx.require("user.assign_role")?;
    Ok(Json(serde_json::json!({"status": "roles_assigned"})))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/branches",
    request_body = AssignBranchesRequest,
    params(
        ("id" = uuid::Uuid, Path, description = "Target user ID")
    ),
    responses(
        (status = 200, description = "Branches assigned"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn assign_branches(
    ctx: TenantContext,
    Path(_id): Path<uuid::Uuid>,
    Json(_req): Json<AssignBranchesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ctx.require("user.assign_role")?;
    Ok(Json(serde_json::json!({"status": "branches_assigned"})))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    params(
        ("id" = uuid::Uuid, Path, description = "Target user ID")
    ),
    responses(
        (status = 200, description = "User soft deleted"),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn delete_user(
    ctx: TenantContext,
    Path(_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ctx.require("user.edit")?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}
