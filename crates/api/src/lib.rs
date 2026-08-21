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
use shifa_ai::AiService;
use shifa_catalog::CatalogService;
use shifa_conversation::ConversationService;
use shifa_fulfilment::FulfilmentService;
use shifa_identity::IdentityService;
use shifa_inventory::{ColdChainService, InventoryService, TransferService};
use shifa_orders::OrderService;
use shifa_payments::PaymentService;
use shifa_prescription::PrescriptionService;
use shifa_tax::TaxService;
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
    pub ai_service: AiService,
    pub prescription_service: PrescriptionService,
    pub payment_service: PaymentService,
    pub fulfilment_service: FulfilmentService,
    pub tax_service: TaxService,
    pub b2b_service: shifa_b2b::B2bService,
}

pub fn build_app(pool: PgPool, identity_service: IdentityService) -> Router {
    let catalog_service = CatalogService::new(pool.clone());
    let inventory_service = InventoryService::new(pool.clone());
    let transfer_service = TransferService::new(pool.clone());
    let cold_chain_service = ColdChainService::new(pool.clone());
    let conversation_service = ConversationService::new(pool.clone());
    let order_service = OrderService::new(pool.clone());
    let ai_service = AiService::new(pool.clone());
    let prescription_service = PrescriptionService::new(pool.clone());
    let payment_service = PaymentService::new(pool.clone());
    let fulfilment_service = FulfilmentService::new(pool.clone());
    let tax_service = TaxService::new(pool.clone());
    let b2b_service = shifa_b2b::B2bService::new(pool.clone());

    let state = AppState {
        pool,
        identity_service,
        catalog_service,
        inventory_service,
        transfer_service,
        cold_chain_service,
        conversation_service,
        order_service,
        ai_service,
        prescription_service,
        payment_service,
        fulfilment_service,
        tax_service,
        b2b_service,
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

    let prescription_routes = Router::new()
        .route(
            "/",
            get(routes::prescriptions::list_prescriptions)
                .post(routes::prescriptions::create_prescription),
        )
        .route("/queue/stats", get(routes::prescriptions::get_queue_stats))
        .route("/:id", get(routes::prescriptions::get_prescription))
        .route(
            "/:id/extract",
            post(routes::prescriptions::extract_prescription),
        )
        .route(
            "/:id/claim",
            post(routes::prescriptions::claim_prescription),
        )
        .route(
            "/:id/approve",
            post(routes::prescriptions::approve_prescription),
        )
        .route(
            "/:id/reject",
            post(routes::prescriptions::reject_prescription),
        )
        .route(
            "/:id/clarify",
            post(routes::prescriptions::clarify_prescription),
        )
        .route("/:id/audit", get(routes::prescriptions::get_audit_trail));

    let ai_routes = Router::new()
        .route("/analyse", post(routes::ai::analyse_handler))
        .route("/draft-reply", post(routes::ai::draft_reply_handler))
        .route("/transcribe", post(routes::ai::transcribe_handler))
        .route("/feedback", post(routes::ai::feedback_handler))
        .route("/health", get(routes::ai::health_handler));

    let payment_routes = Router::new()
        .route("/intent", post(routes::payments::create_payment_intent))
        .route(
            "/webhooks/:gateway",
            post(routes::payments::handle_gateway_webhook),
        )
        .route("/proofs", post(routes::payments::create_payment_proof))
        .route("/proofs/queue", get(routes::payments::list_proofs_queue))
        .route("/proofs/:id", get(routes::payments::get_payment_proof))
        .route(
            "/proofs/:id/approve",
            post(routes::payments::approve_payment_proof),
        )
        .route(
            "/proofs/:id/reject",
            post(routes::payments::reject_payment_proof),
        )
        .route("/:id/refund", post(routes::payments::refund_payment))
        .route("/", get(routes::payments::list_payments))
        .route(
            "/reconciliation",
            get(routes::payments::get_reconciliation_report),
        );

    let fulfilment_routes = Router::new()
        .route(
            "/picking-lists",
            get(routes::fulfilment::list_picking_lists),
        )
        .route(
            "/picking-lists/:id/complete",
            post(routes::fulfilment::complete_picking_list),
        );

    let rider_routes = Router::new()
        .route(
            "/",
            get(routes::fulfilment::list_riders).post(routes::fulfilment::create_rider),
        )
        .route("/:id/shift/start", post(routes::fulfilment::start_shift))
        .route("/:id/shift/end", post(routes::fulfilment::end_shift));

    let delivery_routes = Router::new()
        .route("/", get(routes::fulfilment::list_deliveries))
        .route("/:id/assign", post(routes::fulfilment::assign_delivery))
        .route("/:id/accept", post(routes::fulfilment::accept_delivery))
        .route("/:id/decline", post(routes::fulfilment::decline_delivery))
        .route("/:id/pickup", post(routes::fulfilment::pickup_delivery))
        .route("/:id/deliver", post(routes::fulfilment::complete_delivery))
        .route("/:id/fail", post(routes::fulfilment::fail_delivery));

    let cash_session_routes = Router::new()
        .route("/", get(routes::fulfilment::list_cash_sessions))
        .route("/:id/declare", post(routes::fulfilment::declare_cash))
        .route(
            "/:id/reconcile",
            post(routes::fulfilment::reconcile_cash_session),
        )
        .route(
            "/variance-report",
            get(routes::fulfilment::get_variance_report),
        );

    let invoice_routes = Router::new()
        .route("/", get(routes::tax::list_invoices))
        .route("/:id", get(routes::tax::get_invoice))
        .route("/:id/pdf", get(routes::tax::get_invoice_pdf))
        .route("/:id/resubmit", post(routes::tax::resubmit_invoice))
        .route("/:id/credit-note", post(routes::tax::create_credit_note));

    let tax_routes = Router::new()
        .route(
            "/categories",
            get(routes::tax::list_tax_categories).post(routes::tax::create_tax_category),
        )
        .route("/categories/:id", patch(routes::tax::patch_tax_category))
        .route("/report", get(routes::tax::get_tax_report));

    let fbr_routes = Router::new().route("/queue-status", get(routes::tax::get_fbr_queue_status));

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
        .nest("/prescriptions", prescription_routes)
        .nest("/payments", payment_routes)
        .nest("/fulfilment", fulfilment_routes)
        .nest("/riders", rider_routes)
        .nest("/deliveries", delivery_routes)
        .nest("/cash-sessions", cash_session_routes)
        .nest("/invoices", invoice_routes)
        .nest("/tax", tax_routes)
        .nest("/fbr", fbr_routes)
        .nest("/b2b", routes::b2b::b2b_routes())
        .nest("/ai", ai_routes)
        .route("/health", get(routes::health::system_health_handler))
        .merge(role_routes);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(routes::health::system_health_handler))
        .nest("/api/v1", api_v1)
        .route(
            "/api/v1/track/:token",
            get(routes::fulfilment::get_public_tracking),
        )
        .nest("/webhooks", webhook_routes)
        .with_state(state)
}
