# REVIEW_BRIEF.md — Spec 10 (Orders, Routing, State Machine, and COD Revenue Loop)

## Spec Reference
- **Spec**: `docs/10_ORDERS_AND_ROUTING.md`
- **Branch**: `feat/10-orders-routing`

## Invariants Enforced
- **Exhaustive State Machine (§4)**: Complete 21-state lifecycle (`Draft`, `CartConfirmed`, `AwaitingRx`, `RxUnderReview`, `RxApproved`, `RxRejected`, `AwaitingPayment`, `PaymentUnderReview`, `PaymentRejected`, `Confirmed`, `Picking`, `Packed`, `Dispatched`, `OutForDelivery`, `Delivered`, `CashReconciled`, `Closed`, `Cancelled`, `FailedDelivery`, `Returned`, `Refunded`) with strict transition validations via `can_transition(from, to)`.
- **Atomic Transition & Audit (Invariant I-9)**: Every state change updates order status, inserts an `order_events` row, and writes an `audit_log` row within a single database transaction. If the audit log fails, the entire transition rolls back.
- **Rx Branching (Invariants I-3 & I-6)**: Orders containing any item with `is_prescription_only = true` are forced into `AwaitingRx -> RxUnderReview -> RxApproved`. Direct progression from `Draft` / `CartConfirmed` to `AwaitingPayment` is strictly blocked.
- **Collision-Free Sequence-Based Order Numbering (§5)**: Format `{BRANCH_CODE}-{YYMMDD}-{SEQ4}` backed by atomic PostgreSQL sequence generation ensuring zero collisions under high concurrency.
- **Shared Stock Branch Routing Engine (§6)**: Evaluates active candidate branches, cold-chain handling, and stock depth via FEFO. Ranks single branch fulfillment over split fulfillment and supports three policies (`ALLOW_SPLIT`, `PREFER_TRANSFER`, `SINGLE_BRANCH_ONLY`).
- **Stock Reservations & Idempotent Release (§7)**: Transition to `Confirmed` reserves stock with TTL (2h for COD). Rejection or cancellation triggers idempotent reservation release. Failure to reserve blocks transition to `Confirmed`.
- **MRP Enforcement & Snapshotting (§8)**: Unit price cannot exceed DRAP MRP (`validate_item_price`). `mrp_at_sale` is snapshotted onto each line item, guaranteeing that future MRP updates do not alter historical orders.
- **Money Precision (Invariant I-8)**: All financial calculations use exact `rust_decimal::Decimal`. Zero floating-point arithmetic throughout the crate.
- **Controlled Returns Restocking (§9)**: Restocking of medicines requires explicit pharmacist certification. Cold-chain items that left the cold chain are permanently forbidden from being restocked.

## What Was Built
1. **Order Domain & Service (`crates/orders`)**:
   - `state_machine.rs`: 21-state enum, parser, and exhaustive `can_transition` matrix.
   - `numbering.rs`: Concurrency-safe daily branch sequence order numbering.
   - `pricing.rs`: Exact Decimal line and order total pricing with MRP protection.
   - `routing.rs`: Multi-branch stock evaluation with split fulfillment policies.
   - `service.rs`: `OrderService` handling drafts, cart confirmation, atomic transitions, reservations, and returns.
2. **Axum HTTP API & OpenAPI**:
   - `GET /api/v1/orders` (list with filters)
   - `POST /api/v1/orders` (create draft)
   - `GET /api/v1/orders/:id`
   - `POST /api/v1/orders/:id/items`
   - `POST /api/v1/orders/:id/confirm-cart`
   - `POST /api/v1/orders/:id/transition`
   - Regenerated `contracts/openapi.json` and generated TypeScript client `@shifa/shared`.

## Acceptance Tests Verification
- `cargo test --workspace` passed 36 tests with 0 failures:
  - `test_every_illegal_transition_rejected` -> ok
  - `test_money_arithmetic_uses_decimal_and_mrp_validation` -> ok
  - `test_order_lifecycle_routing_and_reservation_suite` -> ok
  - `test_canned_reply_unresolved_variable_blocks_send` -> ok
  - `test_sla_timer_pauses_outside_opening_hours_and_two_stage_escalation` -> ok
  - `test_conversation_lifecycle_routing_and_human_override_suite` -> ok
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
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt --all --check` clean.
- `pnpm check && pnpm lint && pnpm test` clean.
