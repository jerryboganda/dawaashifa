use serde::{Deserialize, Serialize};
use shifa_core::id::{ConversationId, MessageId};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum AiTask {
    Intent,
    Reply,
    RxOcr,
    Stt,
    Embed,
}

impl std::fmt::Display for AiTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intent => write!(f, "intent"),
            Self::Reply => write!(f, "reply"),
            Self::RxOcr => write!(f, "rx_ocr"),
            Self::Stt => write!(f, "stt"),
            Self::Embed => write!(f, "embed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CustomerScript {
    Urdu,
    English,
    RomanUrdu,
    CodeMixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum IntentType {
    ProductEnquiry,
    PriceEnquiry,
    AvailabilityCheck,
    PlaceOrder,
    OrderStatus,
    CancelOrder,
    PrescriptionUpload,
    DeliveryEnquiry,
    PaymentQuery,
    Complaint,
    Greeting,
    HumanRequest,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiOutcome<T> {
    pub value: T,
    pub confidence: f32,
    pub escalate: bool,
    pub escalation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalysisResult {
    pub detected_script: CustomerScript,
    pub normalised_text: String,
    pub intent: IntentType,
    pub entities: Vec<ExtractedEntity>,
    pub confidence: f32,
    pub escalate: bool,
    pub escalation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExtractedEntity {
    pub entity_type: String, // DRUG | DOSAGE | QUANTITY | BRAND
    pub value: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DraftReplyResult {
    pub draft_body: String,
    pub confidence: f32,
    pub escalate: bool,
    pub can_auto_send: bool,
    pub requires_pharmacist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TranscriptionResult {
    pub transcript: String,
    pub normalised_transcript: String,
    pub confidence: f32,
    pub duration_seconds: i32,
    pub escalate: bool,
    pub escalation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiAnalyseRequest {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub raw_text: String,
    pub is_rx_context: bool,
    pub contains_controlled_substance: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiDraftReplyRequest {
    pub conversation_id: ConversationId,
    pub customer_name: Option<String>,
    pub last_inbound_text: String,
    pub is_rx_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiTranscribeRequest {
    pub message_id: MessageId,
    pub audio_url: String,
    pub duration_seconds: i32,
    pub locale_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeedbackEventRequest {
    pub conversation_id: ConversationId,
    pub message_id: MessageId,
    pub task: String,
    pub prompt_version: String,
    pub ai_output: String,
    pub human_output: String,
    pub intent: String,
    pub confidence: f32,
    pub corrected_alias: Option<(String, String)>, // (alias, canonical_name)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiHealthStatus {
    pub task: String,
    pub state: String, // CLOSED | OPEN | HALF_OPEN
    pub failure_count: u32,
}
