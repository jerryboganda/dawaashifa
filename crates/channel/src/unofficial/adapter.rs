use async_trait::async_trait;
use chrono::Utc;
use shifa_core::id::ChannelId;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapter::ChannelAdapter;
use crate::error::ChannelError;
use crate::types::*;
use crate::unofficial::pacer::HumanPacer;

pub struct UnofficialAdapter {
    pub channel_id: ChannelId,
    pub pacer: Arc<HumanPacer>,
}

impl UnofficialAdapter {
    pub fn new(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
            pacer: Arc::new(HumanPacer::new()),
        }
    }

    /// Renders choice options as numbered plain text (Doc 03 §6)
    pub fn render_choice_as_numbered_text(prompt: &str, options: &[ChoiceOption]) -> String {
        let mut lines = Vec::new();
        lines.push(prompt.to_string());
        for (i, opt) in options.iter().enumerate() {
            if let Some(ref desc) = opt.description {
                lines.push(format!("{}. {} - {}", i + 1, opt.title, desc));
            } else {
                lines.push(format!("{}. {}", i + 1, opt.title));
            }
        }
        lines.push("(Reply with a number)".to_string());
        lines.join("\n")
    }

    /// Renders confirmation prompt for unofficial WhatsApp text transport
    pub fn render_confirm_as_text(prompt: &str) -> String {
        format!("{}\n\nReply YES to confirm, NO to cancel", prompt)
    }

    /// Renders template payload into plain text fallback
    pub fn render_template_fallback(params: &[TemplateParam]) -> String {
        let param_lines: Vec<String> = params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.value))
            .collect();
        param_lines.join("\n")
    }
}

#[async_trait]
impl ChannelAdapter for UnofficialAdapter {
    fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    fn transport(&self) -> Transport {
        Transport::Unofficial
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::unofficial_default()
    }

    async fn send(
        &self,
        msg: OutboundMessage,
        _is_window_open: bool,
    ) -> Result<MessageReceipt, ChannelError> {
        let _rendered_text = match msg.body {
            OutboundBody::Text { body } => body,
            OutboundBody::Choice {
                prompt, options, ..
            } => Self::render_choice_as_numbered_text(&prompt, &options),
            OutboundBody::Confirm { prompt, .. } => Self::render_confirm_as_text(&prompt),
            OutboundBody::Template { params, .. } => Self::render_template_fallback(&params),
            OutboundBody::Media { caption, .. } => {
                caption.unwrap_or_else(|| "[Media Attachment]".to_string())
            }
            OutboundBody::Document {
                caption, filename, ..
            } => caption.unwrap_or_else(|| format!("[Document: {}]", filename)),
        };

        // Enforce human pacing gap (Doc 03 §7)
        self.pacer.enforce_minimum_gap(1).await;

        let transport_id = format!("baileys_{}", Uuid::now_v7());

        Ok(MessageReceipt {
            transport_message_id: transport_id,
            status: "sent".to_string(),
            timestamp: Utc::now(),
        })
    }

    async fn download_media(&self, _r: &MediaRef) -> Result<MediaBytes, ChannelError> {
        Ok(MediaBytes {
            data: vec![],
            content_type: "application/octet-stream".to_string(),
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
