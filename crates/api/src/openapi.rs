use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::routes::*;
use shifa_ai::models::*;
use shifa_catalog::models::*;
use shifa_conversation::models::*;
use shifa_core::id::*;
use shifa_core::money::Money;
use shifa_identity::models::*;
use shifa_inventory::models::*;
use shifa_orders::models::*;
use shifa_orders::state_machine::OrderStatus;

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
        inventory::list_stock,
        inventory::receive_stock,
        inventory::adjust_stock,
        inventory::create_transfer,
        inventory::dispatch_transfer,
        inventory::log_cold_chain,
        inventory::clear_excursion,
        conversations::list_conversations,
        conversations::inbound_message,
        conversations::send_message,
        conversations::claim_handler,
        conversations::assign_handler,
        conversations::transfer_handler,
        conversations::override_message_handler,
        conversations::bulk_approve_handler,
        conversations::create_canned_reply_handler,
        orders::list_orders,
        orders::create_order,
        orders::get_order,
        orders::add_item,
        orders::confirm_cart,
        orders::transition_order,
        ai::analyse_handler,
        ai::draft_reply_handler,
        ai::transcribe_handler,
        ai::feedback_handler,
        ai::health_handler,
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
            BatchId,
            ConversationId,
            CustomerId,
            MessageId,
            OrderId,
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
            StockCurrentDto,
            BatchAllocation,
            StockReceiptRequest,
            StockAdjustmentRequest,
            CreateTransferRequest,
            TransferItemRequest,
            TransferDto,
            ColdChainLogRequest,
            ClearExcursionRequest,
            BranchAvailabilityDto,
            ConversationDto,
            MessageDto,
            InboundMessageRequest,
            SendMessageRequest,
            OverrideMessageRequest,
            CannedReplyDto,
            CreateCannedReplyRequest,
            AssignConversationRequest,
            TransferConversationRequest,
            OrderStatus,
            OrderDto,
            OrderItemDto,
            CreateDraftOrderRequest,
            AddOrderItemRequest,
            TransitionOrderRequest,
            ReturnItemRequest,
            OrderEventDto,
            AiTask,
            CustomerScript,
            IntentType,
            ExtractedEntity,
            AnalysisResult,
            DraftReplyResult,
            TranscriptionResult,
            AiAnalyseRequest,
            AiDraftReplyRequest,
            AiTranscribeRequest,
            FeedbackEventRequest,
            AiHealthStatus,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication and session management"),
        (name = "Users", description = "User management and RBAC assignments"),
        (name = "Branches", description = "Branch store locations and configuration"),
        (name = "Roles", description = "Roles and permissions directory"),
        (name = "Products", description = "Drug master, catalog, MRP enforcement, and matching"),
        (name = "Inventory", description = "Append-only stock ledger, batches, transfers, and cold chain"),
        (name = "Conversations", description = "WhatsApp conversations, routing, human override, and inbox"),
        (name = "Orders", description = "Order state machine, cart, branch routing, and COD"),
        (name = "AI", description = "AI gateway, intent classification, voice transcription, and confidence gating"),
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
