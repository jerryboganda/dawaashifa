use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{BranchId, ConversationId, CustomerId, MessageId, TenantId, UserId};
use std::fmt;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ConversationStatus {
    NEW,
    BOT_HANDLING,
    AWAITING_HUMAN,
    ASSIGNED,
    ESCALATED,
    RESOLVED,
    CLOSED,
}

impl fmt::Display for ConversationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::NEW => "NEW",
            Self::BOT_HANDLING => "BOT_HANDLING",
            Self::AWAITING_HUMAN => "AWAITING_HUMAN",
            Self::ASSIGNED => "ASSIGNED",
            Self::ESCALATED => "ESCALATED",
            Self::RESOLVED => "RESOLVED",
            Self::CLOSED => "CLOSED",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConversationDto {
    pub id: ConversationId,
    pub tenant_id: TenantId,
    pub customer_id: CustomerId,
    pub branch_id: Option<BranchId>,
    pub status: String,
    pub assigned_to: Option<UserId>,
    pub is_rx_linked: bool,
    pub unread_count: i32,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessageDto {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub sender_type: String, // CUSTOMER | AGENT | BOT | SYSTEM
    pub sender_id: Option<Uuid>,
    pub direction: String, // INBOUND | OUTBOUND
    pub status: String, // DRAFT | PENDING_APPROVAL | APPROVED | QUEUED | SENT | DELIVERED | READ | DISCARDED
    pub body: String,
    pub original_body: Option<String>,
    pub overridden_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InboundMessageRequest {
    pub msisdn: String,
    pub display_name: Option<String>,
    pub text: String,
    pub channel_id: Option<Uuid>,
    pub branch_id: Option<BranchId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub body: String,
    pub is_template: bool,
    pub template_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OverrideMessageRequest {
    pub new_body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CannedReplyDto {
    pub id: Uuid,
    pub shortcode: String,
    pub title: String,
    pub body_en: String,
    pub body_ur: Option<String>,
    pub body_ur_latn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCannedReplyRequest {
    pub branch_id: Option<BranchId>,
    pub shortcode: String,
    pub title: String,
    pub body_en: String,
    pub body_ur: Option<String>,
    pub body_ur_latn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignConversationRequest {
    pub user_id: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransferConversationRequest {
    pub branch_id: BranchId,
}
