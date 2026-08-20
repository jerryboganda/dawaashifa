use crate::error::ApiError;
use crate::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use shifa_core::context::TenantContext;
use shifa_core::id::BranchId;
use shifa_identity::models::{BranchDto, CreateBranchRequest, UpdateBranchRequest};

#[utoipa::path(
    get,
    path = "/api/v1/branches",
    responses(
        (status = 200, description = "List branches", body = Vec<BranchDto>),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Branches"
)]
pub async fn list_branches(ctx: TenantContext) -> Result<Json<Vec<BranchDto>>, ApiError> {
    ctx.require("branch.view")?;
    Ok(Json(vec![]))
}

#[utoipa::path(
    post,
    path = "/api/v1/branches",
    request_body = CreateBranchRequest,
    responses(
        (status = 201, description = "Branch created", body = BranchDto),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Branches"
)]
pub async fn create_branch(
    ctx: TenantContext,
    State(_state): State<AppState>,
    Json(req): Json<CreateBranchRequest>,
) -> Result<Json<BranchDto>, ApiError> {
    ctx.require("branch.create")?;
    let new_branch = BranchDto {
        id: BranchId::new(),
        tenant_id: ctx.tenant_id(),
        name: req.name,
        code: req.code,
        drap_licence_no: req.drap_licence_no,
        pharmacist_in_charge: req.pharmacist_in_charge,
        address: req.address,
        city: req.city,
        is_hub: req.is_hub,
        cold_chain_capable: req.cold_chain_capable,
        status: "ACTIVE".to_string(),
    };
    Ok(Json(new_branch))
}

#[utoipa::path(
    patch,
    path = "/api/v1/branches/{id}",
    request_body = UpdateBranchRequest,
    params(
        ("id" = uuid::Uuid, Path, description = "Branch ID")
    ),
    responses(
        (status = 200, description = "Branch updated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Branch not found")
    ),
    security(("bearer_auth" = [])),
    tag = "Branches"
)]
pub async fn update_branch(
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
    Json(_req): Json<UpdateBranchRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ctx.require("branch.edit")?;
    ctx.require_branch(BranchId::from(id))?;
    Ok(Json(serde_json::json!({"status": "updated"})))
}
