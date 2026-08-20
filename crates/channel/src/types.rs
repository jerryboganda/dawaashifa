use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{ChannelId, ConversationId, TenantId};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum Transport {
    CloudApi,
    Unofficial,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Capabilities {
    pub interactive_buttons: bool,
    pub list_messages: bool,
    pub templates: bool,
    pub outside_window: bool,
    pub delivery_receipts: bool,
    pub max_send_rate_per_min: u32,
    pub max_buttons: u8,
}

impl Capabilities {
    pub fn cloud_api_default() -> Self {
        Self {
            interactive_buttons: true,
            list_messages: true,
            templates: true,
            outside_window: false, // only via templates
            delivery_receipts: true,
            max_send_rate_per_min: 1500, // 25 msg/sec Meta limit
            max_buttons: 3,
        }
    }

    pub fn unofficial_default() -> Self {
        Self {
            interactive_buttons: false,
            list_messages: false,
            templates: false,
            outside_window: true,
            delivery_receipts: true,
            max_send_rate_per_min: 12,
            max_buttons: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum IdentityKind {
    UnofficialIsolated,
    OfficialWaba,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ChannelPoolStatus {
    Provisioning,
    Warming,
    Active,
    Degraded,
    Banned,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChoiceOption {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateParam {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MediaKind {
    Image,
    Document,
    Audio,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum OutboundBody {
    Text {
        body: String,
    },
    Choice {
        prompt: String,
        options: Vec<ChoiceOption>,
        min: u8,
        max: u8,
    },
    Confirm {
        prompt: String,
        yes: String,
        no: String,
    },
    Media {
        kind: MediaKind,
        object_key: String,
        caption: Option<String>,
    },
    Document {
        object_key: String,
        filename: String,
        caption: Option<String>,
    },
    Template {
        name: String,
        language: String,
        params: Vec<TemplateParam>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OutboundMessage {
    pub tenant_id: TenantId,
    pub conversation_id: ConversationId,
    pub to: String, // E.164 MSISDN (e.g. +923001234567)
    pub body: OutboundBody,
    pub idempotency_key: Uuid,
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum InboundContent {
    Text {
        body: String,
    },
    Image {
        media_id: String,
        caption: Option<String>,
        mime_type: String,
    },
    Audio {
        media_id: String,
        mime_type: String,
    },
    Document {
        media_id: String,
        filename: String,
        mime_type: String,
        caption: Option<String>,
    },
    Location {
        latitude: f64,
        longitude: f64,
        name: Option<String>,
        address: Option<String>,
    },
    ButtonReply {
        button_id: String,
        title: String,
    },
    ListReply {
        item_id: String,
        title: String,
        description: Option<String>,
    },
    Unsupported {
        raw_type: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InboundMessage {
    pub tenant_id: TenantId,
    pub channel_id: ChannelId,
    pub from: String, // E.164 MSISDN
    pub transport_message_id: String,
    pub content: InboundContent,
    pub timestamp: DateTime<Utc>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessageReceipt {
    pub transport_message_id: String,
    pub status: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChannelHealth {
    pub healthy: bool,
    pub last_inbound_at: Option<DateTime<Utc>>,
    pub last_outbound_at: Option<DateTime<Utc>>,
    pub error_rate: f64,
}

#[derive(Debug, Clone)]
pub struct MediaRef {
    pub id: String,
    pub url: Option<String>,
    pub mime_type: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MediaBytes {
    pub data: Vec<u8>,
    pub content_type: String,
    pub filename: Option<String>,
}
