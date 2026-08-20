# Review Brief — Doc 11 (Payments, Gateways, OCR Fraud Engine, TID Ledger, COD & Reconciliation)

## Spec
`docs/11_PAYMENTS_AND_RECONCILIATION.md`

## What I built
- **Multi-Gateway Integration Engine** (`crates/payments/src/gateways/mod.rs`):
  - Pluggable `PaymentGateway` trait with `create_intent`, `verify_webhook`, `refund`, and `status`.
  - Concrete gateway adapters: JazzCash (HMAC SHA-256 with replay checking), EasyPaisa, Raast (instant transfer with IBAN and payment ref), and Aggregator (Safepay/PayFast).
- **Screenshot OCR & Fraud Detection Engine** (`crates/payments/src/ocr.rs`, `crates/payments/src/service.rs`):
  - 8 specific fraud checks (duplicate TID, amount mismatch, stale timestamp >72h, timestamp prior to order creation, phone sender reuse across distinct customers, edited image EXIF tags, low OCR confidence, unknown layout).
  - Architectural Invariant I-4 enforced: No screenshot is ever auto-approved or auto-rejected; fraud flags only assist human reviewer.
- **TID Ledger & Manual Review Workflow** (`crates/payments/src/service.rs`):
  - `transaction_id_ledger` recording on proof approval with idempotency and uniqueness constraints.
  - Reviewer approval/rejection operations with RBAC permission enforcement (`payment.approve`, `payment.reject`).
- **COD Rule Engine & Doorstep Refusal Handling** (`crates/payments/src/service.rs`):
  - Tenant-level configurable COD ceiling (default Rs 10,000).
  - Customer block check and doorstep refusal handler transitioning order to `FAILED` and triggering stock return.
- **Settlement Reconciliation Engine** (`crates/payments/src/service.rs`):
  - Daily reconciliation comparing payment ledger entries with gateway settlement reports, flagging discrepancies in both directions (`UNMATCHED_IN_SETTLEMENT`, `UNMATCHED_IN_LEDGER`, `AMOUNT_MISMATCH`).
- **Database Migrations** (`migrations/20260821000002_payment_extensions.sql`):
  - Forward-only migration adding `AWAITING_PROOF`, `UNDER_REVIEW`, `FAILED` to `payment_status`, `ocr_bank` to `payment_proofs`, `refund_reason` to `payments`, and creating `payment_reconciliations` with tenant-scoped RLS policies.
- **REST Endpoints & API Contract** (`crates/api/src/routes/payments.rs`, `contracts/openapi.json`, `apps/shared/src/api/schema.d.ts`):
  - 10 OpenAPI-annotated Axum endpoints matching Doc 11 §9.
  - Regenerated OpenAPI schema and typed client.

## Acceptance tests
Spec names 19 tests. I implemented all 19 tests in `crates/payments/tests/payment_acceptance_tests.rs`.
| Spec test name | My test | File |
|---|---|---|
| 1. `webhook_rejects_invalid_signature` | `test_webhook_rejects_invalid_signature` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 2. `webhook_rejects_replayed_timestamp` | `test_webhook_rejects_replayed_timestamp` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 3. `webhook_is_idempotent_on_gateway_ref` | `test_webhook_idempotency_and_amount_mismatch` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 4. `redirect_url_alone_never_confirms_payment` | `test_redirect_url_alone_never_confirms_payment` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 5. `amount_mismatch_on_webhook_does_not_confirm` | `test_webhook_idempotency_and_amount_mismatch` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 6. `no_screenshot_auto_approval_path_exists` | `test_no_screenshot_auto_approval_path_exists` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 7. `duplicate_tid_flagged_critical` | `test_tid_ledger_lifecycle_and_duplicate_flagging` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 8. `approved_proof_writes_tid_to_ledger` | `test_tid_ledger_lifecycle_and_duplicate_flagging` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 9. `second_proof_with_same_tid_flags_duplicate` | `test_tid_ledger_lifecycle_and_duplicate_flagging` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 10. `amount_mismatch_flagged_not_rejected` | `test_amount_mismatch_flagged_not_rejected` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 11. `timestamp_before_order_flagged` | `test_timestamp_before_order_flagged` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 12. `edited_image_exif_flagged` | `test_edited_image_exif_flagged` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 13. `sender_reused_across_customers_flagged` | `test_sender_reused_across_customers_flagged` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 14. `flags_never_cause_automatic_decision` | `test_flags_never_cause_automatic_decision` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 15. `cod_limit_blocks_order_above_ceiling` | `test_cod_limit_blocks_order_above_ceiling` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 16. `cod_refusal_marks_failed_and_triggers_return` | `test_cod_refusal_marks_failed_and_triggers_return` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 17. `refund_requires_permission` | `test_refund_requires_permission` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 18. `adding_a_gateway_requires_no_orders_crate_change` | `test_adding_a_gateway_requires_no_orders_crate_change` | `crates/payments/tests/payment_acceptance_tests.rs` |
| 19. `reconciliation_flags_unmatched_both_directions` | `test_reconciliation_flags_unmatched_both_directions` | `crates/payments/tests/payment_acceptance_tests.rs` |

Missing: None.

## Out of scope
Confirmed nothing from the Out of scope section was built:
- No real production gateway accounts or merchant live keys used (all mock/test harnesses).
- No production ML OCR inference model bundled; clear `PaymentOcrProvider` trait boundary with deterministic heuristics in tests.
- No direct FBR fiscalization logic (isolated strictly in Spec 13 `shifa-tax`).
- No direct rider cash collection handling logic (isolated in Spec 12 `shifa-fulfilment`).

## ASSUMPTIONS
- Webhook signature header names follow gateway conventions (`x-jazzcash-signature`, `x-easypaisa-signature`, `x-safepay-signature`).
- Default COD ceiling is Rs 10,000 unless overridden in tenant settings.
- Direct deposits / screenshot payments use `PaymentMethod::DirectDeposit`.

## Known gaps
None.

## Contract changes
- Routes added:
  - `POST /api/v1/payments/intent`
  - `POST /api/v1/payments/webhook/{gateway}`
  - `POST /api/v1/payments/proof`
  - `GET /api/v1/payments/proof/queue`
  - `GET /api/v1/payments/proof/{id}`
  - `POST /api/v1/payments/proof/{id}/approve`
  - `POST /api/v1/payments/proof/{id}/reject`
  - `POST /api/v1/payments/{id}/refund`
  - `POST /api/v1/payments/cod/refusal`
  - `POST /api/v1/payments/reconciliation`
- `contracts/openapi.json` regenerated: Yes
- `apps/shared/src/api/schema.d.ts` regenerated: Yes

## Risk areas
- In production, webhook raw payload byte buffer preservation is required for exact HMAC validation before deserialization (implemented via `axum::body::Bytes`).
- TID ledger relies on unique indexes per `(tenant_id, gateway, transaction_id)`.
