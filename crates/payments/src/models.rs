use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{OrderId, PaymentId, ProofId, TenantId, UserId};
use shifa_core::money::Money;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum PaymentMethod {
    #[serde(rename = "COD")]
    Cod,
    #[serde(rename = "JAZZCASH")]
    JazzCash,
    #[serde(rename = "EASYPAISA")]
    EasyPaisa,
    #[serde(rename = "RAAST")]
    Raast,
    #[serde(rename = "DIRECT_DEPOSIT")]
    BankTransfer,
    #[serde(rename = "SAFEPAY")]
    Aggregator,
}

impl PaymentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cod => "COD",
            Self::JazzCash => "JAZZCASH",
            Self::EasyPaisa => "EASYPAISA",
            Self::Raast => "RAAST",
            Self::BankTransfer => "DIRECT_DEPOSIT",
            Self::Aggregator => "SAFEPAY",
        }
    }
}

impl std::str::FromStr for PaymentMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "COD" => Ok(Self::Cod),
            "JAZZCASH" => Ok(Self::JazzCash),
            "EASYPAISA" => Ok(Self::EasyPaisa),
            "RAAST" => Ok(Self::Raast),
            "DIRECT_DEPOSIT" | "BANK_TRANSFER" => Ok(Self::BankTransfer),
            "SAFEPAY" | "PAYFAST" | "AGGREGATOR" => Ok(Self::Aggregator),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum PaymentStatus {
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "AWAITING_PROOF")]
    AwaitingProof,
    #[serde(rename = "UNDER_REVIEW")]
    UnderReview,
    #[serde(rename = "CONFIRMED")]
    Confirmed,
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "REFUNDED")]
    Refunded,
    #[serde(rename = "FAILED")]
    Failed,
}

impl PaymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::AwaitingProof => "AWAITING_PROOF",
            Self::UnderReview => "UNDER_REVIEW",
            Self::Confirmed => "CONFIRMED",
            Self::Rejected => "REJECTED",
            Self::Refunded => "REFUNDED",
            Self::Failed => "FAILED",
        }
    }
}

impl std::str::FromStr for PaymentStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(Self::Pending),
            "AWAITING_PROOF" => Ok(Self::AwaitingProof),
            "UNDER_REVIEW" => Ok(Self::UnderReview),
            "CONFIRMED" => Ok(Self::Confirmed),
            "REJECTED" => Ok(Self::Rejected),
            "REFUNDED" => Ok(Self::Refunded),
            "FAILED" => Ok(Self::Failed),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum ProofReviewStatus {
    #[serde(rename = "PENDING")]
    Pending,
    #[serde(rename = "APPROVED")]
    Approved,
    #[serde(rename = "REJECTED")]
    Rejected,
}

impl ProofReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
        }
    }
}

impl std::str::FromStr for ProofReviewStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(Self::Pending),
            "APPROVED" => Ok(Self::Approved),
            "REJECTED" => Ok(Self::Rejected),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum FraudSeverity {
    #[serde(rename = "CRITICAL")]
    Critical,
    #[serde(rename = "HIGH")]
    High,
    #[serde(rename = "MEDIUM")]
    Medium,
    #[serde(rename = "LOW")]
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum FraudFlagType {
    #[serde(rename = "DUPLICATE_TID")]
    DuplicateTid,
    #[serde(rename = "AMOUNT_MISMATCH")]
    AmountMismatch,
    #[serde(rename = "TIMESTAMP_BEFORE_ORDER")]
    TimestampBeforeOrder,
    #[serde(rename = "TIMESTAMP_STALE")]
    TimestampStale,
    #[serde(rename = "EDITED_IMAGE")]
    EditedImage,
    #[serde(rename = "SENDER_REUSED_ACROSS_CUSTOMERS")]
    SenderReusedAcrossCustomers,
    #[serde(rename = "LOW_OCR_CONFIDENCE")]
    LowOcrConfidence,
    #[serde(rename = "UNKNOWN_BANK_LAYOUT")]
    UnknownBankLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FraudFlag {
    pub flag_type: FraudFlagType,
    pub severity: FraudSeverity,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IntentRequest {
    pub order_id: OrderId,
    pub method: PaymentMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentIntent {
    pub payment_id: PaymentId,
    pub order_id: OrderId,
    pub method: PaymentMethod,
    pub amount: Money,
    pub payment_url: Option<String>,
    pub instructions: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateProofRequest {
    pub order_id: OrderId,
    pub payment_id: Option<PaymentId>,
    pub image_object_key: String,
    pub raw_exif_software: Option<String>,
    pub raw_sender: Option<String>,
    pub client_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApproveProofRequest {
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RejectProofRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefundRequest {
    pub amount: Money,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentDto {
    pub id: PaymentId,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub method: PaymentMethod,
    pub amount: Money,
    pub status: PaymentStatus,
    pub gateway: Option<String>,
    pub gateway_ref: Option<String>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub confirmed_by: Option<UserId>,
    pub refund_reason: Option<String>,
    pub refunded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentProofDto {
    pub id: ProofId,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub payment_id: Option<PaymentId>,
    pub image_object_key: String,
    pub ocr_tid: Option<String>,
    pub ocr_amount: Option<Money>,
    pub ocr_timestamp: Option<DateTime<Utc>>,
    pub ocr_sender: Option<String>,
    pub ocr_bank: Option<String>,
    pub ocr_confidence: Option<f32>,
    pub duplicate_of_proof_id: Option<ProofId>,
    pub fraud_flags: Vec<FraudFlag>,
    pub review_status: ProofReviewStatus,
    pub reviewed_by: Option<UserId>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookEvent {
    pub gateway: String,
    pub gateway_ref: String,
    pub order_id: OrderId,
    pub amount: Money,
    pub status: PaymentStatus,
    pub timestamp: DateTime<Utc>,
    pub raw_payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefundResult {
    pub payment_id: PaymentId,
    pub refunded_amount: Money,
    pub status: PaymentStatus,
    pub refund_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconciliationDiscrepancy {
    pub payment_id: Option<PaymentId>,
    pub gateway_ref: Option<String>,
    pub discrepancy_type: String,
    pub expected_amount: Money,
    pub settled_amount: Money,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconciliationReportDto {
    pub report_date: String,
    pub gateway: String,
    pub expected_total: Money,
    pub settled_total: Money,
    pub fee_total: Money,
    pub unmatched_count: i32,
    pub discrepancies: Vec<ReconciliationDiscrepancy>,
}
