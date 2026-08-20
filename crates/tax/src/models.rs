use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shifa_core::id::{BranchId, InvoiceId, OrderId, TaxCategoryId, TenantId};
use shifa_core::money::Money;
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum FbrQueueStatus {
    Pending,
    Submitting,
    Accepted,
    Rejected,
    Failed,
}

impl fmt::Display for FbrQueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Submitting => write!(f, "SUBMITTING"),
            Self::Accepted => write!(f, "ACCEPTED"),
            Self::Rejected => write!(f, "REJECTED"),
            Self::Failed => write!(f, "FAILED"),
        }
    }
}

impl FromStr for FbrQueueStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PENDING" => Ok(Self::Pending),
            "SUBMITTING" => Ok(Self::Submitting),
            "ACCEPTED" | "TRANSMITTED" => Ok(Self::Accepted),
            "REJECTED" => Ok(Self::Rejected),
            "FAILED" => Ok(Self::Failed),
            other => Err(format!("Unknown FBR queue status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum InvoiceStatus {
    Issued,
    Cancelled,
    Refunded,
}

impl fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Issued => write!(f, "ISSUED"),
            Self::Cancelled => write!(f, "CANCELLED"),
            Self::Refunded => write!(f, "REFUNDED"),
        }
    }
}

impl FromStr for InvoiceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ISSUED" => Ok(Self::Issued),
            "CANCELLED" => Ok(Self::Cancelled),
            "REFUNDED" => Ok(Self::Refunded),
            other => Err(format!("Unknown invoice status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxCategoryDto {
    pub id: TaxCategoryId,
    pub tenant_id: TenantId,
    pub name: String,
    pub rate: Decimal,
    pub fbr_code: Option<String>,
    pub is_exempt: bool,
    pub is_zero_rated: bool,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaxCategoryRequest {
    pub name: String,
    pub rate: Decimal,
    pub fbr_code: Option<String>,
    pub is_exempt: Option<bool>,
    pub is_zero_rated: Option<bool>,
    pub effective_from: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatchTaxCategoryRequest {
    pub new_rate: Decimal,
    pub fbr_code: Option<String>,
    pub effective_from: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxLine {
    pub item_name: String,
    pub taxable_amount: Money,
    pub rate: Decimal,
    pub tax_amount: Money,
    pub category_id: TaxCategoryId,
    pub fbr_code: Option<String>,
    pub is_exempt: bool,
    pub is_zero_rated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxCalculationResult {
    pub subtotal: Money,
    pub tax_amount: Money,
    pub total_amount: Money,
    pub lines: Vec<TaxLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceDto {
    pub id: InvoiceId,
    pub tenant_id: TenantId,
    pub branch_id: BranchId,
    pub order_id: OrderId,
    pub invoice_no: String,
    pub fiscal_invoice_no: Option<String>,
    pub status: InvoiceStatus,
    pub subtotal: Money,
    pub tax_amount: Money,
    pub total_amount: Money,
    pub lines: Vec<TaxLine>,
    pub fbr_queue_status: FbrQueueStatus,
    pub fbr_request: Option<serde_json::Value>,
    pub fbr_response: Option<serde_json::Value>,
    pub fbr_qr_payload: Option<String>,
    pub fbr_error: Option<String>,
    pub pdf_object_key: Option<String>,
    pub is_provisional: bool,
    pub credit_note_for: Option<InvoiceId>,
    pub credit_note_reason: Option<String>,
    pub retry_count: i32,
    pub issued_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCreditNoteRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxReportSummary {
    pub taxable_sales: Money,
    pub exempt_sales: Money,
    pub zero_rated_sales: Money,
    pub total_sales: Money,
    pub total_tax_collected: Money,
    pub total_invoices_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxReportDto {
    pub from_date: String,
    pub to_date: String,
    pub branch_id: Option<BranchId>,
    pub summary: TaxReportSummary,
    pub lines: Vec<TaxLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FbrQueueStatusDto {
    pub pending_count: i64,
    pub submitting_count: i64,
    pub accepted_count: i64,
    pub rejected_count: i64,
    pub failed_count: i64,
    pub stale_pending_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FiscalSubmissionResponse {
    pub fiscal_invoice_no: String,
    pub fbr_invoice_number: String,
    pub qr_code_data: String,
    pub status: String,
    pub raw_response: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FiscalStatusResponse {
    pub status: String,
    pub fbr_reference: String,
    pub verified_at: DateTime<Utc>,
}
