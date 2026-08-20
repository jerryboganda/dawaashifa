# REVIEW_BRIEF.md — Spec 07 (Conversation Engine, WhatsApp Threading, Routing, and Human Override)

## Spec Reference
- **Spec**: `docs/07_CONVERSATION_ENGINE.md`
- **Branch**: `feat/07-conversation-engine`

## Invariants Enforced
- **Conversation Threading & Reopen**: Inbound messages maintain conversation lifecycle (`NEW -> AWAITING_HUMAN -> ASSIGNED -> RESOLVED -> CLOSED`). Any inbound on a `RESOLVED` or `CLOSED` conversation automatically reopens it as `AWAITING_HUMAN` while preserving history.
- **Race-Safe Customer Auto-Creation**: First inbound auto-creates customer profile using `ON CONFLICT (tenant_id, phone) DO NOTHING` with fallback query retrieval to eliminate concurrent duplicate customer creation bugs.
- **Silent Storage for Blocked Customers**: Blocked customer messages are persisted to DB for compliance, but receive silence (not routed to agents, no notifications dispatched).
- **Four-Step Branch Routing Precedence**:
  1. Explicit branch on number / channel
  2. Customer's last-ordered branch within 60 days
  3. Customer's default / nearest branch
  4. Tenant default active branch
- **Atomic Claiming**: First writer successfully claims an unassigned conversation; subsequent concurrent claims return 409 Conflict with `AlreadyClaimed`.
- **Human Override & Training Signal (Doc 07 §8)**: Agents/pharmacists can override any `PENDING_APPROVAL` draft message. Editing preserves `original_body`, records `overridden_by`, and emits an audit event for AI model fine-tuning.
- **Invariant I-6 (Rx Bulk Approval Protection)**: Bulk approval of pending drafts is strictly rejected for Rx-linked conversations, enforcing individual review per drug order.
- **Strict Canned Reply Validation**: Unresolved variables (e.g. `{{order_no}}`, `{{customer_name}}`) block transmission with `Err(UnresolvedVariables)`.
- **SLA Engine & Opening Hours**: Response timers pause outside business hours and trigger 2-stage escalation (`BRANCH_MANAGER` -> `OPERATIONS_HEAD`).
- **24-Hour WhatsApp Service Window**: Outbound messages sent >24h after customer's last message require pre-approved templates.

## What Was Built
1. **Conversation Domain & Service (`crates/conversation`)**:
   - Customer auto-creation & resolution (`customer.rs`).
   - 4-step branch routing (`routing.rs`).
   - Assignment strategies (Manual, RoundRobin, LeastBusy) & atomic claiming (`assignment.rs`).
   - Human override engine with Rx bulk protection (`override_engine.rs`).
   - Canned replies with strict placeholder verification (`canned.rs`).
   - SLA business hours evaluation & 2-stage escalation (`sla.rs`).
2. **Axum HTTP API & OpenAPI**:
   - `/api/v1/conversations` (list)
   - `/api/v1/conversations/inbound`
   - `/api/v1/conversations/:id/messages`
   - `/api/v1/conversations/:id/claim`
   - `/api/v1/conversations/:id/assign`
   - `/api/v1/conversations/:id/transfer`
   - `/api/v1/messages/:id` (override draft)
   - `/api/v1/messages/bulk-approve/:conversation_id`
   - `/api/v1/canned-replies`
   - Regenerated `contracts/openapi.json` and generated TypeScript client `@shifa/shared`.

## Acceptance Tests Verification
- `cargo test --workspace` passed 33 tests with 0 failures:
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
- `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings.
- `cargo fmt --all --check` clean.
- `pnpm check && pnpm lint && pnpm test` clean.
