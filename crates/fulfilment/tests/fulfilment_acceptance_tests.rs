use chrono::{Duration, Utc};
use shifa_core::context::TenantContext;
use shifa_core::id::{
    BranchId, CustomerId, OrderId, ProductId, RiderCashSessionId, RiderId, TenantId, UserId,
};
use shifa_core::money::Money;
use shifa_fulfilment::models::*;
use shifa_fulfilment::service::FulfilmentService;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

fn create_admin_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    let mut perms = HashSet::new();
    perms.insert("user.create".to_string());
    perms.insert("order.edit".to_string());
    perms.insert("payment.approve".to_string());
    perms.insert("report.view".to_string());

    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        perms,
        vec!["SUPER_ADMIN".to_string()],
    )
}

fn create_rider_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        HashSet::new(),
        vec!["RIDER".to_string()],
    )
}

async fn seed_test_tenant_and_branch(pool: &PgPool, tenant_id: TenantId, branch_id: BranchId) {
    sqlx::query(
        "INSERT INTO tenants (id, name, slug) VALUES ($1, 'Fulfilment Test Pharmacy', $2)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(tenant_id.0)
    .bind(format!("fulfil-test-{}", tenant_id.0))
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, is_warehouse)
         VALUES ($1, $2, 'Lahore Gulberg Branch', $3, false)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(branch_id.0)
    .bind(tenant_id.0)
    .bind(format!("LHR-{}", &branch_id.0.to_string()[..6]))
    .execute(pool)
    .await
    .unwrap();
}

#[allow(clippy::too_many_arguments)]
async fn seed_test_order(
    pool: &PgPool,
    tenant_id: TenantId,
    branch_id: BranchId,
    customer_id: CustomerId,
    order_id: OrderId,
    amount: Money,
    is_cod: bool,
    is_controlled: bool,
) {
    sqlx::query(
        "INSERT INTO customers (id, tenant_id, phone, full_name, is_blocked)
         VALUES ($1, $2, $3, 'Ali Raza', false)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(customer_id.0)
    .bind(tenant_id.0)
    .bind(format!(
        "+92300{}",
        &customer_id.0.to_string().replace('-', "")[..7]
    ))
    .execute(pool)
    .await
    .unwrap();

    let pay_method = if is_cod { "COD" } else { "JAZZCASH" };
    sqlx::query(
        "INSERT INTO orders (id, tenant_id, branch_id, customer_id, status, subtotal, discount, delivery_fee, tax, total_amount, payment_method, total_price)
         VALUES ($1, $2, $3, $4, 'AWAITING_FULFILMENT'::order_status, $5, 0.0000, 100.0000, 0.0000, $5, $6, $5)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(order_id.0)
    .bind(tenant_id.0)
    .bind(branch_id.0)
    .bind(customer_id.0)
    .bind(amount.0)
    .bind(pay_method)
    .execute(pool)
    .await
    .unwrap();

    if is_controlled {
        let product_id = ProductId::new();
        sqlx::query(
            "INSERT INTO products (id, tenant_id, name, slug, form, strength, mrp, is_prescription_only, is_controlled)
             VALUES ($1, $2, 'Alprazolam 0.5mg', $3, 'TABLET', '0.5mg', 250.0000, true, true)
             ON CONFLICT (id) DO NOTHING"
        )
        .bind(product_id.0)
        .bind(tenant_id.0)
        .bind(format!("alp-{}", product_id.0))
        .execute(pool)
        .await
        .unwrap();

        let item_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO order_items (id, tenant_id, order_id, product_id, quantity, unit_price, total_price, mrp_at_sale)
             VALUES ($1, $2, $3, $4, 1, 250.0000, 250.0000, 250.0000)
             ON CONFLICT (id) DO NOTHING"
        )
        .bind(item_id)
        .bind(tenant_id.0)
        .bind(order_id.0)
        .bind(product_id.0)
        .execute(pool)
        .await
        .unwrap();
    }
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 1: rider_token_cannot_read_other_riders_deliveries
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_rider_token_cannot_read_other_riders_deliveries() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();
    let rider1_user_id = UserId::new();
    let rider2_user_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
        false,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool.clone());

    // Register 2 riders
    let rider1 = service
        .create_rider(
            &admin_ctx,
            CreateRiderRequest {
                branch_id,
                user_id: rider1_user_id,
                vehicle_type: Some("MOTORBIKE".into()),
                cnic: "35201-1234567-1".into(),
                licence_no: "LHR-12345".into(),
            },
        )
        .await
        .unwrap();

    let _rider2 = service
        .create_rider(
            &admin_ctx,
            CreateRiderRequest {
                branch_id,
                user_id: rider2_user_id,
                vehicle_type: Some("MOTORBIKE".into()),
                cnic: "35201-7654321-2".into(),
                licence_no: "LHR-54321".into(),
            },
        )
        .await
        .unwrap();

    // Create and assign delivery to rider1
    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();
    let _ = service
        .assign_delivery(&admin_ctx, delivery.id, rider1.id)
        .await
        .unwrap();

    // Rider 2 tries to view Rider 1's delivery -> Must be Forbidden
    let rider2_ctx = create_rider_context(tenant_id, rider2_user_id);
    let result = service.get_delivery(&rider2_ctx, delivery.id).await;

    assert!(
        result.is_err(),
        "Rider token must not be able to read other riders' deliveries"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 2: rider_token_cannot_list_customers_or_other_riders
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_rider_token_cannot_list_other_riders() {
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
    let branch_id = BranchId::new();
    let rider_user_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    let rider_ctx = create_rider_context(tenant_id, rider_user_id);
    let service = FulfilmentService::new(pool);

    let result = service
        .list_riders(&rider_ctx, Some(branch_id), None, None)
        .await;
    assert!(
        result.is_err(),
        "Rider token must be forbidden from listing all riders"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 3: assignment_blocked_when_rider_over_cash_ceiling
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_assignment_blocked_when_rider_over_cash_ceiling() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order1_id = OrderId::new();
    let order2_id = OrderId::new();
    let admin_id = UserId::new();
    let rider_user_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    // Order 1: Rs 9,000 COD
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order1_id,
        Money::from_major(9000),
        true,
        false,
    )
    .await;
    // Order 2: Rs 2,000 COD (9,000 + 2,000 = 11,000 > 10,000 ceiling)
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order2_id,
        Money::from_major(2000),
        true,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool.clone());

    let rider = service
        .create_rider(
            &admin_ctx,
            CreateRiderRequest {
                branch_id,
                user_id: rider_user_id,
                vehicle_type: Some("MOTORBIKE".into()),
                cnic: "35201-1111111-1".into(),
                licence_no: "LHR-11111".into(),
            },
        )
        .await
        .unwrap();

    // Assign and deliver order 1 (accumulates Rs 9,000 undeposited cash in session)
    let del1 = service
        .create_delivery_for_order(&admin_ctx, branch_id, order1_id)
        .await
        .unwrap();
    let _ = service
        .assign_delivery(&admin_ctx, del1.id, rider.id)
        .await
        .unwrap();
    let _ = service
        .complete_delivery(
            &admin_ctx,
            del1.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/photo1.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Ali Raza".into(),
                recipient_cnic_last4: None,
                prescription_collected: None,
                cash_collected: Some(Money::from_major(9000)),
                latitude: Some(31.5204),
                longitude: Some(74.3587),
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await
        .unwrap();

    // Try assigning order 2 (Rs 2,000) -> Exceeds Rs 10,000 COD ceiling -> Must fail
    let del2 = service
        .create_delivery_for_order(&admin_ctx, branch_id, order2_id)
        .await
        .unwrap();
    let result = service.assign_delivery(&admin_ctx, del2.id, rider.id).await;

    assert!(
        result.is_err(),
        "Assignment must be blocked when undeposited cash exceeds COD ceiling"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 4: pod_photo_required_for_delivered
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_pod_photo_required_for_delivered() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
        false,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool);

    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();

    // Attempt delivery completion with empty photo
    let result = service
        .complete_delivery(
            &admin_ctx,
            delivery.id,
            DeliverRequest {
                pod_image_object_key: "   ".into(),
                pod_signature_object_key: None,
                recipient_name: "Customer Name".into(),
                recipient_cnic_last4: None,
                prescription_collected: None,
                cash_collected: None,
                latitude: Some(31.5),
                longitude: Some(74.3),
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await;

    assert!(result.is_err(), "Delivery without photo must be rejected");
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 5: gps_required_for_delivered
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_gps_required_for_delivered() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
        false,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool);

    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();

    // Attempt completion with no GPS and no gps_denied flag
    let result = service
        .complete_delivery(
            &admin_ctx,
            delivery.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/photo.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Customer Name".into(),
                recipient_cnic_last4: None,
                prescription_collected: None,
                cash_collected: None,
                latitude: None,
                longitude: None,
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await;

    assert!(
        result.is_err(),
        "Delivery without GPS and without gps_denied flag must be rejected"
    );

    // But with gps_denied flag == true, delivery succeeds gracefully
    let success = service
        .complete_delivery(
            &admin_ctx,
            delivery.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/photo.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Customer Name".into(),
                recipient_cnic_last4: None,
                prescription_collected: None,
                cash_collected: None,
                latitude: None,
                longitude: None,
                gps_denied: Some(true),
                idempotency_key: None,
            },
        )
        .await;

    assert!(
        success.is_ok(),
        "Delivery with explicit gps_denied flag must succeed"
    );
    assert!(success.unwrap().gps_denied_flag);
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 6: controlled_order_requires_prescription_collection_and_cnic
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_controlled_order_requires_prescription_collection_and_cnic() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    // Controlled order = true
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(250),
        false,
        true,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool);

    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();

    // 1. Missing physical prescription collection -> Must fail
    let err1 = service
        .complete_delivery(
            &admin_ctx,
            delivery.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/rx_photo.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Tariq Mahmood".into(),
                recipient_cnic_last4: Some("4321".into()),
                prescription_collected: Some(false),
                cash_collected: None,
                latitude: Some(31.5),
                longitude: Some(74.3),
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await;
    assert!(
        err1.is_err(),
        "Controlled order missing prescription collection must be rejected"
    );

    // 2. Missing CNIC last 4 -> Must fail
    let err2 = service
        .complete_delivery(
            &admin_ctx,
            delivery.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/rx_photo.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Tariq Mahmood".into(),
                recipient_cnic_last4: None,
                prescription_collected: Some(true),
                cash_collected: None,
                latitude: Some(31.5),
                longitude: Some(74.3),
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await;
    assert!(
        err2.is_err(),
        "Controlled order missing CNIC last 4 must be rejected"
    );

    // 3. Valid with both prescription collected and CNIC last 4 digits -> Succeeds
    let ok = service
        .complete_delivery(
            &admin_ctx,
            delivery.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/rx_photo.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Tariq Mahmood".into(),
                recipient_cnic_last4: Some("4321".into()),
                prescription_collected: Some(true),
                cash_collected: None,
                latitude: Some(31.5),
                longitude: Some(74.3),
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await;
    assert!(
        ok.is_ok(),
        "Controlled order with prescription and CNIC last 4 must succeed"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 7: cod_delivery_accumulates_expected_amount
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_cod_delivery_accumulates_expected_amount() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order1_id = OrderId::new();
    let order2_id = OrderId::new();
    let admin_id = UserId::new();
    let rider_user_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order1_id,
        Money::from_major(1500),
        true,
        false,
    )
    .await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order2_id,
        Money::from_major(2500),
        true,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool.clone());

    let rider = service
        .create_rider(
            &admin_ctx,
            CreateRiderRequest {
                branch_id,
                user_id: rider_user_id,
                vehicle_type: Some("MOTORBIKE".into()),
                cnic: "35201-9999999-1".into(),
                licence_no: "LHR-99999".into(),
            },
        )
        .await
        .unwrap();

    // Complete delivery 1 (Rs 1500)
    let del1 = service
        .create_delivery_for_order(&admin_ctx, branch_id, order1_id)
        .await
        .unwrap();
    let _ = service
        .assign_delivery(&admin_ctx, del1.id, rider.id)
        .await
        .unwrap();
    let _ = service
        .complete_delivery(
            &admin_ctx,
            del1.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/photo1.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Ali".into(),
                recipient_cnic_last4: None,
                prescription_collected: None,
                cash_collected: Some(Money::from_major(1500)),
                latitude: Some(31.5),
                longitude: Some(74.3),
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await
        .unwrap();

    // Complete delivery 2 (Rs 2500)
    let del2 = service
        .create_delivery_for_order(&admin_ctx, branch_id, order2_id)
        .await
        .unwrap();
    let _ = service
        .assign_delivery(&admin_ctx, del2.id, rider.id)
        .await
        .unwrap();
    let _ = service
        .complete_delivery(
            &admin_ctx,
            del2.id,
            DeliverRequest {
                pod_image_object_key: "tenant/pod/photo2.jpg".into(),
                pod_signature_object_key: None,
                recipient_name: "Ali".into(),
                recipient_cnic_last4: None,
                prescription_collected: None,
                cash_collected: Some(Money::from_major(2500)),
                latitude: Some(31.5),
                longitude: Some(74.3),
                gps_denied: None,
                idempotency_key: None,
            },
        )
        .await
        .unwrap();

    // Check open cash session total
    let sessions = service
        .list_cash_sessions(
            &admin_ctx,
            Some(rider.id),
            None,
            Some(CashSessionStatus::Open),
        )
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].expected_amount, Money::from_major(4000));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 8: variance_blocks_session_close_without_reason
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_variance_blocks_session_close_without_reason() {
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
    let branch_id = BranchId::new();
    let session_id = RiderCashSessionId::new();
    let rider_id = RiderId::new();
    let admin_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;

    // Seed session with expected Rs 4,000
    sqlx::query(
        "INSERT INTO rider_cash_sessions (id, tenant_id, rider_id, branch_id, status, opened_at, expected_amount)
         VALUES ($1, $2, $3, $4, 'DECLARED', now(), 4000.0000)"
    )
    .bind(session_id.0)
    .bind(tenant_id.0)
    .bind(rider_id.0)
    .bind(branch_id.0)
    .execute(&pool)
    .await
    .unwrap();

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool);

    // 1. Attempt reconciliation with variance (deposited 3,800 vs expected 4,000) with NO reason -> Fails
    let err = service
        .reconcile_cash_session(
            &admin_ctx,
            session_id,
            ReconcileCashSessionRequest {
                deposited_amount: Money::from_major(3800),
                note: None,
            },
        )
        .await;
    assert!(
        err.is_err(),
        "Session closure with non-zero variance and no note must be blocked"
    );

    // 2. Reconciliation with variance AND documented reason -> Succeeds
    let ok = service
        .reconcile_cash_session(
            &admin_ctx,
            session_id,
            ReconcileCashSessionRequest {
                deposited_amount: Money::from_major(3800),
                note: Some("Customer short Rs 200 on change, manager approved waiver".into()),
            },
        )
        .await;
    assert!(
        ok.is_ok(),
        "Session closure with non-zero variance and valid note must succeed"
    );
    assert_eq!(ok.unwrap().status, CashSessionStatus::Reconciled);
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 9: stale_session_blocks_new_cod_assignment
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_stale_session_blocks_new_cod_assignment() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();
    let rider_user_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
        true,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool.clone());

    let rider = service
        .create_rider(
            &admin_ctx,
            CreateRiderRequest {
                branch_id,
                user_id: rider_user_id,
                vehicle_type: Some("MOTORBIKE".into()),
                cnic: "35201-8888888-1".into(),
                licence_no: "LHR-88888".into(),
            },
        )
        .await
        .unwrap();

    // Insert a stale un-reconciled session (> 24h old)
    let session_id = RiderCashSessionId::new();
    let stale_opened_at = Utc::now() - Duration::hours(30);
    sqlx::query(
        "INSERT INTO rider_cash_sessions (id, tenant_id, rider_id, branch_id, status, opened_at, expected_amount)
         VALUES ($1, $2, $3, $4, 'OPEN', $5, 500.0000)"
    )
    .bind(session_id.0)
    .bind(tenant_id.0)
    .bind(rider.id.0)
    .bind(branch_id.0)
    .bind(stale_opened_at)
    .execute(&pool)
    .await
    .unwrap();

    // Attempt assigning a new COD delivery -> Must fail due to stale open session
    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();
    let result = service
        .assign_delivery(&admin_ctx, delivery.id, rider.id)
        .await;

    assert!(
        result.is_err(),
        "Rider with stale unclosed cash session must be blocked from COD assignment"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 10: failed_delivery_max_two_reattempts_then_returned
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_failed_delivery_max_two_reattempts_then_returned() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
        false,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool);

    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();

    // 1st failure (reattempt_count = 1) -> status: FAILED
    let f1 = service
        .fail_delivery(
            &admin_ctx,
            delivery.id,
            FailDeliveryRequest {
                reason: "Customer phone switched off".into(),
                photo_object_key: None,
                idempotency_key: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(f1.status, DeliveryStatus::Failed);
    assert_eq!(f1.reattempt_count, 1);

    // 2nd failure (reattempt_count = 2) -> status: FAILED
    let f2 = service
        .fail_delivery(
            &admin_ctx,
            delivery.id,
            FailDeliveryRequest {
                reason: "Door locked, neighbor said out of city".into(),
                photo_object_key: None,
                idempotency_key: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(f2.status, DeliveryStatus::Failed);
    assert_eq!(f2.reattempt_count, 2);

    // 3rd failure (reattempt_count = 3 > 2) -> status: RETURNED
    let f3 = service
        .fail_delivery(
            &admin_ctx,
            delivery.id,
            FailDeliveryRequest {
                reason: "Customer refused package".into(),
                photo_object_key: None,
                idempotency_key: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(f3.status, DeliveryStatus::Returned);
    assert_eq!(f3.reattempt_count, 3);
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 11: duplicate_delivery_submission_idempotent
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_duplicate_delivery_submission_idempotent() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
        false,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool);

    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();

    let req = DeliverRequest {
        pod_image_object_key: "tenant/pod/photo.jpg".into(),
        pod_signature_object_key: None,
        recipient_name: "Customer".into(),
        recipient_cnic_last4: None,
        prescription_collected: None,
        cash_collected: None,
        latitude: Some(31.5),
        longitude: Some(74.3),
        gps_denied: None,
        idempotency_key: Some("idempotent_pod_key_12345".into()),
    };

    // First submission
    let d1 = service
        .complete_delivery(&admin_ctx, delivery.id, req.clone())
        .await
        .unwrap();
    assert_eq!(d1.status, DeliveryStatus::Delivered);

    // Duplicate offline retry submission -> Must return successfully without duplicating or erroring
    let d2 = service
        .complete_delivery(&admin_ctx, delivery.id, req)
        .await
        .unwrap();
    assert_eq!(d2.status, DeliveryStatus::Delivered);
    assert_eq!(d1.id, d2.id);
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 12: public_tracking_link_leaks_no_pii
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_public_tracking_link_leaks_no_pii() {
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
    let branch_id = BranchId::new();
    let customer_id = CustomerId::new();
    let order_id = OrderId::new();
    let admin_id = UserId::new();

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id).await;
    seed_test_order(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
        false,
        false,
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = FulfilmentService::new(pool);

    let delivery = service
        .create_delivery_for_order(&admin_ctx, branch_id, order_id)
        .await
        .unwrap();

    // Public tracking query with no credentials
    let tracking = service
        .get_public_tracking(&delivery.tracking_token)
        .await
        .unwrap();

    // Assert zero PII
    let json_val = serde_json::to_value(&tracking).unwrap();
    assert!(
        json_val.get("phone").is_none(),
        "Customer phone must NOT be in public tracking"
    );
    assert!(
        json_val.get("customer_name").is_none(),
        "Customer name must NOT be in public tracking"
    );
    assert!(
        json_val.get("delivery_address").is_none(),
        "Delivery address must NOT be in public tracking"
    );
    assert!(
        json_val.get("items").is_none(),
        "Order items must NOT be in public tracking"
    );
    assert!(tracking.order_ref.starts_with("ORD-"));
    assert_eq!(tracking.status, DeliveryStatus::Unassigned);
}
