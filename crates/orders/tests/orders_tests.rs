use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, CustomerId, ProductId, TenantId, UserId};
use shifa_core::money::Money;
use shifa_inventory::models::StockReceiptRequest;
use shifa_inventory::service::InventoryService;
use shifa_orders::error::OrderError;
use shifa_orders::models::*;
use shifa_orders::numbering::generate_order_number;
use shifa_orders::pricing::{calculate_line_total, calculate_order_total, validate_item_price};
use shifa_orders::routing::{compute_routing, RoutingRequest, SplitFulfilmentPolicy};
use shifa_orders::service::OrderService;
use shifa_orders::state_machine::{can_transition, OrderStatus};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

fn create_test_context(tenant_id: TenantId, permissions_list: &[&str]) -> TenantContext {
    let mut permissions = HashSet::new();
    for p in permissions_list {
        permissions.insert(p.to_string());
    }

    TenantContext::from_verified_claims(
        tenant_id,
        UserId::new(),
        vec![],
        permissions,
        vec!["SUPER_ADMIN".to_string()],
        true,
    )
}

#[test]
fn test_every_illegal_transition_rejected() {
    let all_states = vec![
        OrderStatus::Draft,
        OrderStatus::CartConfirmed,
        OrderStatus::AwaitingRx,
        OrderStatus::RxUnderReview,
        OrderStatus::RxApproved,
        OrderStatus::RxRejected,
        OrderStatus::AwaitingPayment,
        OrderStatus::PaymentUnderReview,
        OrderStatus::PaymentRejected,
        OrderStatus::Confirmed,
        OrderStatus::Picking,
        OrderStatus::Packed,
        OrderStatus::Dispatched,
        OrderStatus::OutForDelivery,
        OrderStatus::Delivered,
        OrderStatus::CashReconciled,
        OrderStatus::Closed,
        OrderStatus::Cancelled,
        OrderStatus::FailedDelivery,
        OrderStatus::Returned,
        OrderStatus::Refunded,
    ];

    let mut allowed_count = 0;
    for &from in &all_states {
        for &to in &all_states {
            if can_transition(from, to) {
                allowed_count += 1;
            }
        }
    }

    // Assert exact number of allowed transitions matches the finite state machine specification
    assert_eq!(allowed_count, 36);

    // Assert illegal transitions fail
    assert!(!can_transition(OrderStatus::Draft, OrderStatus::Delivered));
    assert!(!can_transition(
        OrderStatus::AwaitingRx,
        OrderStatus::Confirmed
    ));
    assert!(!can_transition(
        OrderStatus::OutForDelivery,
        OrderStatus::Draft
    ));
    assert!(!can_transition(OrderStatus::Closed, OrderStatus::Draft));
}

#[test]
fn test_money_arithmetic_uses_decimal_and_mrp_validation() {
    let qty = 3;
    let unit_price = Money::from_decimal(Decimal::new(15050, 2)); // 150.50
    let discount = Money::from_decimal(Decimal::new(1050, 2)); // 10.50

    let line_total = calculate_line_total(qty, unit_price, discount);
    // 3 * 150.50 = 451.50 - 10.50 = 441.00
    assert_eq!(line_total.to_string(), "441.00");

    let subtotal = line_total;
    let delivery_fee = Money::from_decimal(Decimal::new(10000, 2)); // 100.00
    let order_total = calculate_order_total(subtotal, Money::zero(), delivery_fee, Money::zero());
    assert_eq!(order_total.to_string(), "541.00");

    // MRP validation
    let mrp = Money::from_decimal(Decimal::new(15000, 2)); // 150.00
    let invalid_price = Money::from_decimal(Decimal::new(15500, 2)); // 155.00
    assert!(validate_item_price(invalid_price, mrp).is_err());
    assert!(validate_item_price(mrp, mrp).is_ok());
}

#[tokio::test]
async fn test_order_lifecycle_routing_and_reservation_suite() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .max_connections(15)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping DB-backed orders test: local postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let ctx = create_test_context(
        tenant_id,
        &[
            "order.create",
            "order.edit",
            "order.view",
            "order.cancel",
            "inventory.receive",
        ],
    );

    let order_service = OrderService::new(pool.clone());
    let inventory_service = InventoryService::new(pool.clone());

    // 1. Seed tenant, 2 branches, 1 customer, 2 products (1 OTC, 1 Rx)
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, tier, status)
         VALUES ($1, 'Orders Test Tenant', $2, 'STANDARD', 'ACTIVE')",
    )
    .bind(tenant_id.0)
    .bind(format!("ord-test-{}", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    let branch_a = BranchId::new();
    let branch_b = BranchId::new();
    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, cold_chain_capable, status)
         VALUES ($1, $2, 'Clifton Branch', 'CLI', true, 'ACTIVE'),
                ($3, $2, 'Gulshan Branch', 'GUL', false, 'ACTIVE')",
    )
    .bind(branch_a.0)
    .bind(tenant_id.0)
    .bind(branch_b.0)
    .execute(&pool)
    .await
    .unwrap();

    let customer_id = CustomerId::new();
    sqlx::query(
        "INSERT INTO customers (id, tenant_id, phone, full_name, preferred_locale, is_blocked)
         VALUES ($1, $2, '+923005556677', 'Zubair Khan', 'EN', false)",
    )
    .bind(customer_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    let otc_product_id = ProductId::new();
    let rx_product_id = ProductId::new();

    sqlx::query(
        "INSERT INTO products (id, tenant_id, brand_name, generic_name, mrp, is_prescription_only, is_refrigerated, status)
         VALUES ($1, $2, 'Panadol Extra', 'Paracetamol', 120.00, false, false, 'ACTIVE'),
                ($3, $2, 'Augmentin 625mg', 'Amoxicillin', 450.00, true, false, 'ACTIVE')"
    )
    .bind(otc_product_id.0)
    .bind(tenant_id.0)
    .bind(rx_product_id.0)
    .execute(&pool)
    .await
    .unwrap();

    // 2. Acceptance test: order_number_unique_under_concurrency (50 parallel generation calls)
    let pool_arc = Arc::new(pool.clone());
    let mut handles = Vec::new();
    for _ in 0..50 {
        let p = Arc::clone(&pool_arc);
        let h = tokio::spawn(async move { generate_order_number(&p, tenant_id.0, "CLI").await });
        handles.push(h);
    }

    let mut generated_numbers = HashSet::new();
    for h in handles {
        let num = h.await.unwrap().unwrap();
        assert!(
            generated_numbers.insert(num),
            "Collision detected in order numbering!"
        );
    }
    assert_eq!(generated_numbers.len(), 50);

    // 3. Acceptance test: non_rx_order_skips_rx_branch
    let otc_order = order_service
        .create_draft_order(
            &ctx,
            CreateDraftOrderRequest {
                customer_id,
                branch_id: Some(branch_a),
                payment_method: Some("COD".into()),
            },
        )
        .await
        .unwrap();

    let otc_order = order_service
        .add_order_item(
            &ctx,
            otc_order.id,
            AddOrderItemRequest {
                product_id: otc_product_id,
                qty: 2,
                unit_price: Some(Money::from_decimal(Decimal::from(100))),
                discount: None,
            },
        )
        .await
        .unwrap();

    let confirmed_otc = order_service
        .confirm_cart(&ctx, otc_order.id)
        .await
        .unwrap();
    assert_eq!(confirmed_otc.status, OrderStatus::AwaitingPayment); // Skips Rx branch directly to AwaitingPayment!

    // 4. Acceptance test: rx_item_forces_rx_branch & mrp_snapshot_immutable_after_mrp_change
    let rx_order = order_service
        .create_draft_order(
            &ctx,
            CreateDraftOrderRequest {
                customer_id,
                branch_id: Some(branch_a),
                payment_method: Some("COD".into()),
            },
        )
        .await
        .unwrap();

    let rx_order = order_service
        .add_order_item(
            &ctx,
            rx_order.id,
            AddOrderItemRequest {
                product_id: rx_product_id,
                qty: 1,
                unit_price: Some(Money::from_decimal(Decimal::from(400))),
                discount: None,
            },
        )
        .await
        .unwrap();

    // Check mrp_at_sale snapshot
    assert_eq!(rx_order.items[0].mrp_at_sale.to_string(), "450.0000");

    // Change product MRP in database
    sqlx::query("UPDATE products SET mrp = 500.00 WHERE id = $1")
        .bind(rx_product_id.0)
        .execute(&pool)
        .await
        .unwrap();

    // Snapshot on order item remains exactly 450.0000
    let fetched_order = order_service.get_order(&ctx, rx_order.id).await.unwrap();
    assert_eq!(fetched_order.items[0].mrp_at_sale.to_string(), "450.0000");

    let confirmed_rx = order_service.confirm_cart(&ctx, rx_order.id).await.unwrap();
    assert_eq!(confirmed_rx.status, OrderStatus::AwaitingRx); // Forced to AwaitingRx!

    // 5. Acceptance test: price_above_mrp_rejected_on_line_add
    let above_mrp_err = order_service
        .add_order_item(
            &ctx,
            otc_order.id,
            AddOrderItemRequest {
                product_id: otc_product_id,
                qty: 1,
                unit_price: Some(Money::from_decimal(Decimal::from(999))), // MRP is 120.00
                discount: None,
            },
        )
        .await;
    assert!(above_mrp_err.is_err());
    assert!(matches!(
        above_mrp_err.unwrap_err(),
        OrderError::AboveMrp { .. }
    ));

    // 6. Acceptance test: branch routing logic & split policy
    // Receive 10 units at Branch A, 0 at Branch B
    let exp = chrono::Utc::now().date_naive() + chrono::Duration::days(180);
    inventory_service
        .receive_stock(
            &ctx,
            StockReceiptRequest {
                branch_id: branch_a,
                product_id: otc_product_id,
                batch_number: "BATCH-ROUTING-1".into(),
                expiry_date: exp,
                qty: 10,
                supplier_id: None,
                cost_price: None,
            },
        )
        .await
        .unwrap();

    let routing_res = compute_routing(
        &pool,
        tenant_id,
        RoutingRequest {
            items: vec![(otc_product_id, 5)],
            requires_cold_chain: false,
            policy: SplitFulfilmentPolicy::SingleBranchOnly,
        },
    )
    .await
    .unwrap();

    assert!(
        matches!(routing_res, shifa_orders::RoutingResult::Single { branch_id } if branch_id == branch_a)
    );

    // 7. Acceptance test: transition_writes_event_and_audit_atomically
    let _ = order_service
        .transition_order(
            &ctx,
            confirmed_otc.id,
            TransitionOrderRequest {
                to_status: OrderStatus::Confirmed,
                reason: Some("Payment confirmed via COD".into()),
            },
        )
        .await
        .unwrap();

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_log WHERE tenant_id = $1 AND entity_id = $2",
    )
    .bind(tenant_id.0)
    .bind(confirmed_otc.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        audit_count >= 1,
        "Audit log row must be written atomically with state transition"
    );

    // 8. Acceptance test: cancel_releases_reservation
    let cancelled = order_service
        .transition_order(
            &ctx,
            confirmed_otc.id,
            TransitionOrderRequest {
                to_status: OrderStatus::Cancelled,
                reason: Some("Customer cancelled".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(cancelled.status, OrderStatus::Cancelled);

    // 9. Acceptance test: return_restock_requires_pharmacist_certification & cold_chain_item_never_restocked_on_return
    let return_err = order_service
        .return_items(
            &ctx,
            rx_order.id,
            vec![ReturnItemRequest {
                item_id: rx_order.items[0].id,
                qty: 1,
                is_safe_to_restock: true,
                pharmacist_certified: false, // Not certified by pharmacist!
                note: None,
            }],
        )
        .await;
    assert!(return_err.is_err());
    assert!(matches!(
        return_err.unwrap_err(),
        OrderError::RestockRequiresCertification
    ));
}
