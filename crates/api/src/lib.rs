//! HTTP API service layer, authentication middleware, route handlers,
//! and OpenAPI specification emitter for the Shifa platform.

pub mod error;
pub mod extractor;
pub mod openapi;
pub mod routes;

pub use error::ApiError;
pub use openapi::ApiDoc;

use axum::{
    routing::{get, post},
    Router,
};
use shifa_identity::IdentityService;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub identity_service: IdentityService,
}

pub fn build_app(identity_service: IdentityService) -> Router {
    let state = AppState { identity_service };

    let auth_routes = Router::new()
        .route("/login", post(routes::auth::login))
        .route("/refresh", post(routes::auth::refresh))
        .route("/logout", post(routes::auth::logout))
        .route("/me", get(routes::auth::me))
        .route("/password/change", post(routes::auth::change_password));

    let user_routes = Router::new()
        .route(
            "/",
            get(routes::users::list_users).post(routes::users::create_user),
        )
        .route(
            "/:id",
            axum::routing::patch(routes::users::update_user).delete(routes::users::delete_user),
        )
        .route("/:id/roles", post(routes::users::assign_roles))
        .route("/:id/branches", post(routes::users::assign_branches));

    let branch_routes = Router::new()
        .route(
            "/",
            get(routes::branches::list_branches).post(routes::branches::create_branch),
        )
        .route(
            "/:id",
            axum::routing::patch(routes::branches::update_branch),
        );

    let role_routes = Router::new()
        .route("/roles", get(routes::roles::list_roles))
        .route("/permissions", get(routes::roles::list_permissions));

    let webhook_routes = Router::new().route(
        "/whatsapp/:channel_id",
        get(routes::webhooks::verify_webhook_challenge)
            .post(routes::webhooks::handle_inbound_webhook),
    );

    let api_v1 = Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/branches", branch_routes)
        .merge(role_routes);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api_v1)
        .nest("/webhooks", webhook_routes)
        .with_state(state)
}
