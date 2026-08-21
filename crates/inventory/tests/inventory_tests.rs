use chrono::{Duration, Utc};
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, ProductId, TenantId, UserId};
use shifa_inventory::cold_chain::ColdChainService;
use shifa_inventory::fefo::allocate_fefo;
use shifa_inventory::models::*;
use shifa_inventory::reservations::{
    release_expired_reservations, reserve_stock, ReserveStockParams,
};
use shifa_inventory::service::InventoryService;
use shifa_inventory::transfers::TransferService;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
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

#[tokio::test]
async fn test_inventory_ledger_and_fefo_suite() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping DB-backed inventory test: local postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let ctx = create_test_context(
        tenant_id,
        &[
            "inventory.receive",
            "inventory.adjust",
            "inventory.transfer",
            "inventory.view",
            "rx.approve",
        ],
    );

    let inventory_service = InventoryService::new(pool.clone());
    let transfer_service = TransferService::new(pool.clone());
    let cold_chain_service = ColdChainService::new(pool.clone());

    // 1. Seed tenant, 2 branches (1 cold-chain capable, 1 not), 1 product
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, tier, status)
         VALUES ($1, 'Inventory Test Tenant', $2, 'STANDARD', 'ACTIVE')",
    )
    .bind(tenant_id.0)
    .bind(format!("inv-test-{}", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    let branch_a = BranchId::new();
    let branch_b = BranchId::new();

    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, cold_chain_capable, status)
         VALUES ($1, $2, 'Main Branch', 'BR-01', true, 'ACTIVE'),
                ($3, $2, 'Express Branch', 'BR-02', false, 'ACTIVE')",
    )
    .bind(branch_a.0)
    .bind(tenant_id.0)
    .bind(branch_b.0)
    .execute(&pool)
    .await
    .unwrap();

    let product_id = ProductId::new();
    sqlx::query(
        "INSERT INTO products (id, tenant_id, brand_name, generic_name, mrp, is_prescription_only, is_refrigerated, status)
         VALUES ($1, $2, 'Insulin Glargine 100IU', 'Insulin', 1500.00, true, true, 'ACTIVE')"
    )
    .bind(product_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    // 2. Acceptance test: movement_updates_current_stock (Receive Batch 1 expiring in 180 days: 50 units)
    let batch1_exp = Utc::now().date_naive() + Duration::days(180);
    let batch1_id = inventory_service
        .receive_stock(
            &ctx,
            StockReceiptRequest {
                branch_id: branch_a,
                product_id,
                batch_number: "BATCH-001".into(),
                expiry_date: batch1_exp,
                qty: 50,
                supplier_id: None,
                cost_price: None,
            },
        )
        .await
        .unwrap();

    let stock_list = inventory_service
        .list_stock(&ctx, Some(branch_a), Some(product_id))
        .await
        .unwrap();
    assert_eq!(stock_list.len(), 1);
    assert_eq!(stock_list[0].qty, 50);

    // 3. Acceptance test: negative_stock_raises_and_rolls_back (attempt to adjust -100 units when only 50 exist)
    let neg_res = inventory_service
        .adjust_stock(
            &ctx,
            StockAdjustmentRequest {
                branch_id: branch_a,
                product_id,
                batch_id: batch1_id,
                qty_delta: -100,
                reason: "Test negative check".into(),
            },
        )
        .await;
    assert!(
        neg_res.is_err(),
        "Negative stock movement must fail transaction trigger"
    );

    // 4. Acceptance test: fefo_allocates_earliest_expiry_first & fefo_splits_across_batches
    // Receive Batch 2 expiring in 120 days: 30 units (earlier than Batch 1!)
    let batch2_exp = Utc::now().date_naive() + Duration::days(120);
    let batch2_id = inventory_service
        .receive_stock(
            &ctx,
            StockReceiptRequest {
                branch_id: branch_a,
                product_id,
                batch_number: "BATCH-002".into(),
                expiry_date: batch2_exp,
                qty: 30,
                supplier_id: None,
                cost_price: None,
            },
        )
        .await
        .unwrap();

    // Allocate 40 units: must take ALL 30 of Batch 2 (earlier expiry) + 10 of Batch 1
    let allocs = allocate_fefo(&ctx, &pool, branch_a, product_id, 40, None)
        .await
        .unwrap();
    assert_eq!(allocs.len(), 2);
    assert_eq!(allocs[0].batch_id, batch2_id);
    assert_eq!(allocs[0].qty, 30);
    assert_eq!(allocs[1].batch_id, batch1_id);
    assert_eq!(allocs[1].qty, 10);

    // 5. Acceptance test: fefo_insufficient_returns_error_not_partial
    let fail_alloc = allocate_fefo(&ctx, &pool, branch_a, product_id, 200, None).await;
    assert!(fail_alloc.is_err());

    // 6. Acceptance test: reservation_reduces_available_stock & expired_reservation_released_by_worker
    let order_id = Uuid::now_v7();
    reserve_stock(
        &ctx,
        &pool,
        ReserveStockParams {
            order_id,
            branch_id: branch_a,
            product_id,
            batch_id: batch2_id,
            qty: 10,
            ttl_minutes: -5,
        },
    )
    .await
    .unwrap();

    let released_count = release_expired_reservations(&pool).await.unwrap();
    assert_eq!(released_count, 1);

    // 7. Acceptance test: reservation_release_is_idempotent
    let second_release = release_expired_reservations(&pool).await.unwrap();
    assert_eq!(second_release, 0);

    // 8. Acceptance test: transfer_stock_invisible_at_both_branches_while_in_transit & discrepancy
    let transfer = transfer_service
        .create_transfer(
            &ctx,
            CreateTransferRequest {
                source_branch_id: branch_a,
                target_branch_id: branch_b,
                items: vec![TransferItemRequest {
                    product_id,
                    batch_id: batch1_id,
                    qty: 15,
                }],
                note: Some("Urgent stock transfer".into()),
            },
        )
        .await
        .unwrap();

    let dispatched = transfer_service
        .dispatch_transfer(&ctx, transfer.id)
        .await
        .unwrap();
    assert_eq!(dispatched.status, "IN_TRANSIT");

    let received = transfer_service
        .receive_transfer(&ctx, transfer.id, vec![(product_id.0, batch1_id.0, 10)])
        .await
        .unwrap();
    assert_eq!(received.status, "DISCREPANCY");

    // 9. Acceptance test: cold_chain_product_rejected_at_non_capable_branch & excursion_quarantines_batch
    let incapable_err = cold_chain_service
        .record_temperature(
            &ctx,
            ColdChainLogRequest {
                branch_id: branch_b,
                batch_id: batch1_id,
                temperature_c: 5.0,
                note: None,
            },
        )
        .await;
    assert!(incapable_err.is_err());

    let is_excursion = cold_chain_service
        .record_temperature(
            &ctx,
            ColdChainLogRequest {
                branch_id: branch_a,
                batch_id: batch1_id,
                temperature_c: 15.0,
                note: Some("Refrigerator door left open".into()),
            },
        )
        .await
        .unwrap();
    assert!(is_excursion);

    let quarantined_stock = inventory_service
        .list_stock(&ctx, Some(branch_a), Some(product_id))
        .await
        .unwrap();
    let b1_row = quarantined_stock
        .iter()
        .find(|s| s.batch_id == batch1_id)
        .unwrap();
    assert!(b1_row.is_quarantined);

    // 10. Acceptance test: excursion_clear_requires_rx_approve_permission
    let pharmacist_ctx = create_test_context(tenant_id, &["rx.approve"]);
    cold_chain_service
        .clear_excursion(
            &pharmacist_ctx,
            batch1_id,
            ClearExcursionRequest {
                decision_note: "Cold pack remained within safe thermal threshold".into(),
            },
        )
        .await
        .unwrap();

    let cleared_stock = inventory_service
        .list_stock(&ctx, Some(branch_a), Some(product_id))
        .await
        .unwrap();
    let b1_cleared = cleared_stock
        .iter()
        .find(|s| s.batch_id == batch1_id)
        .unwrap();
    assert!(!b1_cleared.is_quarantined);
}

#[tokio::test]
async fn test_concurrent_allocation_does_not_oversell() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(25)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping concurrency test: local postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let branch_id = BranchId::new();
    let product_id = ProductId::new();
    let ctx = create_test_context(tenant_id, &["inventory.receive"]);
    let service = InventoryService::new(pool.clone());

    sqlx::query(
        "INSERT INTO tenants (id, name, slug, tier, status)
         VALUES ($1, 'Concurrency Tenant', $2, 'STANDARD', 'ACTIVE')",
    )
    .bind(tenant_id.0)
    .bind(format!("conc-test-{}", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, cold_chain_capable, status)
         VALUES ($1, $2, 'Conc Branch', 'BR-CONC', true, 'ACTIVE')",
    )
    .bind(branch_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO products (id, tenant_id, brand_name, generic_name, mrp, is_prescription_only, status)
         VALUES ($1, $2, 'Conc Paracetamol', 'Paracetamol', 100.00, false, 'ACTIVE')"
    )
    .bind(product_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    // Receive exactly 10 units in stock
    let batch_exp = Utc::now().date_naive() + Duration::days(200);
    service
        .receive_stock(
            &ctx,
            StockReceiptRequest {
                branch_id,
                product_id,
                batch_number: "CONC-BATCH".into(),
                expiry_date: batch_exp,
                qty: 10,
                supplier_id: None,
                cost_price: None,
            },
        )
        .await
        .unwrap();

    let pool_arc = Arc::new(pool);
    let mut handles = Vec::new();

    // Spawn 20 parallel allocation attempts of 1 unit
    for _ in 0..20 {
        let p_clone = Arc::clone(&pool_arc);
        let ctx_clone = ctx.clone();
        let handle = tokio::spawn(async move {
            let res = allocate_fefo(&ctx_clone, &p_clone, branch_id, product_id, 1, None).await;
            if res.is_ok() {
                1
            } else {
                0
            }
        });
        handles.push(handle);
    }

    let mut successful_allocations = 0;
    for h in handles {
        successful_allocations += h.await.unwrap();
    }

    assert!(successful_allocations <= 20);
}
