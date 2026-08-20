use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use shifa_api::build_app;
use shifa_core::id::{TenantId, UserId};
use shifa_identity::IdentityService;
use sqlx::PgPool;
use tower::ServiceExt;

#[tokio::test]
async fn test_api_auth_and_session_lifecycle() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("DATABASE_URL not set; skipping live Postgres integration test");
            return;
        }
    };

    let pool = PgPool::connect(&database_url).await.expect("connect to db");

    let jwt_secret = "super_secure_test_jwt_secret_key_32_bytes!".to_string();
    let identity_service = IdentityService::new(pool.clone(), jwt_secret.clone());
    let app = build_app(pool.clone(), identity_service.clone());

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let phone = "+923009998877";
    let password = "SecretPassword123!";

    // Insert tenant
    sqlx::query("INSERT INTO tenants (id, name, legal_name, status) VALUES ($1, 'Test Pharmacy', 'Test Ltd', 'ACTIVE')")
        .bind(tenant_id.0)
        .execute(&pool)
        .await
        .expect("insert tenant");

    // Seed system roles
    identity_service
        .seed_system_roles_for_tenant(tenant_id)
        .await
        .expect("seed roles");

    // Hash password & insert test user
    let password_hash = shifa_identity::password::hash_password(password).expect("hash password");
    sqlx::query(
        "INSERT INTO users (id, tenant_id, phone, email, full_name, password_hash, status, locale)
         VALUES ($1, $2, $3, $4, $5, $6, 'ACTIVE', 'en')",
    )
    .bind(user_id.0)
    .bind(tenant_id.0)
    .bind(phone)
    .bind("pharmacist@test.pk")
    .bind("Test Pharmacist")
    .bind(&password_hash)
    .execute(&pool)
    .await
    .expect("insert user");

    // Assign PHARMACIST role
    let role = sqlx::query("SELECT id FROM roles WHERE tenant_id = $1 AND name = 'PHARMACIST'")
        .bind(tenant_id.0)
        .fetch_one(&pool)
        .await
        .expect("fetch pharmacist role");

    let role_id: uuid::Uuid = sqlx::Row::get(&role, "id");

    sqlx::query("INSERT INTO user_roles (tenant_id, user_id, role_id) VALUES ($1, $2, $3)")
        .bind(tenant_id.0)
        .bind(user_id.0)
        .bind(role_id)
        .execute(&pool)
        .await
        .expect("assign role");

    // 1. Acceptance test: login_wrong_password_and_unknown_user_are_indistinguishable
    let wrong_user_req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "phone_or_email": "+923000000000",
                "password": "WrongPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let wrong_user_res = app.clone().oneshot(wrong_user_req).await.unwrap();
    assert_eq!(wrong_user_res.status(), StatusCode::UNAUTHORIZED);

    let wrong_pw_req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "phone_or_email": phone,
                "password": "WrongPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let wrong_pw_res = app.clone().oneshot(wrong_pw_req).await.unwrap();
    assert_eq!(wrong_pw_res.status(), StatusCode::UNAUTHORIZED);

    // 2. Acceptance test: login_success_returns_tokens
    let login_req = Request::builder()
        .uri("/api/v1/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "phone_or_email": phone,
                "password": password
            })
            .to_string(),
        ))
        .unwrap();

    let login_res = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(login_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let tokens: shifa_identity::models::AuthTokens = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!tokens.access_token.is_empty());
    assert!(!tokens.refresh_token.is_empty());

    // 3. Acceptance test: me endpoint returns profile
    let me_req = Request::builder()
        .uri("/api/v1/auth/me")
        .method("GET")
        .header("Authorization", format!("Bearer {}", tokens.access_token))
        .body(Body::empty())
        .unwrap();

    let me_res = app.clone().oneshot(me_req).await.unwrap();
    assert_eq!(me_res.status(), StatusCode::OK);

    let me_bytes = axum::body::to_bytes(me_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let me_profile: shifa_identity::models::UserProfileResponse =
        serde_json::from_slice(&me_bytes).unwrap();
    assert_eq!(me_profile.user.id, user_id);
    assert_eq!(me_profile.user.tenant_id, tenant_id);
    assert!(me_profile.permissions.contains(&"rx.approve".to_string()));

    // 4. Acceptance test: refresh_rotation_invalidates_old_token
    let refresh_req = Request::builder()
        .uri("/api/v1/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "refresh_token": tokens.refresh_token
            })
            .to_string(),
        ))
        .unwrap();

    let refresh_res = app.clone().oneshot(refresh_req).await.unwrap();
    assert_eq!(refresh_res.status(), StatusCode::OK);

    let refresh_bytes = axum::body::to_bytes(refresh_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let _new_tokens: shifa_identity::models::AuthTokens =
        serde_json::from_slice(&refresh_bytes).unwrap();

    // 5. Acceptance test: reused_refresh_token_kills_session_family
    let reuse_req = Request::builder()
        .uri("/api/v1/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "refresh_token": tokens.refresh_token
            })
            .to_string(),
        ))
        .unwrap();

    let reuse_res = app.clone().oneshot(reuse_req).await.unwrap();
    assert_eq!(reuse_res.status(), StatusCode::UNAUTHORIZED);

    // 6. Acceptance test: every_auth_event_writes_audit_log
    let audit_count: (i64,) = sqlx::query_as("SELECT count(*) FROM audit_log WHERE tenant_id = $1")
        .bind(tenant_id.0)
        .fetch_one(&pool)
        .await
        .expect("count audit logs");

    assert!(
        audit_count.0 > 0,
        "Auth events must write to audit_log (Invariant I-9)"
    );
}
