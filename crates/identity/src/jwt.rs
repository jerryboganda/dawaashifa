use crate::error::AuthError;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use shifa_core::id::{SessionId, TenantId, UserId};

/// JWT Access Token Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: UserId,
    pub tid: TenantId,
    pub sid: SessionId,
    pub roles: Vec<String>,
    pub exp: usize,
    pub iat: usize,
}

/// Create a new 15-minute access token (HS256)
pub fn create_access_token(
    user_id: UserId,
    tenant_id: TenantId,
    session_id: SessionId,
    roles: Vec<String>,
    secret: &str,
) -> Result<String, AuthError> {
    let now = Utc::now();
    let exp = (now + Duration::minutes(15)).timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        tid: tenant_id,
        sid: session_id,
        roles,
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

/// Verify and decode an access token
pub fn verify_access_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}

/// Generate an opaque 32-byte cryptographically secure random refresh token string
pub fn generate_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Hash a refresh token before storing in database
pub fn hash_refresh_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
