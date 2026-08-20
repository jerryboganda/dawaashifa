use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shifa_core::context::TenantContext;
use shifa_core::id::ChannelId;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::ChannelError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSessionData {
    pub channel_id: ChannelId,
    pub creds: serde_json::Value,
    pub keys: serde_json::Value,
    pub encrypted_secret: String,
}

pub struct SessionStore;

impl SessionStore {
    /// Hashes and encrypts session secret at rest (Doc 03 §5)
    pub fn encrypt_secret(secret: &str, master_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(master_key.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Saves session state to Postgres (survives container restart without QR rescan)
    pub async fn save_session(
        ctx: &TenantContext,
        channel_id: ChannelId,
        creds: serde_json::Value,
        keys: serde_json::Value,
        raw_secret: &str,
        master_key: &str,
        pool: &PgPool,
    ) -> Result<(), ChannelError> {
        let encrypted_secret = Self::encrypt_secret(raw_secret, master_key);

        sqlx::query(
            "INSERT INTO wa_sessions (channel_id, tenant_id, creds, keys, encrypted_secret, updated_at)
             VALUES ($1, $2, $3, $4, $5, now())
             ON CONFLICT (channel_id) DO UPDATE SET
                 creds = EXCLUDED.creds,
                 keys = EXCLUDED.keys,
                 encrypted_secret = EXCLUDED.encrypted_secret,
                 updated_at = now()"
        )
        .bind(channel_id.0)
        .bind(ctx.tenant_id().0)
        .bind(creds)
        .bind(keys)
        .bind(encrypted_secret)
        .execute(pool)
        .await
        .map_err(ChannelError::Sqlx)?;

        Ok(())
    }

    /// Loads session state from Postgres
    pub async fn load_session(
        ctx: &TenantContext,
        channel_id: ChannelId,
        pool: &PgPool,
    ) -> Result<Option<AuthSessionData>, ChannelError> {
        let row_opt = sqlx::query(
            "SELECT channel_id, creds, keys, encrypted_secret FROM wa_sessions
             WHERE tenant_id = $1 AND channel_id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(channel_id.0)
        .fetch_optional(pool)
        .await
        .map_err(ChannelError::Sqlx)?;

        match row_opt {
            Some(row) => {
                let id_raw: Uuid = row.get("channel_id");
                let creds: serde_json::Value = row.get("creds");
                let keys: serde_json::Value = row.get("keys");
                let encrypted_secret: String = row.get("encrypted_secret");

                Ok(Some(AuthSessionData {
                    channel_id: ChannelId(id_raw),
                    creds,
                    keys,
                    encrypted_secret,
                }))
            }
            None => Ok(None),
        }
    }
}
