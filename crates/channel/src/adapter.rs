use crate::error::ChannelError;
use crate::types::*;
use async_trait::async_trait;
use shifa_core::id::ChannelId;

/// Unified channel adapter interface for WhatsApp transports (Meta Cloud API / Baileys).
/// Invariant I-10: Business logic interacts ONLY with this trait and never branches on transport.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn channel_id(&self) -> ChannelId;
    fn transport(&self) -> Transport;
    fn capabilities(&self) -> Capabilities;
    async fn send(
        &self,
        msg: OutboundMessage,
        is_window_open: bool,
    ) -> Result<MessageReceipt, ChannelError>;
    async fn download_media(&self, r: &MediaRef) -> Result<MediaBytes, ChannelError>;
    async fn health(&self) -> ChannelHealth;
}
