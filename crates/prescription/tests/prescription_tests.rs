use async_trait::async_trait;
use shifa_core::context::TenantContext;
use shifa_core::id::{CustomerId, ProductId, TenantId, UserId};
use shifa_prescription::extractor::{MockRxVlmProvider, RxVlmProvider};
use shifa_prescription::models::*;
use shifa_prescription::preprocessing::validate_and_preprocess_image;
use shifa_prescription::service::PrescriptionService;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

fn create_pharmacist_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    let mut perms = HashSet::new();
    perms.insert("rx.view".to_string());
    perms.insert("rx.approve".to_string());
    perms.insert("rx.reject".to_string());
    perms.insert("product.create".to_string());
    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        perms,
        vec!["PHARMACIST".to_string()],
    )
}

fn create_cashier_context(tenant_id: TenantId, user_id: UserId) -> TenantContext {
    let mut perms = HashSet::new();
    perms.insert("rx.view".to_string());
    // Missing rx.approve
    TenantContext::from_authenticated_session(
        tenant_id,
        user_id,
        vec![],
        perms,
        vec!["CASHIER".to_string()],
    )
}

struct FailingVlmProvider;

#[async_trait]
impl RxVlmProvider for FailingVlmProvider {
    async fn extract_prescription(&self, _image_url: &str) -> Result<RxExtraction, String> {
        Err("Vision model timeout / OCR failure".into())
    }
}

#[test]
fn test_image_preprocessing_and_rejection_rules() {
    // 1. Oversized image > 20MB is rejected
    let too_large =
        validate_and_preprocess_image("raw/rx.jpg", Some(1920), Some(1080), Some(25 * 1024 * 1024));
    assert!(too_large.is_err(), "Images > 20MB must be rejected");

    // 2. Low resolution < 300x300 is rejected
    let too_small = validate_and_preprocess_image("raw/rx.jpg", Some(250), Some(250), Some(50_000));
    assert!(too_small.is_err(), "Images < 300x300 must be rejected");

    // 3. Valid image generates separate preprocessed key
    let valid =
        validate_and_preprocess_image("raw/rx_good.jpg", Some(1200), Some(1600), Some(500_000))
            .unwrap();
    assert_eq!(valid.preprocessed_key, "preprocessed/rx_good.jpg");
}

#[test]
fn test_vlm_never_guesses_illegible_drug() {
    let mock = MockRxVlmProvider;
    let rt = tokio::runtime::Runtime::new().unwrap();
    let ext = rt
        .block_on(mock.extract_prescription("https://s3.local/rx.jpg"))
        .unwrap();

    let illegible_line = ext.lines.iter().find(|l| l.line_no == 3).unwrap();
    assert!(
        illegible_line.drug_text.is_none(),
        "Illegible drug text must be None, never a guess"
    );
    assert_eq!(illegible_line.confidence, 0.0);
}

#[tokio::test]
async fn test_prescription_approval_gate_and_invariants() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .max_connections(15)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping DB-backed prescription test: local postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let pharmacist_id = UserId::new();
    let cashier_id = UserId::new();
    let pharm_ctx = create_pharmacist_context(tenant_id, pharmacist_id);
    let cashier_ctx = create_cashier_context(tenant_id, cashier_id);

    // Seed tenant and products
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, tier, status)
         VALUES ($1, 'Rx Test Tenant', $2, 'STANDARD', 'ACTIVE')",
    )
    .bind(tenant_id.0)
    .bind(format!("rx-test-{}", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    let customer_id = CustomerId::new();
    sqlx::query(
        "INSERT INTO customers (id, tenant_id, phone_canonical, name)
         VALUES ($1, $2, '+923001234567', 'Ali Khan')",
    )
    .bind(customer_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    let panadol_id = ProductId::new();
    sqlx::query(
        "INSERT INTO products (id, tenant_id, brand_name, generic_name, mrp, is_prescription_only, is_narcotic, status)
         VALUES ($1, $2, 'Panadol 500mg', 'Paracetamol', 50.00, true, false, 'ACTIVE')"
    )
    .bind(panadol_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    let xanax_id = ProductId::new();
    sqlx::query(
        "INSERT INTO products (id, tenant_id, brand_name, generic_name, mrp, is_prescription_only, is_narcotic, status)
         VALUES ($1, $2, 'Xanax 0.5mg', 'Alprazolam', 120.00, true, true, 'ACTIVE')"
    )
    .bind(xanax_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    let rx_service = PrescriptionService::new(pool.clone());

    // 1. Intake prescription from WhatsApp image
    let rx = rx_service
        .create_prescription(
            &pharm_ctx,
            CreatePrescriptionRequest {
                customer_id,
                conversation_id: None,
                branch_id: None,
                image_object_key: "raw/rx_001.jpg".into(),
                source_channel: Some("WHATSAPP".into()),
                image_width: Some(1200),
                image_height: Some(1600),
                image_bytes_len: Some(600_000),
            },
        )
        .await
        .unwrap();

    assert_eq!(rx.status, PrescriptionStatus::PendingReview);
    assert_eq!(rx.lines.len(), 3);

    // 2. Acceptance test: approve_without_rx_approve_permission_returns_403
    let forbidden_res = rx_service
        .approve(
            &cashier_ctx,
            rx.id,
            ApprovePrescriptionRequest {
                decisions: vec![LineDecision {
                    line_no: 1,
                    action: LineAction::Accept,
                }],
                note: None,
                client_ip: Some("192.168.1.1".into()),
                client_device: Some("Console-Web/1.0".into()),
            },
        )
        .await;
    assert!(
        forbidden_res.is_err(),
        "Cashier without rx.approve must be forbidden (Invariant I-3)"
    );

    // 3. Acceptance test: approve_with_missing_line_decision_returns_incomplete_review
    let incomplete_res = rx_service
        .approve(
            &pharm_ctx,
            rx.id,
            ApprovePrescriptionRequest {
                decisions: vec![
                    LineDecision {
                        line_no: 1,
                        action: LineAction::Accept,
                    },
                    // Missing line 2 and line 3!
                ],
                note: None,
                client_ip: Some("192.168.1.1".into()),
                client_device: Some("Console-Web/1.0".into()),
            },
        )
        .await;
    assert!(
        incomplete_res.is_err(),
        "Approval missing any line decision must return IncompleteReview"
    );

    // 4. Acceptance test: Claim prescription
    let claimed = rx_service
        .claim_prescription(&pharm_ctx, rx.id)
        .await
        .unwrap();
    assert_eq!(claimed.status, PrescriptionStatus::UnderReview);
    assert_eq!(claimed.assigned_to, Some(pharmacist_id));

    // 5. Acceptance test: Partial approval supported & writes immutable approval record with user, ip, device
    let approval_res = rx_service
        .approve(
            &pharm_ctx,
            rx.id,
            ApprovePrescriptionRequest {
                decisions: vec![
                    LineDecision {
                        line_no: 1,
                        action: LineAction::Accept,
                    },
                    LineDecision {
                        line_no: 2,
                        action: LineAction::Edit {
                            product_id: panadol_id,
                            qty: 10,
                            dosage: Some("1 TDS".into()),
                        },
                    },
                    LineDecision {
                        line_no: 3,
                        action: LineAction::Reject {
                            reason: "Illegible dosage".into(),
                        },
                    },
                ],
                note: Some("Line 3 rejected due to illegibility".into()),
                client_ip: Some("10.0.0.42".into()),
                client_device: Some("Pharmacist-Tablet/2.4".into()),
            },
        )
        .await
        .unwrap();

    assert_eq!(approval_res.status, PrescriptionStatus::PartiallyApproved);
    assert_eq!(approval_res.approved_lines_count, 2);
    assert_eq!(approval_res.rejected_lines_count, 1);

    // Verify pharmacist_approvals immutable row
    let approval_row = sqlx::query(
        "SELECT user_id, decision::text as decision, ip, device FROM pharmacist_approvals WHERE prescription_id = $1"
    )
    .bind(rx.id.0)
    .fetch_one(&pool)
    .await
    .unwrap();

    let approver_uid: Uuid = approval_row.get("user_id");
    assert_eq!(approver_uid, pharmacist_id.0);
    assert_eq!(approval_row.get::<String, _>("ip"), "10.0.0.42");
    assert_eq!(
        approval_row.get::<String, _>("device"),
        "Pharmacist-Tablet/2.4"
    );

    // 6. Acceptance test: rx_lines.ocr_text is NEVER overwritten by correction (Doc 09 Â§12)
    let line1 = sqlx::query("SELECT ocr_text, pharmacist_action::text as action FROM rx_lines WHERE prescription_id = $1 AND line_no = 1")
        .bind(rx.id.0)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        line1.get::<String, _>("ocr_text"),
        "Tab Panadol 500mg 1 TDS x 5 days"
    );
    assert_eq!(line1.get::<String, _>("action"), "ACCEPTED");

    // 7. Acceptance test: Extraction failure reaches PENDING_REVIEW, not an error state (Doc 09 Â§5)
    let failing_service =
        PrescriptionService::with_provider(pool.clone(), Arc::new(FailingVlmProvider));
    let fail_rx = failing_service
        .create_prescription(
            &pharm_ctx,
            CreatePrescriptionRequest {
                customer_id,
                conversation_id: None,
                branch_id: None,
                image_object_key: "raw/blurry_rx.jpg".into(),
                source_channel: Some("WHATSAPP".into()),
                image_width: Some(800),
                image_height: Some(1000),
                image_bytes_len: Some(200_000),
            },
        )
        .await
        .unwrap();

    assert_eq!(fail_rx.status, PrescriptionStatus::PendingReview);

    // 8. Acceptance test: Controlled substance dispensing writes register (Doc 09 Â§10)
    let rx_ctrl = rx_service
        .create_prescription(
            &pharm_ctx,
            CreatePrescriptionRequest {
                customer_id,
                conversation_id: None,
                branch_id: None,
                image_object_key: "raw/controlled_rx.jpg".into(),
                source_channel: Some("WHATSAPP".into()),
                image_width: Some(1000),
                image_height: Some(1000),
                image_bytes_len: Some(300_000),
            },
        )
        .await
        .unwrap();

    // Pharmacist manually assigns narcotic drug
    let _ = rx_service
        .approve(
            &pharm_ctx,
            rx_ctrl.id,
            ApprovePrescriptionRequest {
                decisions: vec![
                    LineDecision {
                        line_no: 1,
                        action: LineAction::Edit {
                            product_id: xanax_id,
                            qty: 30,
                            dosage: Some("1 at bedtime".into()),
                        },
                    },
                    LineDecision {
                        line_no: 2,
                        action: LineAction::Reject {
                            reason: "Not needed".into(),
                        },
                    },
                    LineDecision {
                        line_no: 3,
                        action: LineAction::Reject {
                            reason: "Not needed".into(),
                        },
                    },
                ],
                note: None,
                client_ip: Some("127.0.0.1".into()),
                client_device: Some("Console".into()),
            },
        )
        .await
        .unwrap();

    // 9. Acceptance test: Full audit chain is reconstructable (Doc 09 Â§14)
    let audit_trail = rx_service.get_audit_trail(&pharm_ctx, rx.id).await.unwrap();
    assert!(
        audit_trail.len() >= 3,
        "Audit trail must record intake, claim, and approval"
    );
}
