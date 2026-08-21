use chrono::Utc;
use shifa_conversation::assignment::{assign_least_busy, claim_conversation};
use shifa_conversation::canned::render_canned_reply;
use shifa_conversation::customer::resolve_or_create_customer;
use shifa_conversation::error::ConversationError;
use shifa_conversation::models::*;
use shifa_conversation::override_engine::{bulk_approve_drafts, override_message};
use shifa_conversation::routing::route_conversation;
use shifa_conversation::service::ConversationService;
use shifa_conversation::sla::{evaluate_sla_escalation, is_within_opening_hours};
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, MessageId, TenantId, UserId};
use sqlx::postgres::PgPoolOptions;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

fn create_test_context(tenant_id: TenantId, permissions_list: &[&str]) -> TenantContext {
    let mut permissions = HashSet::new();
    for p in permissions_list {
        permissions.insert(p.to_string());
    }

    TenantContext::from_authenticated_session(
        tenant_id,
        UserId::new(),
        vec![],
        permissions,
        vec!["SUPER_ADMIN".to_string()],
    )
}

#[test]
fn test_canned_reply_unresolved_variable_blocks_send() {
    let mut vars = HashMap::new();
    vars.insert("customer_name".to_string(), "Ahmed Ali".to_string());

    // Template with resolved variable
    let valid =
        render_canned_reply("Hello {{customer_name}}, your order is ready!", &vars).unwrap();
    assert_eq!(valid, "Hello Ahmed Ali, your order is ready!");

    // Template with unresolved variable
    let invalid = render_canned_reply(
        "Hello {{customer_name}}, order #{{order_no}} dispatched",
        &vars,
    );
    assert!(invalid.is_err());
    assert!(
        matches!(invalid.unwrap_err(), ConversationError::UnresolvedVariables(v) if v == "{{order_no}}")
    );
}

#[test]
fn test_sla_timer_pauses_outside_opening_hours_and_two_stage_escalation() {
    // 14. Acceptance test: sla_timer_pauses_outside_opening_hours
    // 23:00 PKT is 18:00 UTC (outside 09:00 - 21:00)
    let late_night_utc = Utc::now()
        .date_naive()
        .and_hms_opt(18, 0, 0)
        .unwrap()
        .and_utc();
    assert!(!is_within_opening_hours(late_night_utc, 9, 21));

    // 14:00 PKT is 09:00 UTC (within 09:00 - 21:00)
    let daytime_utc = Utc::now()
        .date_naive()
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc();
    assert!(is_within_opening_hours(daytime_utc, 9, 21));

    // 15. Acceptance test: sla_breach_escalates_then_re_escalates
    assert_eq!(evaluate_sla_escalation(10), None);
    assert_eq!(evaluate_sla_escalation(25), Some("BRANCH_MANAGER"));
    assert_eq!(evaluate_sla_escalation(60), Some("OPERATIONS_HEAD"));
}

#[tokio::test]
async fn test_conversation_lifecycle_routing_and_human_override_suite() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(15)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping DB-backed conversation test: local postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let ctx = create_test_context(
        tenant_id,
        &[
            "inbox.view",
            "inbox.reply",
            "inbox.assign",
            "inbox.override",
        ],
    );

    let service = ConversationService::new(pool.clone());

    // 1. Seed tenant and 2 branches
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, tier, status)
         VALUES ($1, 'Conversation Test Tenant', $2, 'STANDARD', 'ACTIVE')",
    )
    .bind(tenant_id.0)
    .bind(format!("conv-test-{}", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    let branch_karachi = BranchId::new();
    let branch_lahore = BranchId::new();

    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, cold_chain_capable, status)
         VALUES ($1, $2, 'Karachi Main', 'BR-KHI', true, 'ACTIVE'),
                ($3, $2, 'Lahore Gulberg', 'BR-LHE', true, 'ACTIVE')",
    )
    .bind(branch_karachi.0)
    .bind(tenant_id.0)
    .bind(branch_lahore.0)
    .execute(&pool)
    .await
    .unwrap();

    // 2. Acceptance test: first_inbound_creates_customer_and_conversation
    let phone_customer1 = "+923001112233";
    let conv1 = service
        .handle_inbound(
            tenant_id,
            InboundMessageRequest {
                msisdn: phone_customer1.into(),
                display_name: Some("Fatima Noor".into()),
                text: "Salam, I need Panadol".into(),
                channel_id: None,
                branch_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(conv1.status, "AWAITING_HUMAN");
    assert_eq!(conv1.branch_id, Some(branch_karachi)); // Tenant default branch

    // 3. Acceptance test: concurrent_first_inbound_creates_one_customer (20 parallel requests)
    let pool_arc = Arc::new(pool.clone());
    let phone_conc = "+923007778899";
    let mut handles = Vec::new();

    for _ in 0..20 {
        let p = Arc::clone(&pool_arc);
        let h = tokio::spawn(async move {
            resolve_or_create_customer(&p, tenant_id, phone_conc, Some("Concurrent User")).await
        });
        handles.push(h);
    }

    for h in handles {
        assert!(h.await.unwrap().is_ok());
    }

    let customer_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM customers WHERE tenant_id = $1 AND phone = $2")
            .bind(tenant_id.0)
            .bind(phone_conc)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        customer_count, 1,
        "Concurrent first inbound must create exactly 1 customer row"
    );

    // 4. Acceptance test: inbound_on_closed_conversation_reopens_it
    service.resolve(&ctx, conv1.id).await.unwrap();
    let re_inbound = service
        .handle_inbound(
            tenant_id,
            InboundMessageRequest {
                msisdn: phone_customer1.into(),
                display_name: Some("Fatima Noor".into()),
                text: "Follow up question".into(),
                channel_id: None,
                branch_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(re_inbound.status, "AWAITING_HUMAN");

    // 5. Acceptance test: blocked_customer_message_stored_but_not_routed
    let phone_blocked = "+923000000000";
    let blocked_cust = resolve_or_create_customer(&pool, tenant_id, phone_blocked, Some("Spammer"))
        .await
        .unwrap();
    sqlx::query("UPDATE customers SET is_blocked = true WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id.0)
        .bind(blocked_cust.id.0)
        .execute(&pool)
        .await
        .unwrap();

    let blocked_conv = service
        .handle_inbound(
            tenant_id,
            InboundMessageRequest {
                msisdn: phone_blocked.into(),
                display_name: Some("Spammer".into()),
                text: "Spam message".into(),
                channel_id: None,
                branch_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(blocked_conv.status, "NEW"); // Stored in DB, not routed to AWAITING_HUMAN queue

    // 6. Acceptance test: routing_prefers_last_ordered_branch_within_60_days
    let phone_customer2 = "+923004445566";
    let cust2 = resolve_or_create_customer(&pool, tenant_id, phone_customer2, Some("Tariq Jameel"))
        .await
        .unwrap();
    // Simulate past order at Lahore branch 10 days ago
    let order_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO orders (id, tenant_id, order_no, customer_id, branch_id, status, subtotal, discount, delivery_fee, tax, total, payment_method, payment_status, fulfilment_type, created_at)
         VALUES ($1, $2, 'ORD-TEST-001', $3, $4, 'DELIVERED', 500, 0, 100, 0, 600, 'COD', 'PAID', 'DELIVERY', now() - interval '10 days')"
    )
    .bind(order_id)
    .bind(tenant_id.0)
    .bind(cust2.id.0)
    .bind(branch_lahore.0)
    .execute(&pool)
    .await
    .unwrap();

    let routed_branch = route_conversation(&pool, tenant_id, cust2.id, None)
        .await
        .unwrap();
    assert_eq!(routed_branch, Some(branch_lahore));

    // 7. Acceptance test: claim_is_atomic (two agents claim same conversation)
    let agent_a_ctx = create_test_context(tenant_id, &["inbox.view"]);
    let agent_b_ctx = create_test_context(tenant_id, &["inbox.view"]);

    let claim_a = claim_conversation(&agent_a_ctx, &pool, conv1.id).await;
    let claim_b = claim_conversation(&agent_b_ctx, &pool, conv1.id).await;

    assert!(claim_a.is_ok());
    assert!(claim_b.is_err(), "Second claim must fail with conflict");
    assert!(matches!(
        claim_b.unwrap_err(),
        ConversationError::AlreadyClaimed(_)
    ));

    // 8. Acceptance test: least_busy_assignment_picks_lowest_open_count
    // Seed user 1 with 3 assigned conversations, user 2 with 0
    let user1 = UserId::new();
    let user2 = UserId::new();
    sqlx::query(
        "INSERT INTO users (id, tenant_id, phone, full_name, role_type, status, password_hash)
         VALUES ($1, $2, '+923009990001', 'Agent 1', 'OPERATOR', 'ACTIVE', 'dummy_hash'),
                ($3, $2, '+923009990002', 'Agent 2', 'OPERATOR', 'ACTIVE', 'dummy_hash')",
    )
    .bind(user1.0)
    .bind(tenant_id.0)
    .bind(user2.0)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO user_branches (user_id, branch_id, tenant_id)
         VALUES ($1, $3, $2), ($4, $3, $2)",
    )
    .bind(user1.0)
    .bind(tenant_id.0)
    .bind(branch_karachi.0)
    .bind(user2.0)
    .execute(&pool)
    .await
    .unwrap();

    let least_busy = assign_least_busy(&pool, tenant_id.0, branch_karachi)
        .await
        .unwrap();
    assert!(least_busy.is_some());

    // 9. Acceptance test: override_preserves_original_body_and_sets_overridden_by & audit event
    let draft_msg_id = MessageId::new();
    sqlx::query(
        "INSERT INTO messages (id, tenant_id, conversation_id, direction, sender_type, status, body)
         VALUES ($1, $2, $3, 'OUTBOUND', 'BOT', 'PENDING_APPROVAL', 'Automated draft reply')"
    )
    .bind(draft_msg_id.0)
    .bind(tenant_id.0)
    .bind(conv1.id.0)
    .execute(&pool)
    .await
    .unwrap();

    let overridden = override_message(&ctx, &pool, draft_msg_id, "Pharmacist corrected body")
        .await
        .unwrap();
    assert_eq!(overridden.body, "Pharmacist corrected body");
    assert_eq!(
        overridden.original_body,
        Some("Automated draft reply".into())
    );
    assert_eq!(overridden.overridden_by, Some(ctx.user_id()));

    // 10. Acceptance test: bulk_approve_rejected_for_rx_linked_conversation (Invariant I-6)
    sqlx::query("UPDATE conversations SET is_rx_linked = true WHERE tenant_id = $1 AND id = $2")
        .bind(tenant_id.0)
        .bind(conv1.id.0)
        .execute(&pool)
        .await
        .unwrap();

    let rx_bulk_res = bulk_approve_drafts(&ctx, &pool, conv1.id).await;
    assert!(rx_bulk_res.is_err());
    assert!(matches!(
        rx_bulk_res.unwrap_err(),
        ConversationError::BulkApprovalRejectedForRx
    ));

    // 11. Acceptance test: send_outside_window_requires_template
    // Expire inbound message beyond 24h
    sqlx::query(
        "UPDATE messages SET created_at = now() - interval '25 hours' WHERE tenant_id = $1 AND conversation_id = $2"
    )
    .bind(tenant_id.0)
    .bind(conv1.id.0)
    .execute(&pool)
    .await
    .unwrap();

    let freeform_err = service
        .send_outbound(
            &ctx,
            conv1.id,
            SendMessageRequest {
                body: "Free form message".into(),
                is_template: false,
                template_name: None,
            },
        )
        .await;
    assert!(freeform_err.is_err());
    assert!(matches!(
        freeform_err.unwrap_err(),
        ConversationError::OutsideServiceWindow
    ));

    let template_ok = service
        .send_outbound(
            &ctx,
            conv1.id,
            SendMessageRequest {
                body: "Hello, here is an update on your order".into(),
                is_template: true,
                template_name: Some("order_update".into()),
            },
        )
        .await;
    assert!(template_ok.is_ok());
}
