use crate::state_machine::OrderStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{BranchId, CustomerId, OrderId, ProductId, TenantId, UserId};
use shifa_core::money::Money;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderDto {
    pub id: OrderId,
    pub tenant_id: TenantId,
    pub order_no: String,
    pub customer_id: CustomerId,
    pub branch_id: Option<BranchId>,
    pub status: OrderStatus,
    pub is_rx_linked: bool,
    pub subtotal: Money,
    pub discount: Money,
    pub delivery_fee: Money,
    pub tax_amount: Money,
    pub total: Money,
    pub payment_method: String,
    pub payment_status: String,
    pub items: Vec<OrderItemDto>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderItemDto {
    pub id: Uuid,
    pub product_id: ProductId,
    pub product_name: String,
    pub qty: i32,
    pub unit_price: Money,
    pub mrp_at_sale: Money,
    pub line_discount: Money,
    pub line_total: Money,
    pub is_prescription_only: bool,
    pub is_refrigerated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDraftOrderRequest {
    pub customer_id: CustomerId,
    pub branch_id: Option<BranchId>,
    pub payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddOrderItemRequest {
    pub product_id: ProductId,
    pub qty: i32,
    pub unit_price: Option<Money>,
    pub discount: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransitionOrderRequest {
    pub to_status: OrderStatus,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReturnItemRequest {
    pub item_id: Uuid,
    pub qty: i32,
    pub is_safe_to_restock: bool,
    pub pharmacist_certified: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderEventDto {
    pub id: Uuid,
    pub order_id: OrderId,
    pub from_status: Option<String>,
    pub to_status: String,
    pub actor_id: Option<UserId>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}
