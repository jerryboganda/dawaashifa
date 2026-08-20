use crate::error::ChannelError;
use crate::types::*;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use shifa_core::id::{ChannelId, TenantId};
use tracing::warn;

type HmacSha256 = Hmac<Sha256>;

/// Verify Meta webhook `X-Hub-Signature-256` signature against app secret.
/// Invariant: On mismatch, reject with 403 and DO NOT log the body content.
pub fn verify_hub_signature(
    body: &[u8],
    signature_header: &str,
    app_secret: &str,
) -> Result<(), ChannelError> {
    let expected_prefix = "sha256=";
    if !signature_header.starts_with(expected_prefix) {
        return Err(ChannelError::InvalidSignature);
    }

    let hex_sig = &signature_header[expected_prefix.len()..];
    let signature_bytes = hex::decode(hex_sig).map_err(|_| ChannelError::InvalidSignature)?;

    let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes())
        .map_err(|_| ChannelError::InvalidSignature)?;
    mac.update(body);

    mac.verify_slice(&signature_bytes)
        .map_err(|_| ChannelError::InvalidSignature)
}

/// Parse and normalize inbound webhook JSON payload from Meta Cloud API
pub fn parse_inbound_webhook(
    payload: &serde_json::Value,
    tenant_id: TenantId,
    channel_id: ChannelId,
) -> Vec<InboundMessage> {
    let mut messages = Vec::new();

    let entries = match payload["entry"].as_array() {
        Some(e) => e,
        None => return messages,
    };

    for entry in entries {
        let changes = match entry["changes"].as_array() {
            Some(c) => c,
            None => continue,
        };

        for change in changes {
            let value = &change["value"];
            let raw_messages = match value["messages"].as_array() {
                Some(m) => m,
                None => continue,
            };

            for msg in raw_messages {
                let from = msg["from"].as_str().unwrap_or_default().to_string();
                let transport_message_id = msg["id"].as_str().unwrap_or_default().to_string();
                let msg_type = msg["type"].as_str().unwrap_or_default();

                let content = match msg_type {
                    "text" => {
                        let body = msg["text"]["body"].as_str().unwrap_or_default().to_string();
                        InboundContent::Text { body }
                    }
                    "image" => {
                        let media_id = msg["image"]["id"].as_str().unwrap_or_default().to_string();
                        let mime_type = msg["image"]["mime_type"]
                            .as_str()
                            .unwrap_or("image/jpeg")
                            .to_string();
                        let caption = msg["image"]["caption"].as_str().map(|s| s.to_string());
                        InboundContent::Image {
                            media_id,
                            caption,
                            mime_type,
                        }
                    }
                    "audio" => {
                        let media_id = msg["audio"]["id"].as_str().unwrap_or_default().to_string();
                        let mime_type = msg["audio"]["mime_type"]
                            .as_str()
                            .unwrap_or("audio/ogg")
                            .to_string();
                        InboundContent::Audio {
                            media_id,
                            mime_type,
                        }
                    }
                    "document" => {
                        let media_id = msg["document"]["id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string();
                        let filename = msg["document"]["filename"]
                            .as_str()
                            .unwrap_or("document")
                            .to_string();
                        let mime_type = msg["document"]["mime_type"]
                            .as_str()
                            .unwrap_or("application/pdf")
                            .to_string();
                        let caption = msg["document"]["caption"].as_str().map(|s| s.to_string());
                        InboundContent::Document {
                            media_id,
                            filename,
                            mime_type,
                            caption,
                        }
                    }
                    "location" => {
                        let lat = msg["location"]["latitude"].as_f64().unwrap_or(0.0);
                        let lon = msg["location"]["longitude"].as_f64().unwrap_or(0.0);
                        let name = msg["location"]["name"].as_str().map(|s| s.to_string());
                        let address = msg["location"]["address"].as_str().map(|s| s.to_string());
                        InboundContent::Location {
                            latitude: lat,
                            longitude: lon,
                            name,
                            address,
                        }
                    }
                    "interactive" => {
                        let interactive = &msg["interactive"];
                        let itype = interactive["type"].as_str().unwrap_or_default();
                        if itype == "button_reply" {
                            let button_id = interactive["button_reply"]["id"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string();
                            let title = interactive["button_reply"]["title"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string();
                            InboundContent::ButtonReply { button_id, title }
                        } else if itype == "list_reply" {
                            let item_id = interactive["list_reply"]["id"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string();
                            let title = interactive["list_reply"]["title"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string();
                            let description = interactive["list_reply"]["description"]
                                .as_str()
                                .map(|s| s.to_string());
                            InboundContent::ListReply {
                                item_id,
                                title,
                                description,
                            }
                        } else {
                            InboundContent::Unsupported {
                                raw_type: format!("interactive:{}", itype),
                            }
                        }
                    }
                    other => {
                        // Unknown message types must NOT error. Stored as Unsupported per Doc 02 §5.
                        warn!("Received unsupported WhatsApp message type: {}", other);
                        InboundContent::Unsupported {
                            raw_type: other.to_string(),
                        }
                    }
                };

                messages.push(InboundMessage {
                    tenant_id,
                    channel_id,
                    from: if from.starts_with('+') {
                        from
                    } else {
                        format!("+{}", from)
                    },
                    transport_message_id,
                    content,
                    timestamp: Utc::now(),
                    raw: msg.clone(),
                });
            }
        }
    }

    messages
}
