use crate::error::AuthError;
use crate::jwt::{
    create_access_token, generate_refresh_token, hash_refresh_token, verify_access_token,
};
use crate::models::*;
use crate::password::verify_password;
use crate::roles::get_system_role_definitions;
use chrono::{DateTime, Duration, Utc};
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, SessionId, TenantId, UserId};
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

/// Parameters for writing an immutable audit log entry per Invariant I-9
#[derive(Debug, Clone)]
pub struct AuditLogParams {
    pub tenant_id: TenantId,
    pub actor_id: Option<UserId>,
    pub actor_type: &'static str,
    pub entity_type: &'static str,
    pub entity_id: Uuid,
    pub action: &'static str,
    pub before: Option<serde_json::Value>,
    pub after: serde_json::Value,
    pub reason: &'static str,
    pub ip: Option<String>,
}

/// In-memory rate limiter tracking failed attempts per identifier (phone / IP)
#[derive(Debug, Default)]
pub struct LoginRateLimiter {
    attempts: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_and_record_failure(&self, key: &str) -> Result<(), AuthError> {
        let mut map = self.attempts.lock().unwrap();
        let now = Utc::now();
        let cutoff = now - Duration::minutes(15);

        let entries = map.entry(key.to_string()).or_default();
        entries.retain(|&time| time > cutoff);

        if entries.len() >= 5 {
            return Err(AuthError::RateLimitExceeded);
        }

        entries.push(now);
        Ok(())
    }

    pub fn record_success(&self, key: &str) {
        let mut map = self.attempts.lock().unwrap();
        map.remove(key);
    }
}

/// Service handling authentication, RBAC, sessions, and branch scoping
#[derive(Debug, Clone)]
pub struct IdentityService {
    pool: PgPool,
    jwt_secret: String,
    rate_limiter: std::sync::Arc<LoginRateLimiter>,
}

impl IdentityService {
    pub fn new(pool: PgPool, jwt_secret: String) -> Self {
        Self {
            pool,
            jwt_secret,
            rate_limiter: std::sync::Arc::new(LoginRateLimiter::new()),
        }
    }

    /// Authenticate a user with phone or email and password
    pub async fn login(
        &self,
        req: LoginRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<AuthTokens, AuthError> {
        let rate_key = format!(
            "login:{}:{}",
            req.phone_or_email,
            ip.clone().unwrap_or_default()
        );
        self.rate_limiter.check_and_record_failure(&rate_key)?;

        // Fetch user by phone or email
        let user_row = sqlx::query(
            "SELECT id, tenant_id, password_hash, status::text as status
             FROM users
             WHERE phone = $1 OR email = $1",
        )
        .bind(&req.phone_or_email)
        .fetch_optional(&self.pool)
        .await?;

        let user = match user_row {
            Some(u) => u,
            None => {
                // Constant-time failure simulation to prevent user enumeration
                let _ = verify_password(
                    "dummy_password",
                    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$dummyhash",
                );
                return Err(AuthError::InvalidCredentials);
            }
        };

        let user_id: Uuid = user.get("id");
        let tenant_id: Uuid = user.get("tenant_id");
        let password_hash: String = user.get("password_hash");
        let status: String = user.get("status");

        if status != "ACTIVE" {
            return Err(AuthError::AccountSuspended);
        }

        if !verify_password(&req.password, &password_hash) {
            return Err(AuthError::InvalidCredentials);
        }

        self.rate_limiter.record_success(&rate_key);

        let user_id = UserId::from(user_id);
        let tenant_id = TenantId::from(tenant_id);
        let session_id = SessionId::new();

        // Fetch roles
        let role_rows = sqlx::query(
            "SELECT r.name FROM roles r
             JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = $1 AND ur.tenant_id = $2",
        )
        .bind(user_id.0)
        .bind(tenant_id.0)
        .fetch_all(&self.pool)
        .await?;

        let roles: Vec<String> = role_rows.into_iter().map(|r| r.get("name")).collect();

        // Create refresh token
        let raw_refresh = generate_refresh_token();
        let token_hash = hash_refresh_token(&raw_refresh);
        let expires_at = Utc::now() + Duration::days(30);

        sqlx::query(
            "INSERT INTO sessions (id, tenant_id, user_id, token_hash, expires_at, ip, user_agent)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(session_id.0)
        .bind(tenant_id.0)
        .bind(user_id.0)
        .bind(token_hash)
        .bind(expires_at)
        .bind(ip.as_ref())
        .bind(user_agent.as_ref())
        .execute(&self.pool)
        .await?;

        // Update last login
        sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1")
            .bind(user_id.0)
            .execute(&self.pool)
            .await?;

        // Write audit log
        self.write_audit_log(AuditLogParams {
            tenant_id,
            actor_id: Some(user_id),
            actor_type: "USER",
            entity_type: "USER",
            entity_id: user_id.0,
            action: "LOGIN_SUCCESS",
            before: None,
            after: serde_json::json!({"ip": ip}),
            reason: "User logged in successfully",
            ip,
        })
        .await?;

        let access_token =
            create_access_token(user_id, tenant_id, session_id, roles, &self.jwt_secret)?;

        Ok(AuthTokens {
            access_token,
            refresh_token: raw_refresh,
            token_type: "Bearer".to_string(),
            expires_in: 900, // 15 minutes
        })
    }

    /// Refresh access token with rotation and family revocation on reuse
    pub async fn refresh_tokens(
        &self,
        req: RefreshRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<AuthTokens, AuthError> {
        let token_hash = hash_refresh_token(&req.refresh_token);

        let session = sqlx::query(
            "SELECT id, tenant_id, user_id, expires_at, revoked_at
             FROM sessions
             WHERE token_hash = $1",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let session = match session {
            Some(s) => s,
            None => {
                warn!("Refresh token reuse detected or invalid token!");
                return Err(AuthError::SessionFamilyRevoked);
            }
        };

        let session_id: Uuid = session.get("id");
        let tenant_id: Uuid = session.get("tenant_id");
        let user_id: Uuid = session.get("user_id");
        let expires_at: DateTime<Utc> = session.get("expires_at");
        let revoked_at: Option<DateTime<Utc>> = session.get("revoked_at");

        if revoked_at.is_some() {
            // Token was already rotated or revoked! Re-use detected: revoke all sessions for this user
            warn!(
                "Revoked refresh token reused! Revoking session family for user: {}",
                user_id
            );
            sqlx::query(
                "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(user_id)
            .execute(&self.pool)
            .await?;
            return Err(AuthError::SessionFamilyRevoked);
        }

        if expires_at < Utc::now() {
            return Err(AuthError::InvalidToken);
        }

        let user_id = UserId::from(user_id);
        let tenant_id = TenantId::from(tenant_id);
        let session_id = SessionId::from(session_id);

        // Fetch roles
        let role_rows = sqlx::query(
            "SELECT r.name FROM roles r
             JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = $1 AND ur.tenant_id = $2",
        )
        .bind(user_id.0)
        .bind(tenant_id.0)
        .fetch_all(&self.pool)
        .await?;

        let roles: Vec<String> = role_rows.into_iter().map(|r| r.get("name")).collect();

        // Rotate refresh token
        let new_refresh = generate_refresh_token();
        let new_token_hash = hash_refresh_token(&new_refresh);
        let new_expires_at = Utc::now() + Duration::days(30);

        // Revoke old session and insert new rotated session
        sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1")
            .bind(session_id.0)
            .execute(&self.pool)
            .await?;

        let new_session_id = SessionId::new();
        sqlx::query(
            "INSERT INTO sessions (id, tenant_id, user_id, token_hash, expires_at, ip, user_agent)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(new_session_id.0)
        .bind(tenant_id.0)
        .bind(user_id.0)
        .bind(new_token_hash)
        .bind(new_expires_at)
        .bind(ip.as_ref())
        .bind(user_agent.as_ref())
        .execute(&self.pool)
        .await?;

        let access_token =
            create_access_token(user_id, tenant_id, new_session_id, roles, &self.jwt_secret)?;

        Ok(AuthTokens {
            access_token,
            refresh_token: new_refresh,
            token_type: "Bearer".to_string(),
            expires_in: 900,
        })
    }

    /// Revoke current session on logout
    pub async fn logout(&self, session_id: SessionId, user_id: UserId) -> Result<(), AuthError> {
        sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1 AND user_id = $2")
            .bind(session_id.0)
            .bind(user_id.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Extract and validate TenantContext from a Bearer token
    pub async fn extract_tenant_context(&self, token: &str) -> Result<TenantContext, AuthError> {
        let claims = verify_access_token(token, &self.jwt_secret)?;

        // Verify session not revoked
        let session =
            sqlx::query("SELECT revoked_at FROM sessions WHERE id = $1 AND tenant_id = $2")
                .bind(claims.sid.0)
                .bind(claims.tid.0)
                .fetch_optional(&self.pool)
                .await?;

        match session {
            Some(s) if s.get::<Option<DateTime<Utc>>, _>("revoked_at").is_none() => (),
            _ => return Err(AuthError::SessionRevoked),
        }

        // Fetch assigned branch IDs
        let branch_rows = sqlx::query(
            "SELECT branch_id FROM user_branches WHERE user_id = $1 AND tenant_id = $2",
        )
        .bind(claims.sub.0)
        .bind(claims.tid.0)
        .fetch_all(&self.pool)
        .await?;

        let branch_ids: Vec<BranchId> = branch_rows
            .into_iter()
            .map(|b| BranchId::from(b.get::<Uuid, _>("branch_id")))
            .collect();

        // Fetch distinct permissions from all assigned roles
        let perm_rows = sqlx::query(
            "SELECT DISTINCT p.key FROM permissions p
             JOIN role_permissions rp ON rp.permission_id = p.id
             JOIN user_roles ur ON ur.role_id = rp.role_id
             WHERE ur.user_id = $1 AND ur.tenant_id = $2",
        )
        .bind(claims.sub.0)
        .bind(claims.tid.0)
        .fetch_all(&self.pool)
        .await?;

        let permissions: HashSet<String> = perm_rows
            .into_iter()
            .map(|p| p.get::<String, _>("key"))
            .collect();

        Ok(TenantContext::from_claims(
            claims.tid,
            claims.sub,
            branch_ids,
            permissions,
            claims.roles,
        ))
    }

    /// Seed the 10 system roles and permissions for a tenant
    pub async fn seed_system_roles_for_tenant(&self, tenant_id: TenantId) -> Result<(), AuthError> {
        let system_roles = get_system_role_definitions();

        // 1. Seed all permissions
        for &perm_key in crate::roles::ALL_PERMISSIONS {
            sqlx::query(
                "INSERT INTO permissions (id, tenant_id, key, description)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (tenant_id, key) DO NOTHING",
            )
            .bind(Uuid::now_v7())
            .bind(tenant_id.0)
            .bind(perm_key)
            .bind(format!("System permission: {}", perm_key))
            .execute(&self.pool)
            .await?;
        }

        // 2. Seed system roles & link permissions
        for (role_name, perms) in system_roles {
            let role_id = Uuid::now_v7();
            sqlx::query(
                "INSERT INTO roles (id, tenant_id, name, is_system, description)
                 VALUES ($1, $2, $3, true, $4)
                 ON CONFLICT (tenant_id, name) DO NOTHING",
            )
            .bind(role_id)
            .bind(tenant_id.0)
            .bind(role_name)
            .bind(format!("Built-in system role: {}", role_name))
            .execute(&self.pool)
            .await?;

            let existing_role =
                sqlx::query("SELECT id FROM roles WHERE tenant_id = $1 AND name = $2")
                    .bind(tenant_id.0)
                    .bind(role_name)
                    .fetch_one(&self.pool)
                    .await?;

            let existing_role_id: Uuid = existing_role.get("id");

            for perm_key in perms {
                let perm =
                    sqlx::query("SELECT id FROM permissions WHERE tenant_id = $1 AND key = $2")
                        .bind(tenant_id.0)
                        .bind(perm_key)
                        .fetch_one(&self.pool)
                        .await?;

                let perm_id: Uuid = perm.get("id");

                sqlx::query(
                    "INSERT INTO role_permissions (tenant_id, role_id, permission_id)
                     VALUES ($1, $2, $3)
                     ON CONFLICT DO NOTHING",
                )
                .bind(tenant_id.0)
                .bind(existing_role_id)
                .bind(perm_id)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Audit log writer helper per Invariant I-9
    pub async fn write_audit_log(&self, params: AuditLogParams) -> Result<(), AuthError> {
        sqlx::query(
            "INSERT INTO audit_log (tenant_id, actor_id, actor_type, entity_type, entity_id, action, before, after, reason, ip)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(params.tenant_id.0)
        .bind(params.actor_id.map(|a| a.0))
        .bind(params.actor_type)
        .bind(params.entity_type)
        .bind(params.entity_id)
        .bind(params.action)
        .bind(params.before)
        .bind(params.after)
        .bind(params.reason)
        .bind(params.ip)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
