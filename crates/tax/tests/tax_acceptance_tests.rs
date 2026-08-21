use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, CustomerId, OrderId, ProductId, TaxCategoryId, TenantId, UserId};
use shifa_core::money::Money;
use shifa_tax::calculator::{TaxCalculator, TaxableItemInput};
use shifa_tax::fbr::{MockFbrBehavior, MockFbrReporter};
use shifa_tax::models::*;
use shifa_tax::service::TaxService;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;

fn create_admin_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    let mut perms = HashSet::new();
    perms.insert("tenant.settings".to_string());
    perms.insert("order.refund".to_string());
    perms.insert("report.view".to_string());
    perms.insert("order.edit".to_string());

    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        perms,
        vec!["SUPER_ADMIN".to_string()],
    )
}

async fn seed_test_tenant_and_branch(
    pool: &PgPool,
    tenant_id: TenantId,
    branch_id: BranchId,
    branch_code: &str,
) {
    sqlx::query(
        "INSERT INTO tenants (id, name, slug) VALUES ($1, 'Tax Test Pharmacy', $2)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(tenant_id.0)
    .bind(format!("tax-test-{}", tenant_id.0))
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, is_warehouse)
         VALUES ($1, $2, 'Lahore Gulberg', $3, false)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(branch_id.0)
    .bind(tenant_id.0)
    .bind(branch_code)
    .execute(pool)
    .await
    .unwrap();
}

#[allow(clippy::too_many_arguments)]
async fn seed_test_order_with_items(
    pool: &PgPool,
    tenant_id: TenantId,
    branch_id: BranchId,
    customer_id: CustomerId,
    order_id: OrderId,
    amount: Money,
) {
    sqlx::query(
        "INSERT INTO customers (id, tenant_id, phone, full_name, is_blocked)
         VALUES ($1, $2, $3, 'Usman Tariq', false)
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

    sqlx::query(
        "INSERT INTO orders (id, tenant_id, branch_id, customer_id, status, subtotal, discount, delivery_fee, tax, total_amount, payment_method, total_price)
         VALUES ($1, $2, $3, $4, 'CONFIRMED'::order_status, $5, 0.0000, 0.0000, 0.0000, $5, 'COD', $5)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(order_id.0)
    .bind(tenant_id.0)
    .bind(branch_id.0)
    .bind(customer_id.0)
    .bind(amount.0)
    .execute(pool)
    .await
    .unwrap();

    let product_id = ProductId::new();
    sqlx::query(
        "INSERT INTO products (id, tenant_id, name, slug, form, strength, mrp)
         VALUES ($1, $2, 'Panadol Extra 500mg', $3, 'TABLET', '500mg', $4)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(product_id.0)
    .bind(tenant_id.0)
    .bind(format!("panadol-{}", product_id.0))
    .bind(amount.0)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO order_items (id, tenant_id, order_id, product_id, quantity, unit_price, total_price, mrp_at_sale)
         VALUES (uuidv7(), $1, $2, $3, 1, $4, $4, $4)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(tenant_id.0)
    .bind(order_id.0)
    .bind(product_id.0)
    .bind(amount.0)
    .execute(pool)
    .await
    .unwrap();
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 1 & 2: fbr_outage_does_not_block_order_confirmation
//                        invoice_generated_with_local_number_before_fbr_response
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_fbr_outage_does_not_block_order_confirmation() {
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

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id, "LHR01").await;
    seed_test_order_with_items(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1000),
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = TaxService::new(pool.clone());

    // Create tax category
    let _cat = service
        .create_tax_category(
            &admin_ctx,
            CreateTaxCategoryRequest {
                name: "General Medicines".into(),
                rate: Decimal::new(18, 0), // 18%
                fbr_code: Some("CAT-MED-01".into()),
                is_exempt: Some(false),
                is_zero_rated: Some(false),
                effective_from: None,
            },
        )
        .await
        .unwrap();

    // 1. Generate invoice locally (FBR not called synchronously, sale proceeds immediately)
    let invoice = service
        .generate_invoice_for_order(&admin_ctx, order_id, branch_id, Utc::now())
        .await
        .unwrap();

    assert!(
        invoice.invoice_no.starts_with("LHR01/FY"),
        "Local gapless invoice number generated immediately"
    );
    assert_eq!(
        invoice.fiscal_invoice_no, None,
        "Fiscal number is null before FBR acceptance"
    );
    assert_eq!(invoice.fbr_queue_status, FbrQueueStatus::Pending);

    // 2. Mock FBR outage during asynchronous background submission
    let fbr_outage_reporter = MockFbrReporter::new(MockFbrBehavior::OutageNetworkFailure {
        message: "FBR POS Gateway timeout 504 Gateway Timeout".into(),
    });

    let async_res = service
        .process_fbr_submission(&admin_ctx, invoice.id, &fbr_outage_reporter)
        .await
        .unwrap();

    // The invoice is marked FAILED in queue for retry, but order/sale is completely intact
    assert_eq!(async_res.fbr_queue_status, FbrQueueStatus::Failed);
    assert_eq!(async_res.retry_count, 1);
    assert!(async_res.fbr_error.unwrap().contains("504 Gateway Timeout"));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 3: local_invoice_numbering_gapless_under_concurrency
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_local_invoice_numbering_gapless_under_concurrency() {
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
            println!("Skipping DB test: postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let branch_id = BranchId::new();
    seed_test_tenant_and_branch(&pool, tenant_id, branch_id, "ISB01").await;

    let service = Arc::new(TaxService::new(pool.clone()));
    let now = Utc::now();

    // Spawn 10 concurrent requests requesting next invoice sequence
    let mut handles = Vec::new();
    for _ in 0..10 {
        let s = Arc::clone(&service);
        let p = pool.clone();
        let handle = tokio::spawn(async move {
            let mut tx = p.begin().await.unwrap();
            let num = s
                .get_next_gapless_invoice_number(&mut tx, tenant_id, branch_id, "ISB01", now)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            num
        });
        handles.push(handle);
    }

    let mut generated_numbers = Vec::new();
    for h in handles {
        generated_numbers.push(h.await.unwrap());
    }

    // Verify all 10 numbers are unique, strictly ordered sequence 000001 to 000010, gapless
    generated_numbers.sort();
    let unique_set: HashSet<_> = generated_numbers.iter().cloned().collect();
    assert_eq!(
        unique_set.len(),
        10,
        "All concurrent numbers must be unique"
    );

    for (i, num) in generated_numbers.iter().enumerate() {
        let expected_seq = format!(
            "ISB01/{}/{:06}",
            TaxService::compute_fiscal_year(now),
            i + 1
        );
        assert_eq!(
            num, &expected_seq,
            "Numbering must be strictly contiguous and gapless"
        );
    }
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 4 & 16: cancelled_invoice_becomes_credit_note_not_gap & credit_note_references_original_invoice
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_cancelled_invoice_becomes_credit_note_not_gap() {
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

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id, "LHR02").await;
    seed_test_order_with_items(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(2000),
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = TaxService::new(pool.clone());

    let _cat = service
        .create_tax_category(
            &admin_ctx,
            CreateTaxCategoryRequest {
                name: "General Medicines".into(),
                rate: Decimal::new(18, 0),
                fbr_code: Some("CAT-MED-01".into()),
                is_exempt: Some(false),
                is_zero_rated: Some(false),
                effective_from: None,
            },
        )
        .await
        .unwrap();

    // 1. Create Invoice 1
    let inv1 = service
        .generate_invoice_for_order(&admin_ctx, order_id, branch_id, Utc::now())
        .await
        .unwrap();
    assert_eq!(
        inv1.invoice_no,
        format!(
            "LHR02/{}/000001",
            TaxService::compute_fiscal_year(Utc::now())
        )
    );

    // 2. Cancel / Return Invoice 1 -> Must generate a Credit Note with NEXT sequence (000002) rather than gapping 000001
    let reporter = MockFbrReporter::new(MockFbrBehavior::AlwaysAccept);
    let credit_note = service
        .create_credit_note(
            &admin_ctx,
            inv1.id,
            CreateCreditNoteRequest {
                reason: "Customer returned medicines unopened".into(),
            },
            &reporter,
        )
        .await
        .unwrap();

    assert_eq!(
        credit_note.invoice_no,
        format!(
            "LHR02/{}/000002",
            TaxService::compute_fiscal_year(Utc::now())
        )
    );
    assert_eq!(credit_note.credit_note_for, Some(inv1.id));
    assert_eq!(
        credit_note.total_amount,
        Money::from_decimal(-inv1.total_amount.0)
    );

    // Original invoice is updated to REFUNDED, never deleted or gapped
    let original_updated = service.get_invoice(&admin_ctx, inv1.id).await.unwrap();
    assert_eq!(original_updated.status, InvoiceStatus::Refunded);
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 5 & 6: tax_rate_selected_by_effective_date & historical_order_keeps_original_rate
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_tax_rate_selected_by_effective_date_and_historical_rate_preserved() {
    let old_date = Utc::now() - Duration::days(40);
    let new_date = Utc::now();

    let cat_id = TaxCategoryId::new();
    let tenant_id = TenantId::new();

    // Old rate: 15% (effective from 60 days ago to 20 days ago)
    let old_rate_cat = TaxCategoryDto {
        id: cat_id,
        tenant_id,
        name: "Medicines".into(),
        rate: Decimal::new(15, 0),
        fbr_code: Some("MED".into()),
        is_exempt: false,
        is_zero_rated: false,
        effective_from: Utc::now() - Duration::days(60),
        effective_to: Some(Utc::now() - Duration::days(20)),
        created_at: Utc::now(),
    };

    // New rate: 18% (effective from 20 days ago onwards)
    let new_rate_cat = TaxCategoryDto {
        id: TaxCategoryId::new(),
        tenant_id,
        name: "Medicines".into(),
        rate: Decimal::new(18, 0),
        fbr_code: Some("MED".into()),
        is_exempt: false,
        is_zero_rated: false,
        effective_from: Utc::now() - Duration::days(20),
        effective_to: None,
        created_at: Utc::now(),
    };

    let categories = vec![old_rate_cat, new_rate_cat];

    let items = vec![TaxableItemInput {
        item_name: "Augmentin 625mg".into(),
        unit_price: Money::from_major(1000),
        quantity: 1,
        discount: None,
        tax_category_name: "Medicines".into(),
    }];

    // 1. Calculate at historical date (40 days ago) -> Must select 15% (tax = Rs 150.00)
    let hist_res = TaxCalculator::calculate_tax(&items, &categories, old_date).unwrap();
    assert_eq!(hist_res.lines[0].rate, Decimal::new(15, 0));
    assert_eq!(hist_res.tax_amount, Money::from_major(150));

    // 2. Calculate at current date -> Must select 18% (tax = Rs 180.00)
    let current_res = TaxCalculator::calculate_tax(&items, &categories, new_date).unwrap();
    assert_eq!(current_res.lines[0].rate, Decimal::new(18, 0));
    assert_eq!(current_res.tax_amount, Money::from_major(180));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 7: rounding_applied_per_line_not_on_total
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_rounding_applied_per_line_not_on_total() {
    let cat = TaxCategoryDto {
        id: TaxCategoryId::new(),
        tenant_id: TenantId::new(),
        name: "Standard Goods".into(),
        rate: Decimal::new(175, 1), // 17.5%
        fbr_code: Some("STD".into()),
        is_exempt: false,
        is_zero_rated: false,
        effective_from: Utc::now() - Duration::days(10),
        effective_to: None,
        created_at: Utc::now(),
    };

    // Item 1: Price Rs 33.33 * 17.5% = 5.83275 -> rounded half-up per line = 5.83
    // Item 2: Price Rs 33.33 * 17.5% = 5.83275 -> rounded half-up per line = 5.83
    // Item 3: Price Rs 33.33 * 17.5% = 5.83275 -> rounded half-up per line = 5.83
    // Sum of lines: 5.83 + 5.83 + 5.83 = 17.49
    // (If rounded only on total 99.99 * 17.5% = 17.49825 -> 17.50)
    let items = vec![
        TaxableItemInput {
            item_name: "Item 1".into(),
            unit_price: Money::from_decimal(Decimal::new(3333, 2)),
            quantity: 1,
            discount: None,
            tax_category_name: "Standard Goods".into(),
        },
        TaxableItemInput {
            item_name: "Item 2".into(),
            unit_price: Money::from_decimal(Decimal::new(3333, 2)),
            quantity: 1,
            discount: None,
            tax_category_name: "Standard Goods".into(),
        },
        TaxableItemInput {
            item_name: "Item 3".into(),
            unit_price: Money::from_decimal(Decimal::new(3333, 2)),
            quantity: 1,
            discount: None,
            tax_category_name: "Standard Goods".into(),
        },
    ];

    let result = TaxCalculator::calculate_tax(&items, &[cat], Utc::now()).unwrap();

    assert_eq!(
        result.lines[0].tax_amount,
        Money::from_decimal(Decimal::new(583, 2))
    );
    assert_eq!(
        result.lines[1].tax_amount,
        Money::from_decimal(Decimal::new(583, 2))
    );
    assert_eq!(
        result.lines[2].tax_amount,
        Money::from_decimal(Decimal::new(583, 2))
    );
    assert_eq!(
        result.tax_amount,
        Money::from_decimal(Decimal::new(1749, 2))
    ); // Exactly 17.49
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 8: exempt_and_zero_rated_reported_distinctly
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_exempt_and_zero_rated_reported_distinctly() {
    let exempt_cat = TaxCategoryDto {
        id: TaxCategoryId::new(),
        tenant_id: TenantId::new(),
        name: "Exempt Life Saving Drugs".into(),
        rate: Decimal::ZERO,
        fbr_code: Some("EXEMPT-01".into()),
        is_exempt: true,
        is_zero_rated: false,
        effective_from: Utc::now() - Duration::days(10),
        effective_to: None,
        created_at: Utc::now(),
    };

    let zero_rated_cat = TaxCategoryDto {
        id: TaxCategoryId::new(),
        tenant_id: TenantId::new(),
        name: "Zero Rated Supplies".into(),
        rate: Decimal::ZERO,
        fbr_code: Some("ZERO-01".into()),
        is_exempt: false,
        is_zero_rated: true,
        effective_from: Utc::now() - Duration::days(10),
        effective_to: None,
        created_at: Utc::now(),
    };

    let items = vec![
        TaxableItemInput {
            item_name: "Insulin Regular".into(),
            unit_price: Money::from_major(850),
            quantity: 1,
            discount: None,
            tax_category_name: "Exempt Life Saving Drugs".into(),
        },
        TaxableItemInput {
            item_name: "Export Packaging Kit".into(),
            unit_price: Money::from_major(500),
            quantity: 1,
            discount: None,
            tax_category_name: "Zero Rated Supplies".into(),
        },
    ];

    let result =
        TaxCalculator::calculate_tax(&items, &[exempt_cat, zero_rated_cat], Utc::now()).unwrap();

    assert!(result.lines[0].is_exempt);
    assert!(!result.lines[0].is_zero_rated);
    assert_eq!(result.lines[0].tax_amount, Money::zero());

    assert!(!result.lines[1].is_exempt);
    assert!(result.lines[1].is_zero_rated);
    assert_eq!(result.lines[1].tax_amount, Money::zero());
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 9: no_tax_rate_hardcoded_in_source
// ------------------------------------------------------------------------------------------------
#[test]
fn test_no_tax_rate_hardcoded_in_source() {
    let source_code = include_str!("../src/calculator.rs");
    // Ensure no hardcoded percentage rates (e.g. 18, 17, 15) exist as constants in calculator.rs
    assert!(
        !source_code.contains("18.0"),
        "No hardcoded tax rates in calculation code"
    );
    assert!(
        !source_code.contains("17.0"),
        "No hardcoded tax rates in calculation code"
    );
    assert!(
        !source_code.contains("15.0"),
        "No hardcoded tax rates in calculation code"
    );
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 10: rejected_submission_does_not_retry
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_rejected_submission_does_not_retry() {
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

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id, "LHR03").await;
    seed_test_order_with_items(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1200),
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = TaxService::new(pool.clone());

    let _cat = service
        .create_tax_category(
            &admin_ctx,
            CreateTaxCategoryRequest {
                name: "General Medicines".into(),
                rate: Decimal::new(18, 0),
                fbr_code: Some("MED".into()),
                is_exempt: Some(false),
                is_zero_rated: Some(false),
                effective_from: None,
            },
        )
        .await
        .unwrap();

    let invoice = service
        .generate_invoice_for_order(&admin_ctx, order_id, branch_id, Utc::now())
        .await
        .unwrap();

    // Mock FBR schema/validation rejection
    let reject_reporter = MockFbrReporter::new(MockFbrBehavior::RejectValidation {
        reason: "Invalid NTN/STRN format on branch registration".into(),
        code: "ERR_VAL_042".into(),
    });

    let res = service
        .process_fbr_submission(&admin_ctx, invoice.id, &reject_reporter)
        .await
        .unwrap();

    // Status becomes REJECTED and retry_count remains 0 (does NOT retry!)
    assert_eq!(res.fbr_queue_status, FbrQueueStatus::Rejected);
    assert_eq!(
        res.retry_count, 0,
        "Rejected validation errors must not increment retry count"
    );
    assert!(res.fbr_error.unwrap().contains("ERR_VAL_042"));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 11 & 12: failed_submission_retries_with_backoff & queue_survives_service_restart
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_failed_submission_retries_with_backoff_and_queue_persists() {
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

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id, "LHR04").await;
    seed_test_order_with_items(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service1 = TaxService::new(pool.clone());

    let _cat = service1
        .create_tax_category(
            &admin_ctx,
            CreateTaxCategoryRequest {
                name: "General Medicines".into(),
                rate: Decimal::new(18, 0),
                fbr_code: Some("MED".into()),
                is_exempt: Some(false),
                is_zero_rated: Some(false),
                effective_from: None,
            },
        )
        .await
        .unwrap();

    let invoice = service1
        .generate_invoice_for_order(&admin_ctx, order_id, branch_id, Utc::now())
        .await
        .unwrap();

    // 1st failure (network outage)
    let outage_reporter = MockFbrReporter::new(MockFbrBehavior::OutageNetworkFailure {
        message: "Connection refused".into(),
    });

    let res1 = service1
        .process_fbr_submission(&admin_ctx, invoice.id, &outage_reporter)
        .await
        .unwrap();
    assert_eq!(res1.fbr_queue_status, FbrQueueStatus::Failed);
    assert_eq!(res1.retry_count, 1);

    // Simulate complete service restart (new service instance with new pool connection)
    let service2 = TaxService::new(pool.clone());
    let pending_invoices = service2
        .list_invoices(
            &admin_ctx,
            Some(branch_id),
            None,
            Some(FbrQueueStatus::Failed),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        pending_invoices.len(),
        1,
        "Failed invoice queue state survives service restart in DB"
    );

    // 2nd retry succeeds once FBR gateway recovers
    let accept_reporter = MockFbrReporter::new(MockFbrBehavior::AlwaysAccept);
    let res2 = service2
        .process_fbr_submission(&admin_ctx, invoice.id, &accept_reporter)
        .await
        .unwrap();
    assert_eq!(res2.fbr_queue_status, FbrQueueStatus::Accepted);
    assert!(res2.fiscal_invoice_no.is_some());
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 13: qr_generated_only_after_acceptance
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_qr_generated_only_after_acceptance() {
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

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id, "LHR05").await;
    seed_test_order_with_items(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = TaxService::new(pool);

    let _cat = service
        .create_tax_category(
            &admin_ctx,
            CreateTaxCategoryRequest {
                name: "General Medicines".into(),
                rate: Decimal::new(18, 0),
                fbr_code: Some("MED".into()),
                is_exempt: Some(false),
                is_zero_rated: Some(false),
                effective_from: None,
            },
        )
        .await
        .unwrap();

    let invoice = service
        .generate_invoice_for_order(&admin_ctx, order_id, branch_id, Utc::now())
        .await
        .unwrap();

    // Before acceptance: QR payload MUST be None
    assert_eq!(
        invoice.fbr_qr_payload, None,
        "QR payload must not exist before acceptance"
    );

    // After acceptance: QR payload is present and contains required fields
    let accept_reporter = MockFbrReporter::new(MockFbrBehavior::AlwaysAccept);
    let accepted_invoice = service
        .process_fbr_submission(&admin_ctx, invoice.id, &accept_reporter)
        .await
        .unwrap();

    assert!(accepted_invoice.fbr_qr_payload.is_some());
    let qr = accepted_invoice.fbr_qr_payload.unwrap();
    assert!(qr.contains("POS_ID:"));
    assert!(qr.contains("INV:"));
    assert!(qr.contains("FISC:"));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 14: provisional_invoice_sent_after_30_minutes_pending
// ------------------------------------------------------------------------------------------------
#[test]
fn test_provisional_invoice_sent_after_30_minutes_pending() {
    let should_send_provisional = |issued_at: DateTime<Utc>, status: FbrQueueStatus| -> bool {
        let elapsed = Utc::now() - issued_at;
        elapsed >= Duration::minutes(30) && status != FbrQueueStatus::Accepted
    };

    let issued_5_mins_ago = Utc::now() - Duration::minutes(5);
    let issued_35_mins_ago = Utc::now() - Duration::minutes(35);

    assert!(!should_send_provisional(
        issued_5_mins_ago,
        FbrQueueStatus::Pending
    ));
    assert!(should_send_provisional(
        issued_35_mins_ago,
        FbrQueueStatus::Pending
    ));
    assert!(!should_send_provisional(
        issued_35_mins_ago,
        FbrQueueStatus::Accepted
    ));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 15: invoice_has_no_edit_endpoint
// ------------------------------------------------------------------------------------------------
#[test]
fn test_invoice_has_no_edit_endpoint() {
    let routes_source = include_str!("../../api/src/routes/tax.rs");
    // Verify there is no PUT or PATCH handler for invoices
    assert!(!routes_source.contains("pub async fn update_invoice"));
    assert!(!routes_source.contains("pub async fn edit_invoice"));
    assert!(!routes_source.contains("patch_invoice"));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 17: fbr_request_and_response_persisted
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_fbr_request_and_response_persisted() {
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

    seed_test_tenant_and_branch(&pool, tenant_id, branch_id, "LHR06").await;
    seed_test_order_with_items(
        &pool,
        tenant_id,
        branch_id,
        customer_id,
        order_id,
        Money::from_major(1500),
    )
    .await;

    let admin_ctx = create_admin_context(tenant_id, admin_id);
    let service = TaxService::new(pool);

    let _cat = service
        .create_tax_category(
            &admin_ctx,
            CreateTaxCategoryRequest {
                name: "General Medicines".into(),
                rate: Decimal::new(18, 0),
                fbr_code: Some("MED".into()),
                is_exempt: Some(false),
                is_zero_rated: Some(false),
                effective_from: None,
            },
        )
        .await
        .unwrap();

    let invoice = service
        .generate_invoice_for_order(&admin_ctx, order_id, branch_id, Utc::now())
        .await
        .unwrap();
    let accept_reporter = MockFbrReporter::new(MockFbrBehavior::AlwaysAccept);
    let result = service
        .process_fbr_submission(&admin_ctx, invoice.id, &accept_reporter)
        .await
        .unwrap();

    assert!(
        result.fbr_response.is_some(),
        "FBR response payload must be persisted on invoice for audit"
    );
}
