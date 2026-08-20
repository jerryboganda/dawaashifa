use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{BranchId, RoleId, TenantId, UserId};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDto {
    pub id: UserId,
    pub tenant_id: TenantId,
    pub phone: String,
    pub email: Option<String>,
    pub full_name: String,
    pub status: String,
    pub locale: String,
    pub roles: Vec<String>,
    pub branch_ids: Vec<BranchId>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BranchDto {
    pub id: BranchId,
    pub tenant_id: TenantId,
    pub name: String,
    pub code: String,
    pub drap_licence_no: String,
    pub pharmacist_in_charge: String,
    pub address: String,
    pub city: String,
    pub is_hub: bool,
    pub cold_chain_capable: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleDto {
    pub id: RoleId,
    pub tenant_id: TenantId,
    pub name: String,
    pub is_system: bool,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub phone_or_email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub phone: String,
    pub email: Option<String>,
    pub full_name: String,
    pub password: String,
    pub role_names: Vec<String>,
    pub branch_ids: Vec<BranchId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub status: Option<String>,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBranchRequest {
    pub name: String,
    pub code: String,
    pub drap_licence_no: String,
    pub pharmacist_in_charge: String,
    pub address: String,
    pub city: String,
    pub longitude: f64,
    pub latitude: f64,
    pub is_hub: bool,
    pub cold_chain_capable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateBranchRequest {
    pub name: Option<String>,
    pub pharmacist_in_charge: Option<String>,
    pub address: Option<String>,
    pub is_hub: Option<bool>,
    pub cold_chain_capable: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignRolesRequest {
    pub role_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignBranchesRequest {
    pub branch_ids: Vec<BranchId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfileResponse {
    pub user: UserDto,
    pub permissions: Vec<String>,
}
