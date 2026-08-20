//! HTTP API service layer, authentication middleware, route handlers,
//! and OpenAPI specification emitter for the Shifa platform.

pub mod error;
pub mod extractor;
pub mod openapi;
pub mod routes;

pub use error::ApiError;
pub use openapi::ApiDoc;

use axum::{
    routing::{get, patch, post},
    Router,
};
use shifa_catalog::CatalogService;
use shifa_conversation::ConversationService;
use shifa_identity::IdentityService;
use shifa_inventory::{ColdChainService, InventoryService, TransferService};
use shifa_orders::OrderService;
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub identity_service: IdentityService,
    pub catalog_service: CatalogService,
    pub inventory_service: InventoryService,
    pub transfer_service: TransferService,
    pub cold_chain_service: ColdChainService,
    pub conversation_service: ConversationService,
    pub order_service: OrderService,
}

pub fn build_app(pool: PgPool, identity_service: IdentityService) -> Router {
    let catalog_service = CatalogService::new(pool.clone());
    let inventory_service = InventoryService::new(pool.clone());
    let transfer_service = TransferService::new(pool.clone());
    let cold_chain_service = ColdChainService::new(pool.clone());
    let conversation_service = ConversationService::new(pool.clone());
    let order_service = OrderService::new(pool.clone());

    let state = AppState {
        pool,
        identity_service,
        catalog_service,
        inventory_service,
        transfer_service,
        cold_chain_service,
        conversation_service,
        order_service,
    };

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
            patch(routes::users::update_user).delete(routes::users::delete_user),
        )
        .route("/:id/roles", post(routes::users::assign_roles))
        .route("/:id/branches", post(routes::users::assign_branches));

    let branch_routes = Router::new()
        .route(
            "/",
            get(routes::branches::list_branches).post(routes::branches::create_branch),
        )
        .route("/:id", patch(routes::branches::update_branch));

    let role_routes = Router::new()
        .route("/roles", get(routes::roles::list_roles))
        .route("/permissions", get(routes::roles::list_permissions));

    let product_routes = Router::new()
        .route(
            "/",
            get(routes::products::list_products).post(routes::products::create_product),
        )
        .route("/:id", get(routes::products::get_product))
        .route("/match", post(routes::products::match_products_handler))
        .route("/:id/substitutes", get(routes::products::get_substitutes));

    let inventory_routes = Router::new()
        .route("/stock", get(routes::inventory::list_stock))
        .route("/receipts", post(routes::inventory::receive_stock))
        .route("/adjustments", post(routes::inventory::adjust_stock))
        .route("/transfers", post(routes::inventory::create_transfer))
        .route(
            "/transfers/:id/dispatch",
            post(routes::inventory::dispatch_transfer),
        )
        .route("/cold-chain/logs", post(routes::inventory::log_cold_chain))
        .route(
            "/cold-chain/:batch_id/clear-excursion",
            post(routes::inventory::clear_excursion),
        );

    let conversation_routes = Router::new()
        .route("/", get(routes::conversations::list_conversations))
        .route("/inbound", post(routes::conversations::inbound_message))
        .route("/:id/messages", post(routes::conversations::send_message))
        .route("/:id/claim", post(routes::conversations::claim_handler))
        .route("/:id/assign", post(routes::conversations::assign_handler))
        .route(
            "/:id/transfer",
            post(routes::conversations::transfer_handler),
        );

    let message_routes = Router::new()
        .route(
            "/:id",
            patch(routes::conversations::override_message_handler),
        )
        .route(
            "/bulk-approve/:conversation_id",
            post(routes::conversations::bulk_approve_handler),
        );

    let canned_reply_routes = Router::new().route(
        "/",
        post(routes::conversations::create_canned_reply_handler),
    );

    let order_routes = Router::new()
        .route(
            "/",
            get(routes::orders::list_orders).post(routes::orders::create_order),
        )
        .route("/:id", get(routes::orders::get_order))
        .route("/:id/items", post(routes::orders::add_item))
        .route("/:id/confirm-cart", post(routes::orders::confirm_cart))
        .route("/:id/transition", post(routes::orders::transition_order));

    let webhook_routes = Router::new().route(
        "/whatsapp/:channel_id",
        get(routes::webhooks::verify_webhook_challenge)
            .post(routes::webhooks::handle_inbound_webhook),
    );

    let api_v1 = Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/branches", branch_routes)
        .nest("/products", product_routes)
        .nest("/inventory", inventory_routes)
        .nest("/conversations", conversation_routes)
        .nest("/messages", message_routes)
        .nest("/canned-replies", canned_reply_routes)
        .nest("/orders", order_routes)
        .merge(role_routes);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api_v1)
        .nest("/webhooks", webhook_routes)
        .with_state(state)
}
