use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::routes::*;
use shifa_catalog::models::*;
use shifa_core::id::*;
use shifa_core::money::Money;
use shifa_identity::models::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login,
        auth::refresh,
        auth::logout,
        auth::me,
        auth::change_password,
        users::list_users,
        users::create_user,
        users::update_user,
        users::assign_roles,
        users::assign_branches,
        users::delete_user,
        branches::list_branches,
        branches::create_branch,
        branches::update_branch,
        roles::list_roles,
        roles::list_permissions,
        products::list_products,
        products::get_product,
        products::create_product,
        products::match_products_handler,
        products::get_substitutes,
        webhooks::verify_webhook_challenge,
        webhooks::handle_inbound_webhook,
    ),
    components(
        schemas(
            TenantId,
            BranchId,
            UserId,
            RoleId,
            ProductId,
            CategoryId,
            GenericId,
            Money,
            AuthTokens,
            LoginRequest,
            RefreshRequest,
            ChangePasswordRequest,
            CreateUserRequest,
            UpdateUserRequest,
            AssignRolesRequest,
            AssignBranchesRequest,
            CreateBranchRequest,
            UpdateBranchRequest,
            UserDto,
            BranchDto,
            RoleDto,
            UserProfileResponse,
            ProductDto,
            CreateProductRequest,
            UpdateProductRequest,
            MatchRequest,
            MatchCandidate,
            MatchMethod,
            SubstitutionCandidate,
            ProductAliasDto,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication and session management"),
        (name = "Users", description = "User management and RBAC assignments"),
        (name = "Branches", description = "Branch store locations and configuration"),
        (name = "Roles", description = "Roles and permissions directory"),
        (name = "Products", description = "Drug master, catalog, MRP enforcement, and matching"),
        (name = "Webhooks", description = "WhatsApp Meta Cloud API webhooks")
    ),
    info(
        title = "Shifa Platform API",
        version = "0.1.0",
        description = "High-performance modular API for Shifa WhatsApp pharmacy commerce platform in Pakistan."
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
