use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{BatchId, BranchId, ProductId, TenantId};
use shifa_core::money::Money;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockCurrentDto {
    pub branch_id: BranchId,
    pub product_id: ProductId,
    pub batch_id: BatchId,
    pub qty: i32,
    pub batch_number: String,
    pub expiry_date: NaiveDate,
    pub is_quarantined: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchAllocation {
    pub batch_id: BatchId,
    pub batch_number: String,
    pub expiry_date: NaiveDate,
    pub qty: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockReceiptRequest {
    pub branch_id: BranchId,
    pub product_id: ProductId,
    pub batch_number: String,
    pub expiry_date: NaiveDate,
    pub qty: i32,
    pub supplier_id: Option<Uuid>,
    pub cost_price: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockAdjustmentRequest {
    pub branch_id: BranchId,
    pub product_id: ProductId,
    pub batch_id: BatchId,
    pub qty_delta: i32,
    pub reason: String, // Reason required for DRAP compliance
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTransferRequest {
    pub source_branch_id: BranchId,
    pub target_branch_id: BranchId,
    pub items: Vec<TransferItemRequest>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransferItemRequest {
    pub product_id: ProductId,
    pub batch_id: BatchId,
    pub qty: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransferDto {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub source_branch_id: BranchId,
    pub target_branch_id: BranchId,
    pub status: String, // DRAFT | DISPATCHED | IN_TRANSIT | RECEIVED | DISCREPANCY | CANCELLED
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ColdChainLogRequest {
    pub branch_id: BranchId,
    pub batch_id: BatchId,
    pub temperature_c: f64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClearExcursionRequest {
    pub decision_note: String, // Pharmacist documented decision required
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BranchAvailabilityDto {
    pub branch_id: BranchId,
    pub branch_name: String,
    pub can_fulfill_all: bool,
    pub total_available: i32,
}
