use shifa_core::id::{ConversationId, CustomerId, MessageId, UserId};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConversationError {
    #[error("Conversation {0} not found")]
    NotFound(ConversationId),

    #[error("Customer {0} not found")]
    CustomerNotFound(CustomerId),

    #[error("Message {0} not found")]
    MessageNotFound(MessageId),

    #[error("Conversation already claimed by user {0}")]
    AlreadyClaimed(UserId),

    #[error("Invalid conversation status transition from {0} to {1}")]
    InvalidStatusTransition(String, String),

    #[error("Invalid message status transition from {0} to {1}")]
    InvalidMessageStatusTransition(String, String),

    #[error(
        "Bulk approval rejected: Rx-linked conversations require individual review (Invariant I-6)"
    )]
    BulkApprovalRejectedForRx,

    #[error("Canned reply contains unresolved variables: {0}")]
    UnresolvedVariables(String),

    #[error("Free-form message rejected: outside 24h WhatsApp service window, template required")]
    OutsideServiceWindow,

    #[error("Unauthorized conversation action: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    Db(#[from] shifa_db::DbError),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}
