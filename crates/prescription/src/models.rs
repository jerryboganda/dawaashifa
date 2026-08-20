use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{
    BranchId, ConversationId, CustomerId, PrescriptionId, ProductId, TenantId, UserId,
};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum PrescriptionStatus {
    #[serde(rename = "RECEIVED")]
    Received,
    #[serde(rename = "PREPROCESSING")]
    Preprocessing,
    #[serde(rename = "EXTRACTING")]
    Extracting,
    #[serde(rename = "PENDING_REVIEW")]
    PendingReview,
    #[serde(rename = "UNDER_REVIEW")]
    UnderReview,
    #[serde(rename = "APPROVED")]
    Approved,
    #[serde(rename = "PARTIALLY_APPROVED")]
    PartiallyApproved,
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "NEEDS_CLARIFICATION")]
    NeedsClarification,
    #[serde(rename = "CANCELLED")]
    Cancelled,
}

impl PrescriptionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Received => "RECEIVED",
            Self::Preprocessing => "PREPROCESSING",
            Self::Extracting => "EXTRACTING",
            Self::PendingReview => "PENDING_REVIEW",
            Self::UnderReview => "UNDER_REVIEW",
            Self::Approved => "APPROVED",
            Self::PartiallyApproved => "PARTIALLY_APPROVED",
            Self::Rejected => "REJECTED",
            Self::NeedsClarification => "NEEDS_CLARIFICATION",
            Self::Cancelled => "CANCELLED",
        }
    }
}

impl std::str::FromStr for PrescriptionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RECEIVED" => Ok(Self::Received),
            "PREPROCESSING" => Ok(Self::Preprocessing),
            "EXTRACTING" => Ok(Self::Extracting),
            "PENDING_REVIEW" | "PENDING_OCR" => Ok(Self::PendingReview),
            "UNDER_REVIEW" | "RX_UNDER_REVIEW" => Ok(Self::UnderReview),
            "APPROVED" => Ok(Self::Approved),
            "PARTIALLY_APPROVED" => Ok(Self::PartiallyApproved),
            "REJECTED" => Ok(Self::Rejected),
            "NEEDS_CLARIFICATION" => Ok(Self::NeedsClarification),
            "CANCELLED" => Ok(Self::Cancelled),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RxExtractedLine {
    pub line_no: i32,
    pub raw_text: String,
    pub drug_text: Option<String>,
    pub strength_text: Option<String>,
    pub form_text: Option<String>,
    pub qty_text: Option<String>,
    pub dosage_text: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RxExtraction {
    pub doctor_name: Option<String>,
    pub doctor_pmdc_no: Option<String>,
    pub issued_date: Option<NaiveDate>,
    pub patient_name: Option<String>,
    pub lines: Vec<RxExtractedLine>,
    pub overall_confidence: f64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum LineAction {
    Accept,
    Edit {
        product_id: ProductId,
        qty: i32,
        dosage: Option<String>,
    },
    Substitute {
        product_id: ProductId,
        reason: String,
    },
    Reject {
        reason: String,
    },
    AddManual {
        product_id: ProductId,
        qty: i32,
        dosage: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LineDecision {
    pub line_no: i32,
    pub action: LineAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePrescriptionRequest {
    pub customer_id: CustomerId,
    pub conversation_id: Option<ConversationId>,
    pub branch_id: Option<BranchId>,
    pub image_object_key: String,
    pub source_channel: Option<String>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub image_bytes_len: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApprovePrescriptionRequest {
    pub decisions: Vec<LineDecision>,
    pub note: Option<String>,
    pub client_ip: Option<String>,
    pub client_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RejectPrescriptionRequest {
    pub reason: String,
    pub client_ip: Option<String>,
    pub client_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClarifyPrescriptionRequest {
    pub question_to_customer: String,
    pub client_ip: Option<String>,
    pub client_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApprovalResult {
    pub prescription_id: PrescriptionId,
    pub status: PrescriptionStatus,
    pub approved_lines_count: usize,
    pub rejected_lines_count: usize,
    pub approval_id: Uuid,
    pub controlled_substances_dispensed: usize,
    pub substitutions_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrescriptionDto {
    pub id: PrescriptionId,
    pub tenant_id: TenantId,
    pub customer_id: CustomerId,
    pub conversation_id: Option<ConversationId>,
    pub branch_id: Option<BranchId>,
    pub image_object_key: String,
    pub preprocessed_image_key: Option<String>,
    pub source_channel: String,
    pub received_at: DateTime<Utc>,
    pub status: PrescriptionStatus,
    pub doctor_name: Option<String>,
    pub doctor_pmdc_no: Option<String>,
    pub issued_date: Option<NaiveDate>,
    pub patient_name: Option<String>,
    pub assigned_to: Option<UserId>,
    pub clarification_notes: Option<String>,
    pub lines: Vec<RxLineDto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RxLineDto {
    pub id: Uuid,
    pub line_no: i32,
    pub ocr_text: String,
    pub matched_product_id: Option<ProductId>,
    pub matched_brand_name: Option<String>,
    pub match_confidence: Option<f32>,
    pub match_method: Option<String>,
    pub qty: i32,
    pub dosage_instructions: Option<String>,
    pub pharmacist_action: Option<String>,
    pub pharmacist_note: Option<String>,
    pub is_controlled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueueStatsDto {
    pub total_pending: i64,
    pub total_under_review: i64,
    pub total_needs_clarification: i64,
    pub oldest_waiting_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RxAuditEntryDto {
    pub id: Uuid,
    pub action: String,
    pub actor_id: Option<UserId>,
    pub timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
}
