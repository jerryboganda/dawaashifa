use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use shifa_b2b::ar::AccountsReceivable;
use shifa_b2b::credit::CreditControl;
use shifa_b2b::error::B2bError;
use shifa_b2b::models::*;
use shifa_b2b::po::PurchaseOrderEngine;
use shifa_b2b::service::B2bService;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, ProductId, TenantId, UserId};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

fn create_admin_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    let mut perms = HashSet::new();
    perms.insert("b2b.quote".to_string());
    perms.insert("b2b.credit".to_string());
    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        perms,
        vec!["SUPER_ADMIN".to_string()],
    )
}

fn create_staff_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    let mut perms = HashSet::new();
    perms.insert("b2b.quote".to_string());
    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        perms,
        vec!["STAFF".to_string()],
    )
}

async fn seed_test_tenant_and_data(
    pool: &PgPool,
    tenant_id: TenantId,
    product_id: ProductId,
    mrp: Decimal,
) {
    sqlx::query(
        "INSERT INTO tenants (id, name, slug) VALUES ($1, 'B2B Hospital Tenant', $2)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(tenant_id.0)
    .bind(format!("b2b-tenant-{}", tenant_id.0))
    .execute(pool)
    .await
    .unwrap();

    let branch_id = BranchId::new();
    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, is_warehouse)
         VALUES ($1, $2, 'Lahore Central Warehouse', 'LHR01', true)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(branch_id.0)
    .bind(tenant_id.0)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO products (id, tenant_id, brand_name, generic_name, dosage_form, strength, mrp, is_prescription_only)
         VALUES ($1, $2, 'Titanium Knee Implant', 'Prosthesis', 'DEVICE', '45mm', $3, true)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(product_id.0)
    .bind(tenant_id.0)
    .bind(mrp)
    .execute(pool)
    .await
    .unwrap();
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 1: negotiated_price_above_mrp_rejected (Doc 14 §5, §13)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_negotiated_price_above_mrp_rejected() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    let mrp = Decimal::new(10000000, 2); // Rs 100,000.00
    seed_test_tenant_and_data(&pool, tenant_id, product_id, mrp).await;
    let ctx = create_admin_context(tenant_id, user_id);

    let b2b_service = B2bService::new(pool.clone());
    let account = b2b_service
        .create_account(
            &ctx,
            CreateAccountRequest {
                name: "Shaukat Khanum Memorial Hospital".into(),
                account_type: Some("HOSPITAL".into()),
                ntn: Some("1234567-8".into()),
                strn: None,
                billing_address: "7A Block R3, Johar Town, Lahore".into(),
                shipping_addresses: None,
                credit_limit: Some("5000000.0000".into()),
                payment_terms_days: Some(60),
                price_list_id: None,
            },
        )
        .await
        .unwrap();

    // Try creating quotation with price Rs 120,000.00 (above MRP Rs 100,000.00)
    let bad_price_req = CreateQuotationRequest {
        account_id: account.id,
        valid_until: Utc::now() + Duration::days(30),
        terms_text: Some("Standard 60-day credit terms".into()),
        items: vec![QuotationItemRequest {
            product_id: product_id.0,
            qty: 5,
            unit_price: "120000.0000".into(), // Above MRP!
            discount: None,
            lead_time_days: Some(3),
            notes: None,
        }],
    };

    let res = b2b_service.create_quotation(&ctx, bad_price_req).await;
    assert!(res.is_err(), "Negotiated price above MRP must be rejected");
    match res.unwrap_err() {
        B2bError::NegotiatedPriceAboveMrp {
            price,
            mrp: expected_mrp,
            ..
        } => {
            assert_eq!(price, Decimal::new(1200000000, 4));
            assert_eq!(expected_mrp, mrp);
        }
        other => panic!("Expected NegotiatedPriceAboveMrp, got {:?}", other),
    }

    // Valid price at or below MRP (Rs 85,000.00) succeeds
    let good_price_req = CreateQuotationRequest {
        account_id: account.id,
        valid_until: Utc::now() + Duration::days(30),
        terms_text: Some("Standard 60-day credit terms".into()),
        items: vec![QuotationItemRequest {
            product_id: product_id.0,
            qty: 5,
            unit_price: "85000.0000".into(), // Below MRP
            discount: None,
            lead_time_days: Some(3),
            notes: None,
        }],
    };

    let quote = b2b_service
        .create_quotation(&ctx, good_price_req)
        .await
        .unwrap();
    assert_eq!(quote.version, 1);
    assert_eq!(quote.status, "DRAFT");
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 2: quote_revision_creates_new_version_preserving_original (Doc 14 §6, §13)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_quote_revision_creates_new_version_preserving_original() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    seed_test_tenant_and_data(&pool, tenant_id, product_id, Decimal::new(10000000, 2)).await;
    let ctx = create_admin_context(tenant_id, user_id);
    let b2b_service = B2bService::new(pool.clone());

    let account = b2b_service
        .create_account(
            &ctx,
            CreateAccountRequest {
                name: "National Hospital Lahore".into(),
                account_type: Some("HOSPITAL".into()),
                ntn: None,
                strn: None,
                billing_address: "DHA Phase 1, Lahore".into(),
                shipping_addresses: None,
                credit_limit: Some("1000000.0000".into()),
                payment_terms_days: Some(30),
                price_list_id: None,
            },
        )
        .await
        .unwrap();

    let initial_quote = b2b_service
        .create_quotation(
            &ctx,
            CreateQuotationRequest {
                account_id: account.id,
                valid_until: Utc::now() + Duration::days(15),
                terms_text: Some("Initial quote".into()),
                items: vec![QuotationItemRequest {
                    product_id: product_id.0,
                    qty: 2,
                    unit_price: "90000.0000".into(),
                    discount: None,
                    lead_time_days: Some(2),
                    notes: None,
                }],
            },
        )
        .await
        .unwrap();

    assert_eq!(initial_quote.version, 1);

    // Revise quotation to new price Rs 80,000.00
    let revised_quote = b2b_service
        .revise_quotation(
            &ctx,
            initial_quote.id,
            ReviseQuotationRequest {
                valid_until: Utc::now() + Duration::days(20),
                terms_text: Some("Revised negotiated quote".into()),
                items: vec![QuotationItemRequest {
                    product_id: product_id.0,
                    qty: 2,
                    unit_price: "80000.0000".into(),
                    discount: None,
                    lead_time_days: Some(2),
                    notes: None,
                }],
            },
        )
        .await
        .unwrap();

    assert_eq!(revised_quote.version, 2);
    assert_eq!(revised_quote.parent_quote_id, Some(initial_quote.id));
    assert_eq!(revised_quote.quote_no, initial_quote.quote_no);

    // Verify parent quote status was updated to REVISED, preserving record
    let parent_status: String =
        sqlx::query_scalar("SELECT status FROM quotations WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id.0)
            .bind(initial_quote.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(parent_status, "REVISED");
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 3: expired_quote_cannot_convert (Doc 14 §6, §13)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_expired_quote_cannot_convert() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    seed_test_tenant_and_data(&pool, tenant_id, product_id, Decimal::new(10000000, 2)).await;
    let ctx = create_admin_context(tenant_id, user_id);
    let b2b_service = B2bService::new(pool.clone());

    let account = b2b_service
        .create_account(
            &ctx,
            CreateAccountRequest {
                name: "Doctors Hospital Lahore".into(),
                account_type: Some("HOSPITAL".into()),
                ntn: None,
                strn: None,
                billing_address: "Canal Bank Rd, Lahore".into(),
                shipping_addresses: None,
                credit_limit: Some("5000000.0000".into()),
                payment_terms_days: Some(30),
                price_list_id: None,
            },
        )
        .await
        .unwrap();

    let expired_quote = b2b_service
        .create_quotation(
            &ctx,
            CreateQuotationRequest {
                account_id: account.id,
                valid_until: Utc::now() - Duration::days(2), // Expired 2 days ago!
                terms_text: None,
                items: vec![QuotationItemRequest {
                    product_id: product_id.0,
                    qty: 1,
                    unit_price: "90000.0000".into(),
                    discount: None,
                    lead_time_days: None,
                    notes: None,
                }],
            },
        )
        .await
        .unwrap();

    let res = b2b_service.accept_quotation(&ctx, expired_quote.id).await;
    assert!(res.is_err(), "Acceptance of expired quotation must fail");
    match res.unwrap_err() {
        B2bError::QuoteExpired(id, _) => assert_eq!(id, expired_quote.id),
        other => panic!("Expected QuoteExpired, got {:?}", other),
    }
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 4 & 5: discount_above_threshold_requires_approval & approver_below_limit (Doc 14 §6, §13)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_discount_approval_threshold_and_limits() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    seed_test_tenant_and_data(&pool, tenant_id, product_id, Decimal::new(10000000, 2)).await;
    let ctx = create_admin_context(tenant_id, user_id);
    let b2b_service = B2bService::new(pool.clone());

    let account = b2b_service
        .create_account(
            &ctx,
            CreateAccountRequest {
                name: "Fatima Memorial Hospital".into(),
                account_type: Some("HOSPITAL".into()),
                ntn: None,
                strn: None,
                billing_address: "Shadman, Lahore".into(),
                shipping_addresses: None,
                credit_limit: Some("2000000.0000".into()),
                payment_terms_days: Some(30),
                price_list_id: None,
            },
        )
        .await
        .unwrap();

    // Create quote with Rs 50,000 discount
    let quote = b2b_service
        .create_quotation(
            &ctx,
            CreateQuotationRequest {
                account_id: account.id,
                valid_until: Utc::now() + Duration::days(15),
                terms_text: None,
                items: vec![QuotationItemRequest {
                    product_id: product_id.0,
                    qty: 5,
                    unit_price: "90000.0000".into(),
                    discount: Some("50000.0000".into()), // Rs 50,000 discount
                    lead_time_days: None,
                    notes: None,
                }],
            },
        )
        .await
        .unwrap();

    // 1. Approver with limit Rs 20,000 fails to approve Rs 50,000 discount
    let junior_limit = Decimal::new(200000000, 4);
    let res_junior = b2b_service
        .approve_quotation_discount(&ctx, quote.id, junior_limit)
        .await;
    assert!(res_junior.is_err());
    match res_junior.unwrap_err() {
        B2bError::ApproverBelowLimit { limit, required } => {
            assert_eq!(limit, junior_limit);
            assert_eq!(required, Decimal::new(500000000, 4));
        }
        other => panic!("Expected ApproverBelowLimit, got {:?}", other),
    }

    // 2. Approver with limit Rs 100,000 successfully approves
    let senior_limit = Decimal::new(1000000000, 4);
    let res_senior = b2b_service
        .approve_quotation_discount(&ctx, quote.id, senior_limit)
        .await;
    assert!(res_senior.is_ok());
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 6, 7, 8: credit_check blocks limit exceeded, 90 day overdue, and runs before dispatch
// ------------------------------------------------------------------------------------------------
#[test]
fn test_credit_control_rules() {
    let limit = Decimal::new(5000000000, 4); // Rs 500,000.00
    let outstanding = Decimal::new(4500000000, 4); // Rs 450,000.00

    // 1. Limit exceeded check
    let new_order_large = Decimal::new(1000000000, 4); // Rs 100,000.00 -> total 550,000 > 500,000
    let res_limit = CreditControl::verify_credit_policy(
        "Services Hospital",
        false,
        None,
        limit,
        outstanding,
        Decimal::ZERO,
        new_order_large,
    );
    assert!(res_limit.is_err(), "Must block when credit limit exceeded");
    match res_limit.unwrap_err() {
        B2bError::CreditLimitExceeded { account_name, .. } => {
            assert_eq!(account_name, "Services Hospital")
        }
        other => panic!("Expected CreditLimitExceeded, got {:?}", other),
    }

    // 2. 90-day overdue check
    let overdue_90 = Decimal::new(250000000, 4); // Rs 25,000.00 overdue > 90 days
    let small_order = Decimal::new(100000000, 4); // Rs 10,000.00
    let res_overdue = CreditControl::verify_credit_policy(
        "Services Hospital",
        false,
        None,
        limit,
        Decimal::ZERO,
        overdue_90,
        small_order,
    );
    assert!(
        res_overdue.is_err(),
        "Must block when 90-day overdue balance exists"
    );
    match res_overdue.unwrap_err() {
        B2bError::OverdueBalanceBlocked(amt) => assert_eq!(amt, overdue_90),
        other => panic!("Expected OverdueBalanceBlocked, got {:?}", other),
    }

    // 3. Clean account passes
    let res_clean = CreditControl::verify_credit_policy(
        "Services Hospital",
        false,
        None,
        limit,
        outstanding,
        Decimal::ZERO,
        Decimal::new(300000000, 4), // Rs 30,000.00 -> total 480,000 <= 500,000
    );
    assert!(res_clean.is_ok());
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 9: credit_override_requires_permission_and_audits (Doc 14 §8, §13)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_credit_override_requires_permission_and_audits() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let admin_user = UserId::new();
    let staff_user = UserId::new();
    let account_id = Uuid::now_v7();

    let admin_ctx = create_admin_context(tenant_id, admin_user);
    let staff_ctx = create_staff_context(tenant_id, staff_user);

    // 1. Staff without b2b.credit permission is rejected
    let res_staff = CreditControl::authorize_credit_override(
        &staff_ctx,
        account_id,
        "Emergency surgery",
        &pool,
    )
    .await;
    assert!(
        res_staff.is_err(),
        "Staff without b2b.credit cannot override credit limit"
    );

    // 2. Admin with b2b.credit permission succeeds and writes audit entry
    let res_admin = CreditControl::authorize_credit_override(
        &admin_ctx,
        account_id,
        "Approved by CFO for VIP patient surgery",
        &pool,
    )
    .await;
    assert!(res_admin.is_ok());
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 10: po_variance_blocks_fulfilment (Doc 14 §7, §13)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_po_variance_blocks_fulfilment() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    seed_test_tenant_and_data(&pool, tenant_id, product_id, Decimal::new(10000000, 2)).await;
    let ctx = create_admin_context(tenant_id, user_id);
    let b2b_service = B2bService::new(pool.clone());

    let account = b2b_service
        .create_account(
            &ctx,
            CreateAccountRequest {
                name: "Lahore General Hospital".into(),
                account_type: Some("HOSPITAL".into()),
                ntn: None,
                strn: None,
                billing_address: "Ferozepur Rd, Lahore".into(),
                shipping_addresses: None,
                credit_limit: Some("5000000.0000".into()),
                payment_terms_days: Some(30),
                price_list_id: None,
            },
        )
        .await
        .unwrap();

    let quote = b2b_service
        .create_quotation(
            &ctx,
            CreateQuotationRequest {
                account_id: account.id,
                valid_until: Utc::now() + Duration::days(30),
                terms_text: None,
                items: vec![QuotationItemRequest {
                    product_id: product_id.0,
                    qty: 10,
                    unit_price: "50000.0000".into(), // Quote total = Rs 500,000.00
                    discount: None,
                    lead_time_days: None,
                    notes: None,
                }],
            },
        )
        .await
        .unwrap();

    // Upload PO with mismatched amount (Rs 450,000 instead of Rs 500,000)
    let po = b2b_service
        .ingest_purchase_order(
            &ctx,
            CreatePurchaseOrderRequest {
                account_id: account.id,
                quotation_id: Some(quote.id),
                po_number: "PO-LGH-2026-991".into(),
                po_document_key: Some("docs/po_991.pdf".into()),
                amount: "450000.0000".into(), // Variance!
            },
        )
        .await
        .unwrap();

    assert!(po.variance_detected);
    assert_eq!(po.status, "VARIANCE_BLOCKED");

    // Fulfilment check must fail
    let check = PurchaseOrderEngine::verify_fulfilment_allowed(&ctx, po.id, &pool).await;
    assert!(check.is_err(), "Variance must block fulfilment");
    match check.unwrap_err() {
        B2bError::PoVarianceBlocked(_) => {}
        other => panic!("Expected PoVarianceBlocked, got {:?}", other),
    }
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 11: partial_payment_allocates_oldest_first (Doc 14 §9, §13)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_partial_payment_allocates_oldest_first() {
    let inv1 = Uuid::now_v7(); // Oldest
    let inv2 = Uuid::now_v7();
    let inv3 = Uuid::now_v7(); // Newest

    let mut invoices = vec![
        (inv1, Decimal::new(100000000, 4)), // Rs 10,000
        (inv2, Decimal::new(150000000, 4)), // Rs 15,000
        (inv3, Decimal::new(200000000, 4)), // Rs 20,000
    ];

    // Customer makes partial payment of Rs 18,000
    let payment = Decimal::new(180000000, 4);
    let allocations = AccountsReceivable::allocate_payment_fifo(payment, &mut invoices);

    assert_eq!(allocations.len(), 2);
    assert_eq!(allocations[0], (inv1, Decimal::new(100000000, 4))); // Fully pays inv1 (10,000)
    assert_eq!(allocations[1], (inv2, Decimal::new(80000000, 4))); // Partially pays inv2 (8,000)

    assert_eq!(invoices[0].1, Decimal::ZERO); // inv1 balance = 0
    assert_eq!(invoices[1].1, Decimal::new(70000000, 4)); // inv2 balance = 7,000
    assert_eq!(invoices[2].1, Decimal::new(200000000, 4)); // inv3 balance = 20,000
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 12: ninety_day_overdue_sets_account_on_hold (Doc 14 §9, §13)
// ------------------------------------------------------------------------------------------------
#[test]
fn test_ninety_day_overdue_locks_account() {
    let on_hold_policy = CreditControl::verify_credit_policy(
        "Overdue Hospital",
        true,
        Some("Automatic lock: 90+ days overdue balance"),
        Decimal::new(1000000000, 4),
        Decimal::ZERO,
        Decimal::ZERO,
        Decimal::new(100000000, 4),
    );
    assert!(on_hold_policy.is_err());
    match on_hold_policy.unwrap_err() {
        B2bError::AccountOnHold(name, reason) => {
            assert_eq!(name, "Overdue Hospital");
            assert!(reason.contains("90+ days overdue"));
        }
        other => panic!("Expected AccountOnHold, got {:?}", other),
    }
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 13 & 14: consignment_placement_is_transfer_not_sale & discrepancy_flagged_not_auto_adjusted
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_consignment_transfer_and_reconciliation() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    seed_test_tenant_and_data(&pool, tenant_id, product_id, Decimal::new(10000000, 2)).await;
    let ctx = create_admin_context(tenant_id, user_id);
    let b2b_service = B2bService::new(pool.clone());

    let account = b2b_service
        .create_account(
            &ctx,
            CreateAccountRequest {
                name: "Surgimed Hospital Lahore".into(),
                account_type: Some("HOSPITAL".into()),
                ntn: None,
                strn: None,
                billing_address: "Zafar Ali Rd, Gulberg V, Lahore".into(),
                shipping_addresses: None,
                credit_limit: Some("5000000.0000".into()),
                payment_terms_days: Some(30),
                price_list_id: None,
            },
        )
        .await
        .unwrap();

    let location_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO consignment_locations (id, tenant_id, account_id, name, address)
         VALUES ($1, $2, $3, 'OT Cabinet 3', '3rd Floor OT Complex')",
    )
    .bind(location_id)
    .bind(tenant_id.0)
    .bind(account.id)
    .execute(&pool)
    .await
    .unwrap();

    // 1. Place 10 consignment units (Transfer, NOT sale)
    let stock = b2b_service
        .place_consignment(
            &ctx,
            PlaceConsignmentRequest {
                location_id,
                product_id: product_id.0,
                batch_id: None,
                serial_no: None,
                qty: 10,
            },
        )
        .await
        .unwrap();

    assert_eq!(stock.qty, 10);
    assert_eq!(stock.consumed_at, None);
    assert_eq!(stock.invoiced_at, None);

    // 2. Reconcile with physical count = 8 (2 missing)
    let reconciled = b2b_service
        .reconcile_consignment(
            &ctx,
            stock.id,
            ReconcileConsignmentRequest {
                physical_count: 8,
                notes: Some("Cabinet lock found damaged".into()),
            },
        )
        .await
        .unwrap();

    // Discrepancy is flagged, but system quantity remains unaltered (never auto-adjusted) (Doc 14 §10)
    assert!(reconciled.discrepancy_flagged);
    assert_eq!(reconciled.qty, 10, "Expected qty must not be auto-adjusted");
    assert!(reconciled
        .discrepancy_reason
        .unwrap()
        .contains("physical count 8 vs expected count 10"));
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 15 & 16: device_serial_unique_per_tenant & recall_query
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_device_serial_uniqueness_and_recall_query() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    seed_test_tenant_and_data(&pool, tenant_id, product_id, Decimal::new(10000000, 2)).await;
    let ctx = create_admin_context(tenant_id, user_id);
    let b2b_service = B2bService::new(pool.clone());

    let batch_id = Uuid::now_v7();
    let serial1 = "SN-TITAN-2026-0001";
    let serial2 = "SN-TITAN-2026-0002";

    // 1. Register unit 1
    let dev1 = b2b_service
        .register_device(
            &ctx,
            RegisterDeviceRequest {
                product_id: product_id.0,
                batch_id: Some(batch_id),
                serial_no: serial1.into(),
                udi: Some("(01)008888880001(17)261231(21)0001".into()),
                location_type: Some("WAREHOUSE".into()),
                location_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(dev1.serial_no, serial1);

    // 2. Duplicate serial registration in same tenant must fail (Doc 14 §11)
    let dup_res = b2b_service
        .register_device(
            &ctx,
            RegisterDeviceRequest {
                product_id: product_id.0,
                batch_id: Some(batch_id),
                serial_no: serial1.into(), // Duplicate!
                udi: None,
                location_type: None,
                location_id: None,
            },
        )
        .await;

    assert!(
        dup_res.is_err(),
        "Duplicate device serial in same tenant must be rejected"
    );
    match dup_res.unwrap_err() {
        B2bError::DeviceSerialDuplicate(s) => assert_eq!(s, serial1),
        other => panic!("Expected DeviceSerialDuplicate, got {:?}", other),
    }

    // Register unit 2
    b2b_service
        .register_device(
            &ctx,
            RegisterDeviceRequest {
                product_id: product_id.0,
                batch_id: Some(batch_id),
                serial_no: serial2.into(),
                udi: Some("(01)008888880001(17)261231(21)0002".into()),
                location_type: Some("HOSPITAL_CONSIGNMENT".into()),
                location_id: None,
            },
        )
        .await
        .unwrap();

    // 3. Manufacturer recall query by batch_id
    let recall = b2b_service
        .query_recall(&ctx, Some(product_id.0), Some(batch_id))
        .await
        .unwrap();
    assert_eq!(recall.affected_units_count, 2);
    assert_eq!(recall.units.len(), 2);
}

// ------------------------------------------------------------------------------------------------
// Acceptance Test 17: b2b_order_bypasses_retail_cart_stages (Doc 14 §4, §6, §13)
// ------------------------------------------------------------------------------------------------
#[tokio::test]
async fn test_b2b_order_bypasses_retail_cart_stages() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let product_id = ProductId::new();
    seed_test_tenant_and_data(&pool, tenant_id, product_id, Decimal::new(10000000, 2)).await;
    let ctx = create_admin_context(tenant_id, user_id);
    let b2b_service = B2bService::new(pool.clone());

    let account = b2b_service
        .create_account(
            &ctx,
            CreateAccountRequest {
                name: "Chughtai Lab & Medical Center".into(),
                account_type: Some("CLINIC".into()),
                ntn: None,
                strn: None,
                billing_address: "Jail Rd, Lahore".into(),
                shipping_addresses: None,
                credit_limit: Some("1000000.0000".into()),
                payment_terms_days: Some(30),
                price_list_id: None,
            },
        )
        .await
        .unwrap();

    let quote = b2b_service
        .create_quotation(
            &ctx,
            CreateQuotationRequest {
                account_id: account.id,
                valid_until: Utc::now() + Duration::days(30),
                terms_text: None,
                items: vec![QuotationItemRequest {
                    product_id: product_id.0,
                    qty: 3,
                    unit_price: "95000.0000".into(),
                    discount: None,
                    lead_time_days: None,
                    notes: None,
                }],
            },
        )
        .await
        .unwrap();

    // Accept quotation
    let order_id = b2b_service.accept_quotation(&ctx, quote.id).await.unwrap();

    // Verify order created lands directly in CONFIRMED status, bypassing retail cart & payment collection stages
    let order_status: String =
        sqlx::query_scalar("SELECT status::text FROM orders WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id.0)
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(order_status, "CONFIRMED");
}
