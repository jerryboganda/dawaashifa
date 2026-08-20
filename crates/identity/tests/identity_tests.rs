use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, SessionId, TenantId, UserId};
use shifa_identity::jwt::{
    create_access_token, generate_refresh_token, hash_refresh_token, verify_access_token,
};
use shifa_identity::password::{hash_password, validate_password_strength, verify_password};
use shifa_identity::roles::get_system_role_definitions;
use shifa_identity::service::LoginRateLimiter;
use std::collections::HashSet;

#[test]
fn test_argon2id_hashing_and_complexity() {
    let raw = "ValidStrongPassword123!";
    let hash = hash_password(raw).expect("hash password");
    assert!(verify_password(raw, &hash));
    assert!(!verify_password("InvalidStrongPassword123!", &hash));

    // Short password < 10 chars
    assert!(validate_password_strength("short123").is_err());
    // Common password
    assert!(validate_password_strength("password123").is_err());
}

#[test]
fn test_jwt_access_and_refresh_tokens() {
    let user_id = UserId::new();
    let tenant_id = TenantId::new();
    let session_id = SessionId::new();
    let roles = vec!["PHARMACIST".to_string()];
    let secret = "a_very_secret_jwt_key_at_least_32_bytes_long!";

    let token = create_access_token(user_id, tenant_id, session_id, roles.clone(), secret)
        .expect("create access token");
    let claims = verify_access_token(&token, secret).expect("verify access token");

    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.tid, tenant_id);
    assert_eq!(claims.sid, session_id);
    assert_eq!(claims.roles, roles);

    let refresh = generate_refresh_token();
    assert_eq!(refresh.len(), 64); // 32 bytes hex encoded
    let hash1 = hash_refresh_token(&refresh);
    let hash2 = hash_refresh_token(&refresh);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_only_pharmacist_and_super_admin_have_rx_approve() {
    let roles = get_system_role_definitions();
    assert_eq!(roles.len(), 10, "Must have exactly 10 seeded system roles");

    for (role_name, perms) in roles {
        let has_rx_approve = perms.contains(&"rx.approve");
        if role_name == "PHARMACIST" || role_name == "SUPER_ADMIN" {
            assert!(
                has_rx_approve,
                "Role {} must have rx.approve permission",
                role_name
            );
        } else {
            assert!(
                !has_rx_approve,
                "Role {} must NOT hold rx.approve permission (violates Invariant I-3)",
                role_name
            );
        }
    }
}

#[test]
fn test_login_rate_limiting() {
    let limiter = LoginRateLimiter::new();
    let key = "login:+923001234567:127.0.0.1";

    // 5 attempts allowed
    for _ in 0..5 {
        assert!(limiter.check_and_record_failure(key).is_ok());
    }

    // 6th attempt rejected
    assert!(limiter.check_and_record_failure(key).is_err());

    // Record success resets
    limiter.record_success(key);
    assert!(limiter.check_and_record_failure(key).is_ok());
}

#[test]
fn test_branch_scoped_access_enforcement() {
    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let branch_a = BranchId::new();
    let branch_b = BranchId::new();

    let ctx = TenantContext::from_claims(
        tenant_id,
        user_id,
        vec![branch_a],
        HashSet::from(["order.view".to_string(), "branch.view".to_string()]),
        vec!["BRANCH_MANAGER".to_string()],
    );

    assert!(ctx.can_act_on_branch(branch_a));
    assert!(!ctx.can_act_on_branch(branch_b));
    assert!(ctx.require_branch(branch_a).is_ok());
    assert!(ctx.require_branch(branch_b).is_err());
}
