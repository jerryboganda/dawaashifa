use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("24-hour service window closed: cannot send free-form message. Must send an approved Template.")]
    WindowClosed,

    #[error("Template '{0}' is not approved by Meta (current status: {1})")]
    TemplateNotApproved(String, String),

    #[error("Template '{0}' not found in registry")]
    TemplateNotFound(String),

    #[error("Invalid webhook signature")]
    InvalidSignature,

    #[error("Rate limit exceeded for channel")]
    RateLimitExceeded,

    #[error("Media exceeds maximum permitted size: {0} bytes (limit is {1} bytes)")]
    MediaTooLarge(usize, usize),

    #[error("Permanent transport error (HTTP {0}): {1}")]
    PermanentError(u16, String),

    #[error("Transient transport error (HTTP {0}): {1}")]
    TransientError(u16, String),

    #[error("Transport network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Identity isolation violation: Unofficial Baileys channel cannot join Official WABA identity")]
    IdentityIsolationViolation,

    #[error("Channel {0} is banned by WhatsApp and cannot send messages")]
    ChannelBanned(String),

    #[error("Daily message limit exceeded for channel ({current}/{limit})")]
    DailyLimitExceeded { current: u32, limit: u32 },

    #[error("Session not found for channel {0}")]
    SessionNotFound(String),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Db(#[from] shifa_db::DbError),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
