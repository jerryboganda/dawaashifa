# REVIEW_BRIEF.md — Spec 06 (Inventory Ledger, Batches, FEFO, Expiry, and Cold Chain)

## Spec Reference
- **Spec**: `docs/06_INVENTORY_LEDGER.md`
- **Branch**: `feat/06-inventory-ledger`

## Invariants Enforced
- **I-5 (Append-Only Stock Ledger)**: `stock_movements` is strictly append-only. No application code directly updates `stock_current.qty`. The PostgreSQL `apply_stock_movement` trigger projects movements into `stock_current` and raises an uncatchable exception rolling back transactions if stock would go negative.
- **FEFO Allocation & Safety Floor**: Allocation strictly sorts batches by `expiry_date ASC`, immediately excludes expired batches and batches below the minimum shelf-life floor (or patient course length), excludes quarantined batches, splits across batches as needed, and returns `Err(InsufficientStock)` without silent partial allocation.
- **Idempotent Reservations with TTL**: `reserve_stock` creates reservations with negative `RESERVATION` movements and TTL. A scheduled worker `release_expired_reservations` releases expired reservations with compensating `RELEASE` movements idempotently.
- **Inter-Branch Transfer Isolation**: In-transit stock is deducted from the source branch via `TRANSFER_OUT` and belongs to neither branch's available pool until `TRANSFER_IN` at receipt. Quantity mismatches trigger `DISCREPANCY` status requiring manual reconciliation.
- **Cold-Chain Monitoring & Pharmacist Clearance**: Products requiring refrigeration cannot be stored at incapable branches. Temperature logs outside 2–8°C trigger excursion alerts and immediately quarantine affected batches from allocation. Clearing an excursion requires `rx.approve` permission with a documented clinical decision note.
- **Concurrency Safety**: `allocate_fefo` uses PostgreSQL row-level locks (`SELECT ... FOR UPDATE OF sc`) to prevent concurrent overselling under heavy load.

## What Was Built
1. **Inventory Domain & Services (`crates/inventory`)**:
   - `InventoryService`: Stock receipt, adjustment with reason codes, write-offs, and multi-branch availability queries.
   - `fefo`: Concurrency-safe FEFO allocation with shelf-life floor and multi-batch splitting.
   - `reservations`: TTL reservations and idempotent release worker with compensating movements.
   - `transfers`: State machine (`DRAFT -> DISPATCHED -> IN_TRANSIT -> RECEIVED / DISCREPANCY`).
   - `cold_chain`: Temperature excursion logging, automated quarantine, and audited pharmacist clearance.
2. **Axum HTTP API & OpenAPI**:
   - `/api/v1/inventory/stock`
   - `/api/v1/inventory/receipts`
   - `/api/v1/inventory/adjustments`
   - `/api/v1/inventory/transfers`
   - `/api/v1/inventory/transfers/:id/dispatch`
   - `/api/v1/inventory/cold-chain/logs`
   - `/api/v1/inventory/cold-chain/:batch_id/clear-excursion`
   - Regenerated `contracts/openapi.json` and generated TypeScript client `@shifa/shared`.

## Acceptance Tests Verification
- `cargo test --workspace` passed 30 tests with 0 failures:
  - `test_inventory_ledger_and_fefo_suite` -> ok
  - `test_concurrent_allocation_does_not_oversell` -> ok
  - `test_urdu_phonetics_and_normalization_table` -> ok
  - `test_mrp_hard_block_enforcement` -> ok
  - `test_catalog_matching_and_substitutions_integration` -> ok
  - `test_rate_limiter_and_idempotency_prevention` -> ok
  - `test_choice_rendering_three_tiers` -> ok
  - `test_unknown_message_type_is_stored_as_unsupported` -> ok
  - `test_webhook_signature_verification` -> ok
  - `test_freeform_outside_window_fails_loudly` -> ok
  - `test_unapproved_template_fails_before_network_call` -> ok
  - `test_cloud_api_send_success_and_error_handling` -> ok
  - `test_api_auth_and_session_lifecycle` -> ok
  - `test_database_migrations_and_rls_suite` -> ok
- `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings.
- `cargo fmt --all --check` clean.
- `pnpm check && pnpm lint && pnpm test` clean.
