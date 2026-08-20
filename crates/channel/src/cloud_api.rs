use crate::adapter::ChannelAdapter;
use crate::error::ChannelError;
use crate::templates::TemplateRegistry;
use crate::types::*;
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use shifa_core::id::ChannelId;
use std::sync::Arc;

pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024; // 5 MB
pub const MAX_DOCUMENT_BYTES: usize = 100 * 1024 * 1024; // 100 MB
pub const MAX_AUDIO_BYTES: usize = 16 * 1024 * 1024; // 16 MB

#[derive(Clone)]
pub struct CloudApiConfig {
    pub base_url: String,
    pub api_version: String,
    pub phone_number_id: String,
    pub access_token: String,
}

pub struct CloudApiAdapter {
    channel_id: ChannelId,
    config: CloudApiConfig,
    client: Client,
    capabilities: Capabilities,
    template_registry: Arc<TemplateRegistry>,
}

impl CloudApiAdapter {
    pub fn new(
        channel_id: ChannelId,
        config: CloudApiConfig,
        template_registry: Arc<TemplateRegistry>,
    ) -> Self {
        Self {
            channel_id,
            config,
            client: Client::new(),
            capabilities: Capabilities::cloud_api_default(),
            template_registry,
        }
    }

    /// Format choice into 3 tiers per Doc 02 §4.1:
    /// - Choice <= 3 options: interactive reply buttons
    /// - Choice 4..=10 options: interactive list message
    /// - Choice > 10 options: numbered text list
    pub fn render_choice(prompt: &str, options: &[ChoiceOption]) -> serde_json::Value {
        if options.len() <= 3 {
            // Interactive reply buttons
            let buttons: Vec<_> = options
                .iter()
                .map(|opt| {
                    json!({
                        "type": "reply",
                        "reply": {
                            "id": opt.id,
                            "title": if opt.title.len() > 20 { &opt.title[..20] } else { &opt.title }
                        }
                    })
                })
                .collect();

            json!({
                "type": "interactive",
                "interactive": {
                    "type": "button",
                    "body": { "text": prompt },
                    "action": { "buttons": buttons }
                }
            })
        } else if options.len() <= 10 {
            // Interactive list message
            let rows: Vec<_> = options
                .iter()
                .map(|opt| {
                    let mut row = json!({
                        "id": opt.id,
                        "title": if opt.title.len() > 24 { &opt.title[..24] } else { &opt.title }
                    });
                    if let Some(desc) = &opt.description {
                        row["description"] =
                            json!(if desc.len() > 72 { &desc[..72] } else { desc });
                    }
                    row
                })
                .collect();

            json!({
                "type": "interactive",
                "interactive": {
                    "type": "list",
                    "body": { "text": prompt },
                    "action": {
                        "button": "Select Option",
                        "sections": [{
                            "title": "Options",
                            "rows": rows
                        }]
                    }
                }
            })
        } else {
            // > 10 options: Numbered text list
            let mut text = format!("{}\n\n", prompt);
            for (idx, opt) in options.iter().enumerate() {
                text.push_str(&format!("{}. {}\n", idx + 1, opt.title));
            }
            text.push_str(&format!(
                "\nReply with the option number (1-{})",
                options.len()
            ));

            json!({
                "type": "text",
                "text": { "body": text }
            })
        }
    }

    /// Render Confirm intent as two reply buttons
    pub fn render_confirm(prompt: &str, yes: &str, no: &str) -> serde_json::Value {
        json!({
            "type": "interactive",
            "interactive": {
                "type": "button",
                "body": { "text": prompt },
                "action": {
                    "buttons": [
                        { "type": "reply", "reply": { "id": "yes", "title": if yes.len() > 20 { &yes[..20] } else { yes } } },
                        { "type": "reply", "reply": { "id": "no", "title": if no.len() > 20 { &no[..20] } else { no } } }
                    ]
                }
            }
        })
    }
}

#[async_trait]
impl ChannelAdapter for CloudApiAdapter {
    fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    fn transport(&self) -> Transport {
        Transport::CloudApi
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    async fn send(
        &self,
        msg: OutboundMessage,
        is_window_open: bool,
    ) -> Result<MessageReceipt, ChannelError> {
        let is_template = matches!(msg.body, OutboundBody::Template { .. });

        // Enforce 24-hour service window tracking per Doc 02 §6:
        // Free-form messages outside the window MUST fail loudly with Err(WindowClosed).
        if !is_window_open && !is_template {
            return Err(ChannelError::WindowClosed);
        }

        // Build Meta Cloud API payload
        let mut payload = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": msg.to.trim_start_matches('+'),
        });

        match msg.body {
            OutboundBody::Text { body } => {
                payload["type"] = json!("text");
                payload["text"] = json!({ "body": body });
            }
            OutboundBody::Choice {
                prompt, options, ..
            } => {
                let interactive = Self::render_choice(&prompt, &options);
                for (k, v) in interactive.as_object().unwrap() {
                    payload[k] = v.clone();
                }
            }
            OutboundBody::Confirm { prompt, yes, no } => {
                let interactive = Self::render_confirm(&prompt, &yes, &no);
                for (k, v) in interactive.as_object().unwrap() {
                    payload[k] = v.clone();
                }
            }
            OutboundBody::Media {
                kind,
                object_key,
                caption,
            } => {
                let kind_str = match kind {
                    MediaKind::Image => "image",
                    MediaKind::Document => "document",
                    MediaKind::Audio => "audio",
                    MediaKind::Video => "video",
                };
                payload["type"] = json!(kind_str);
                let mut media_obj = json!({ "link": object_key });
                if let Some(cap) = caption {
                    media_obj["caption"] = json!(cap);
                }
                payload[kind_str] = media_obj;
            }
            OutboundBody::Document {
                object_key,
                filename,
                caption,
            } => {
                payload["type"] = json!("document");
                let mut doc_obj = json!({ "link": object_key, "filename": filename });
                if let Some(cap) = caption {
                    doc_obj["caption"] = json!(cap);
                }
                payload["document"] = doc_obj;
            }
            OutboundBody::Template {
                name,
                language,
                params,
            } => {
                // Verify template is APPROVED before making any network call per Doc 02 §7
                let template_status = self.template_registry.get_status(&name).await;
                match template_status {
                    Some(ref s) if s == "APPROVED" => (),
                    Some(s) => return Err(ChannelError::TemplateNotApproved(name, s)),
                    None => return Err(ChannelError::TemplateNotFound(name)),
                }

                let parameters: Vec<_> = params
                    .into_iter()
                    .map(|p| json!({ "type": "text", "text": p.value }))
                    .collect();

                payload["type"] = json!("template");
                payload["template"] = json!({
                    "name": name,
                    "language": { "code": language },
                    "components": [{
                        "type": "body",
                        "parameters": parameters
                    }]
                });
            }
        }

        let url = format!(
            "{}/{}/{}/messages",
            self.config.base_url.trim_end_matches('/'),
            self.config.api_version,
            self.config.phone_number_id
        );

        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.config.access_token)
            .header("X-Idempotency-Key", msg.idempotency_key.to_string())
            .json(&payload)
            .send()
            .await?;

        let status = res.status();
        if status.is_success() {
            let body: serde_json::Value = res.json().await?;
            let msg_id = body["messages"][0]["id"]
                .as_str()
                .unwrap_or("wamid.mock")
                .to_string();

            Ok(MessageReceipt {
                transport_message_id: msg_id,
                status: "ACCEPTED".to_string(),
                timestamp: Utc::now(),
            })
        } else if status.as_u16() == 429 || status.is_server_error() {
            let text = res.text().await.unwrap_or_default();
            Err(ChannelError::TransientError(status.as_u16(), text))
        } else {
            let text = res.text().await.unwrap_or_default();
            Err(ChannelError::PermanentError(status.as_u16(), text))
        }
    }

    async fn download_media(&self, r: &MediaRef) -> Result<MediaBytes, ChannelError> {
        let media_url = match &r.url {
            Some(u) => u.clone(),
            None => {
                let url = format!(
                    "{}/{}/{}",
                    self.config.base_url.trim_end_matches('/'),
                    self.config.api_version,
                    r.id
                );
                let res = self
                    .client
                    .get(&url)
                    .bearer_auth(&self.config.access_token)
                    .send()
                    .await?;
                let json: serde_json::Value = res.json().await?;
                json["url"]
                    .as_str()
                    .ok_or_else(|| {
                        ChannelError::PermanentError(404, "Missing media URL from Meta".to_string())
                    })?
                    .to_string()
            }
        };

        let media_res = self
            .client
            .get(&media_url)
            .bearer_auth(&self.config.access_token)
            .send()
            .await?;

        let content_type = media_res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(&r.mime_type)
            .to_string();

        let bytes = media_res.bytes().await?;

        // Enforce media size limits
        let size = bytes.len();
        if content_type.starts_with("image/") && size > MAX_IMAGE_BYTES {
            return Err(ChannelError::MediaTooLarge(size, MAX_IMAGE_BYTES));
        } else if content_type.starts_with("audio/") && size > MAX_AUDIO_BYTES {
            return Err(ChannelError::MediaTooLarge(size, MAX_AUDIO_BYTES));
        } else if size > MAX_DOCUMENT_BYTES {
            return Err(ChannelError::MediaTooLarge(size, MAX_DOCUMENT_BYTES));
        }

        Ok(MediaBytes {
            data: bytes.to_vec(),
            content_type,
            filename: None,
        })
    }

    async fn health(&self) -> ChannelHealth {
        ChannelHealth {
            healthy: true,
            last_inbound_at: Some(Utc::now()),
            last_outbound_at: Some(Utc::now()),
            error_rate: 0.0,
        }
    }
}
