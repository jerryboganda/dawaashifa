use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ------------------------------------------------------------------------------------------------
// Business Accounts & Contacts
// ------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BusinessAccountDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub account_type: String,
    pub ntn: Option<String>,
    pub strn: Option<String>,
    pub billing_address: String,
    pub shipping_addresses: serde_json::Value,
    pub credit_limit: String,
    pub payment_terms_days: i32,
    pub price_list_id: Option<Uuid>,
    pub status: String,
    pub on_hold: bool,
    pub hold_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub name: String,
    pub account_type: Option<String>,
    pub ntn: Option<String>,
    pub strn: Option<String>,
    pub billing_address: String,
    pub shipping_addresses: Option<serde_json::Value>,
    pub credit_limit: Option<String>,
    pub payment_terms_days: Option<i32>,
    pub price_list_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatchAccountRequest {
    pub name: Option<String>,
    pub billing_address: Option<String>,
    pub credit_limit: Option<String>,
    pub payment_terms_days: Option<i32>,
    pub price_list_id: Option<Uuid>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountHoldRequest {
    pub on_hold: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BusinessContactDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub designation: String,
    pub phone: String,
    pub email: Option<String>,
    pub can_approve_po: bool,
    pub approval_limit: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateContactRequest {
    pub name: String,
    pub designation: String,
    pub phone: String,
    pub email: Option<String>,
    pub can_approve_po: Option<bool>,
    pub approval_limit: Option<String>,
}

// ------------------------------------------------------------------------------------------------
// Quotations
// ------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotationItemDto {
    pub id: Uuid,
    pub quotation_id: Uuid,
    pub product_id: Uuid,
    pub qty: i32,
    pub unit_price: String,
    pub discount: String,
    pub line_total: String,
    pub lead_time_days: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotationDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub quote_no: String,
    pub version: i32,
    pub parent_quote_id: Option<Uuid>,
    pub status: String,
    pub valid_until: DateTime<Utc>,
    pub subtotal: String,
    pub discount: String,
    pub tax_amount: String,
    pub total: String,
    pub terms_text: Option<String>,
    pub prepared_by: Uuid,
    pub approved_by: Option<Uuid>,
    pub sent_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub items: Vec<QuotationItemDto>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotationItemRequest {
    pub product_id: Uuid,
    pub qty: i32,
    pub unit_price: String,
    pub discount: Option<String>,
    pub lead_time_days: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateQuotationRequest {
    pub account_id: Uuid,
    pub valid_until: DateTime<Utc>,
    pub terms_text: Option<String>,
    pub items: Vec<QuotationItemRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviseQuotationRequest {
    pub valid_until: DateTime<Utc>,
    pub terms_text: Option<String>,
    pub items: Vec<QuotationItemRequest>,
}

// ------------------------------------------------------------------------------------------------
// Purchase Orders
// ------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PurchaseOrderDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub quotation_id: Option<Uuid>,
    pub po_number: String,
    pub po_document_key: Option<String>,
    pub received_at: DateTime<Utc>,
    pub verified_by: Option<Uuid>,
    pub amount: String,
    pub variance_detected: bool,
    pub variance_notes: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePurchaseOrderRequest {
    pub account_id: Uuid,
    pub quotation_id: Option<Uuid>,
    pub po_number: String,
    pub po_document_key: Option<String>,
    pub amount: String,
}

// ------------------------------------------------------------------------------------------------
// Accounts Receivable (AR) & Aging
// ------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArAgingBucketDto {
    pub current: String,
    pub days_1_30: String,
    pub days_31_60: String,
    pub days_61_90: String,
    pub days_90_plus: String,
    pub total_outstanding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArSummaryDto {
    pub account_id: Uuid,
    pub account_name: String,
    pub credit_limit: String,
    pub available_credit: String,
    pub on_hold: bool,
    pub aging: ArAgingBucketDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArAgingReportDto {
    pub tenant_id: Uuid,
    pub accounts: Vec<ArSummaryDto>,
    pub aggregate_aging: ArAgingBucketDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AllocatePaymentRequest {
    pub amount: String,
    pub override_invoice_id: Option<Uuid>,
    pub reason: Option<String>,
}

// ------------------------------------------------------------------------------------------------
// Consignment Stock
// ------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsignmentLocationDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub address: String,
    pub managed_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsignmentStockDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub location_id: Uuid,
    pub product_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub serial_no: Option<String>,
    pub qty: i32,
    pub placed_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub invoiced_at: Option<DateTime<Utc>>,
    pub discrepancy_flagged: bool,
    pub discrepancy_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlaceConsignmentRequest {
    pub location_id: Uuid,
    pub product_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub serial_no: Option<String>,
    pub qty: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsumeConsignmentRequest {
    pub qty_consumed: i32,
    pub patient_ref: Option<String>,
    pub surgeon_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconcileConsignmentRequest {
    pub physical_count: i32,
    pub notes: Option<String>,
}

// ------------------------------------------------------------------------------------------------
// Device Traceability
// ------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeviceUnitDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub product_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub serial_no: String,
    pub udi: Option<String>,
    pub status: String,
    pub location_type: String,
    pub location_id: Option<Uuid>,
    pub implanted_at: Option<DateTime<Utc>>,
    pub patient_ref: Option<String>,
    pub surgeon_name: Option<String>,
    pub order_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterDeviceRequest {
    pub product_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub serial_no: String,
    pub udi: Option<String>,
    pub location_type: Option<String>,
    pub location_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecallQueryResponse {
    pub product_id: Option<Uuid>,
    pub batch_id: Option<Uuid>,
    pub affected_units_count: usize,
    pub units: Vec<DeviceUnitDto>,
}
