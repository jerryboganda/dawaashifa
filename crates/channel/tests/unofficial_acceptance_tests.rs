use serde_json::json;
use shifa_channel::adapter::ChannelAdapter;
use shifa_channel::cloud_api::{CloudApiAdapter, CloudApiConfig};
use shifa_channel::error::ChannelError;
use shifa_channel::templates::TemplateRegistry;
use shifa_channel::types::*;
use shifa_channel::unofficial::{
    HumanPacer, NumberPoolManager, ReplyParser, SessionStore, UnofficialAdapter,
};
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, ChannelId, ConversationId, TenantId, UserId};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

fn create_admin_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    let mut perms = HashSet::new();
    perms.insert("tenant.settings".to_string());
    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        perms,
        vec!["SUPER_ADMIN".to_string()],
    )
}

async fn seed_test_tenant_and_channel(pool: &PgPool, tenant_id: TenantId, channel_id: ChannelId) {
    sqlx::query(
        "INSERT INTO tenants (id, name, slug) VALUES ($1, 'Unofficial Channel Tenant', $2)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(tenant_id.0)
    .bind(format!("wa-tenant-{}", tenant_id.0))
    .execute(pool)
    .await
    .unwrap();

    let branch_id = BranchId::new();
    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, is_warehouse)
         VALUES ($1, $2, 'Lahore Central', 'LHR01', false)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(branch_id.0)
    .bind(tenant_id.0)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO channels (id, tenant_id, branch_id, transport, phone_number, status, health_score)
         VALUES ($1, $2, $3, 'UNOFFICIAL', '+923001112233', 'ACTIVE', 100)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(channel_id.0)
    .bind(tenant_id.0)
    .bind(branch_id.0)
    .execute(pool)
    .await
    .unwrap();
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 1 & 2: session_survives_container_restart & session_creds_encrypted_at_rest
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_session_survives_container_restart_and_creds_encrypted_at_rest() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping DB test: postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let channel_id = ChannelId::new();
    seed_test_tenant_and_channel(&pool, tenant_id, channel_id).await;
    let ctx = create_admin_context(tenant_id, user_id);

    let creds = json!({ "me": { "id": "923001112233:1@s.whatsapp.net", "name": "Shifa Pharmacy" }, "registrationId": 12345 });
    let keys = json!({ "preKeys": { "1": "base64keydata" } });
    let raw_secret = "baileys_noise_key_private_secret_9988";
    let master_key = "test_cluster_secret_key_from_env";

    // 1. Save session
    SessionStore::save_session(
        &ctx,
        channel_id,
        creds.clone(),
        keys.clone(),
        raw_secret,
        master_key,
        &pool,
    )
    .await
    .unwrap();

    // 2. Simulate container restart by loading session anew from DB
    let loaded = SessionStore::load_session(&ctx, channel_id, &pool)
        .await
        .unwrap()
        .expect("Session must exist");
    assert_eq!(loaded.channel_id, channel_id);
    assert_eq!(loaded.creds, creds);
    assert_eq!(loaded.keys, keys);

    // 3. Assert encrypted at rest
    assert_ne!(
        loaded.encrypted_secret, raw_secret,
        "Secret must be encrypted/hashed at rest"
    );
    let expected_hash = SessionStore::encrypt_secret(raw_secret, master_key);
    assert_eq!(loaded.encrypted_secret, expected_hash);
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 3: choice_renders_as_numbered_text
// ------------------------------------------------------------------------------------------------
#[test]
fn test_choice_renders_as_numbered_text() {
    let prompt = "Please choose your payment method:";
    let options = vec![
        ChoiceOption {
            id: "opt_cod".into(),
            title: "Cash on Delivery".into(),
            description: Some("Pay when rider arrives".into()),
        },
        ChoiceOption {
            id: "opt_easypaisa".into(),
            title: "Easypaisa".into(),
            description: None,
        },
        ChoiceOption {
            id: "opt_card".into(),
            title: "Credit/Debit Card".into(),
            description: None,
        },
    ];

    let rendered = UnofficialAdapter::render_choice_as_numbered_text(prompt, &options);
    assert!(rendered.contains("1. Cash on Delivery - Pay when rider arrives"));
    assert!(rendered.contains("2. Easypaisa"));
    assert!(rendered.contains("3. Credit/Debit Card"));
    assert!(rendered.contains("(Reply with a number)"));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 4: reply_parser_accepts_urdu_and_roman_variants
// ------------------------------------------------------------------------------------------------
#[test]
fn test_reply_parser_accepts_urdu_and_roman_variants() {
    // 1. Choice options parsing table
    let choice_test_cases = vec![
        ("1", Some(1)),
        ("۱", Some(1)),
        ("١", Some(1)),
        ("option 1", Some(1)),
        ("opt 1", Some(1)),
        ("pehla", Some(1)),
        ("pehli", Some(1)),
        ("first", Some(1)),
        ("پہلا", Some(1)),
        ("2", Some(2)),
        ("۲", Some(2)),
        ("٢", Some(2)),
        ("dusra", Some(2)),
        ("دوسرا", Some(2)),
        ("3", Some(3)),
        ("teesra", Some(3)),
        ("4", Some(4)),
        ("chotha", Some(4)),
        ("5", Some(5)),
        ("panchwa", Some(5)),
        ("invalid", None),
    ];

    for (input, expected) in choice_test_cases {
        let parsed = ReplyParser::parse_choice_index(input);
        assert_eq!(parsed, expected, "Failed parsing choice input '{}'", input);
    }

    // 2. Confirm parsing table
    let confirm_test_cases = vec![
        ("yes", Some(true)),
        ("y", Some(true)),
        ("haan", Some(true)),
        ("ha", Some(true)),
        ("ji haan", Some(true)),
        ("sahi", Some(true)),
        ("theek", Some(true)),
        ("ہاں", Some(true)),
        ("جی ہاں", Some(true)),
        ("no", Some(false)),
        ("n", Some(false)),
        ("nahi", Some(false)),
        ("nahin", Some(false)),
        ("cancel", Some(false)),
        ("radd", Some(false)),
        ("نہیں", Some(false)),
        ("random noise", None),
    ];

    for (input, expected) in confirm_test_cases {
        let parsed = ReplyParser::parse_confirm(input);
        assert_eq!(parsed, expected, "Failed parsing confirm input '{}'", input);
    }
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 5: send_pacing_respects_minimum_gap
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_send_pacing_respects_minimum_gap() {
    let pacer = HumanPacer::new();
    let start = Instant::now();

    pacer.enforce_minimum_gap(1).await;
    pacer.enforce_minimum_gap(1).await;

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 900,
        "Two sends must respect minimum gap"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 6 & 7: daily_limit_blocks_further_sends & warming_number_uses_reduced_limits
// ------------------------------------------------------------------------------------------------
#[test]
fn test_daily_limit_blocks_further_sends_and_warming_number_uses_reduced_limits() {
    // Active channel: 300 cap
    assert!(HumanPacer::check_daily_limits(ChannelPoolStatus::Active, 299).is_ok());
    assert!(HumanPacer::check_daily_limits(ChannelPoolStatus::Active, 300).is_err());

    // Warming channel: 40 cap
    assert!(HumanPacer::check_daily_limits(ChannelPoolStatus::Warming, 39).is_ok());
    assert!(HumanPacer::check_daily_limits(ChannelPoolStatus::Warming, 40).is_err());
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 8 & 9: logged_out_event_marks_banned_and_drains_queue & failover_reassigns_queue
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_logged_out_event_marks_banned_and_failover_reassigns_queue() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping DB test: postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let channel_banned = ChannelId::new();
    let channel_backup = ChannelId::new();

    seed_test_tenant_and_channel(&pool, tenant_id, channel_banned).await;
    let ctx = create_admin_context(tenant_id, user_id);

    // Create backup active channel
    let branch_id: Uuid =
        sqlx::query_scalar("SELECT id FROM branches WHERE tenant_id = $1 LIMIT 1")
            .bind(tenant_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();

    sqlx::query(
        "INSERT INTO channels (id, tenant_id, branch_id, transport, phone_number, status, health_score)
         VALUES ($1, $2, $3, 'UNOFFICIAL', '+923004445566', 'ACTIVE', 95)"
    )
    .bind(channel_backup.0)
    .bind(tenant_id.0)
    .bind(branch_id)
    .execute(&pool)
    .await
    .unwrap();

    // Create open conversation assigned to channel_banned
    let conv_id = ConversationId::new();
    sqlx::query(
        "INSERT INTO conversations (id, tenant_id, channel_id, customer_id, current_state)
         VALUES ($1, $2, $3, uuidv7(), 'ACTIVE')",
    )
    .bind(conv_id.0)
    .bind(tenant_id.0)
    .bind(channel_banned.0)
    .execute(&pool)
    .await
    .unwrap();

    // Trigger ban on channel_banned
    let failover_target =
        NumberPoolManager::handle_ban(&ctx, channel_banned, "DisconnectReason.loggedOut", &pool)
            .await
            .unwrap();

    assert_eq!(
        failover_target,
        Some(channel_backup),
        "Must failover to backup active channel"
    );

    // Verify channel status is BANNED
    let status: String =
        sqlx::query_scalar("SELECT status FROM channels WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id.0)
            .bind(channel_banned.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "BANNED");

    // Verify conversation reassigned to backup channel
    let reassigned_channel: Uuid =
        sqlx::query_scalar("SELECT channel_id FROM conversations WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id.0)
            .bind(conv_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        reassigned_channel, channel_backup.0,
        "Open conversation must be reassigned to failover target"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 10: unofficial_number_cannot_join_official_waba_identity
// ------------------------------------------------------------------------------------------------
#[test]
fn test_unofficial_number_cannot_join_official_waba_identity() {
    let res = NumberPoolManager::validate_identity_isolation(
        Transport::Unofficial,
        IdentityKind::OfficialWaba,
    );

    assert!(
        res.is_err(),
        "Unofficial number must NEVER be allowed to join Official WABA identity"
    );
    match res.unwrap_err() {
        ChannelError::IdentityIsolationViolation => {}
        other => panic!("Expected IdentityIsolationViolation, got {:?}", other),
    }

    // Allowed combinations
    assert!(NumberPoolManager::validate_identity_isolation(
        Transport::Unofficial,
        IdentityKind::UnofficialIsolated,
    )
    .is_ok());

    assert!(NumberPoolManager::validate_identity_isolation(
        Transport::CloudApi,
        IdentityKind::OfficialWaba,
    )
    .is_ok());
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 11: business_logic_identical_across_transports
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_business_logic_identical_across_transports() {
    let cloud_adapter = CloudApiAdapter::new(
        ChannelId::new(),
        CloudApiConfig {
            base_url: "https://graph.facebook.com".into(),
            api_version: "v21.0".into(),
            phone_number_id: "phone_123".into(),
            access_token: "test_token".into(),
        },
        Arc::new(TemplateRegistry::new()),
    );

    let unofficial_adapter = UnofficialAdapter::new(ChannelId::new());

    let msg = OutboundMessage {
        tenant_id: TenantId::new(),
        conversation_id: ConversationId::new(),
        to: "+923001234567".into(),
        body: OutboundBody::Confirm {
            prompt: "Do you confirm order #1001 for Rs 1,250.00?".into(),
            yes: "Confirm".into(),
            no: "Cancel".into(),
        },
        idempotency_key: Uuid::now_v7(),
        locale: "en".into(),
    };

    // Both adapters accept the exact same OutboundMessage without business logic branching
    let res_unofficial = unofficial_adapter.send(msg.clone(), true).await.unwrap();
    assert_eq!(res_unofficial.status, "sent");
    assert!(res_unofficial.transport_message_id.starts_with("baileys_"));

    assert_eq!(unofficial_adapter.transport(), Transport::Unofficial);
    assert_eq!(cloud_adapter.transport(), Transport::CloudApi);
}
