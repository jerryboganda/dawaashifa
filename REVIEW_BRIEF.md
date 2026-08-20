# Review Brief — Doc 14: B2B Module: Quotes, Credit & AR Aging

## Spec
`docs/14_B2B_QUOTES_AND_CREDIT.md`

## What I built
- **Database Schema (`migrations/20260821000007_b2b_schema.sql`)**:
  - `business_accounts`, `business_contacts`, `price_lists`, `price_list_items`, `quotations`, `quotation_items`, `purchase_orders`, `consignment_locations`, `consignment_stock`, `device_units`.
  - All tables include `tenant_id UUID NOT NULL` and active Postgres RLS isolation policies (Invariants I-1, I-2).
- **Business Accounts & Contacts (`crates/b2b/src/service.rs`, `models.rs`)**:
  - Hospital and clinic business account profile management, credit limits, payment terms, and contacts with approval limit hierarchies (Doc 14 §5).
- **Quotation Engine (`crates/b2b/src/quotes.rs`)**:
  - Sequential numbering `Q-{BRANCH}-{YY}-{SEQ5}`.
  - MRP cap validation: rejects any negotiated price above product MRP (Doc 14 §5).
  - Versioned revisions: increments version, links `parent_quote_id`, and marks previous version as `REVISED` without mutating history (Doc 14 §6).
  - Discount approval gates: high discounts require approver with sufficient `approval_limit` (Doc 14 §6).
  - Expiry enforcement: expired quotations are blocked from acceptance and conversion.
  - B2B order conversion: accepted quotes directly create B2B orders at `CONFIRMED` status, bypassing retail carts (Doc 14 §4, §6).
- **Purchase Order Matching & Variance Detection (`crates/b2b/src/po.rs`)**:
  - Uploads and matches PO against quotation. Variance sets `status = VARIANCE_BLOCKED` and blocks fulfilment until resolved (Doc 14 §7).
- **Credit Control (`crates/b2b/src/credit.rs`)**:
  - Verifies credit limits, account hold flags, and 90+ days overdue balances before quote acceptance and before dispatch (Doc 14 §8).
  - Credit overrides require `b2b.credit` permission and write audited entries to `audit_logs` (Invariant I-9).
- **Accounts Receivable & Aging (`crates/b2b/src/ar.rs`)**:
  - Aging buckets: `Current`, `1-30`, `31-60`, `61-90`, `90+` days (Doc 14 §9).
  - Automatic account lock (`on_hold = true`) when 90+ days overdue balance exists.
  - FIFO partial payment allocation against oldest outstanding invoices.
- **Consignment Stock Management (`crates/b2b/src/consignment.rs`)**:
  - Placement records stock transfer to virtual hospital location, not a sale (Doc 14 §10).
  - Reconciliation flags discrepancies with audit notes without auto-adjusting system quantities.
- **Device Traceability & Recall Query (`crates/b2b/src/device.rs`)**:
  - Unit-level tracking for implants (UDI, lot, serial) with `UNIQUE (tenant_id, serial_no)` invariant (Doc 14 §11).
  - First-class manufacturer recall query returning all affected units, current locations, and patient references.
- **API & OpenAPI**:
  - 12 REST endpoints in `crates/api/src/routes/b2b.rs` matching Doc 14 §12.
  - Emitted `contracts/openapi.json` and regenerated `@shifa/shared` types.

## Acceptance tests
Spec names 17 acceptance tests. I implemented **17** across 12 test functions.

| Spec test name | My test | File |
|---|---|---|
| `negotiated_price_above_mrp_rejected` | `test_negotiated_price_above_mrp_rejected` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `quote_revision_creates_new_version_preserving_original` | `test_quote_revision_creates_new_version_preserving_original` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `expired_quote_cannot_convert` | `test_expired_quote_cannot_convert` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `discount_above_threshold_requires_approval` | `test_discount_approval_threshold_and_limits` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `approver_below_limit_cannot_approve` | `test_discount_approval_threshold_and_limits` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `credit_check_blocks_on_limit_exceeded` | `test_credit_control_rules` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `credit_check_blocks_on_90_day_overdue` | `test_credit_control_rules` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `credit_check_runs_again_before_dispatch` | `test_credit_control_rules` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `credit_override_requires_permission_and_audits` | `test_credit_override_requires_permission_and_audits` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `po_variance_blocks_fulfilment` | `test_po_variance_blocks_fulfilment` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `partial_payment_allocates_oldest_first` | `test_partial_payment_allocates_oldest_first` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `ninety_day_overdue_sets_account_on_hold` | `test_ninety_day_overdue_locks_account` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `consignment_placement_is_transfer_not_sale` | `test_consignment_transfer_and_reconciliation` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `consignment_discrepancy_flagged_not_auto_adjusted` | `test_consignment_transfer_and_reconciliation` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `device_serial_unique_per_tenant` | `test_device_serial_uniqueness_and_recall_query` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `recall_query_returns_all_affected_units_with_locations` | `test_device_serial_uniqueness_and_recall_query` | `crates/b2b/tests/b2b_acceptance_tests.rs` |
| `b2b_order_bypasses_retail_cart_stages` | `test_b2b_order_bypasses_retail_cart_stages` | `crates/b2b/tests/b2b_acceptance_tests.rs` |

Missing, with reason: None. All 17 acceptance assertions passing.

## Out of scope
Confirmed nothing from the Out of scope section was built:
- No public tender or e-procurement integration.
- No general ledger.
- No sales commission tracking.
- No retail order flow modifications.

## ASSUMPTIONS
- Orders created from B2B quotes use `CREDIT_TERMS` payment method.

## Known gaps
None.

## Contract changes
- Added B2B endpoints (`/api/v1/b2b/*`) and types to `contracts/openapi.json` and `@shifa/shared`.

## Risk areas
- Device serial numbers must be entered accurately during warehouse receipt to guarantee recall query fidelity.
