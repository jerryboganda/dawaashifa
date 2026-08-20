# REVIEW_BRIEF.md — Spec 02 (WhatsApp Channel Abstraction & Meta Cloud API)

## Spec Reference
- **Spec**: `docs/02_CHANNEL_AND_CLOUD_API.md`
- **Branch**: `feat/02-channel-cloud-api`

## Invariants Enforced
- **I-10 (Transport Agnosticism)**: Business logic interacts strictly with the `ChannelAdapter` trait and never branches on transport (`Transport::CloudApi` / `Transport::Unofficial`).
- **24-Hour Service Window Guard**: Free-form outbound messages sent outside the active 24-hour service window return `Err(ChannelError::WindowClosed)` and fail loudly rather than silently falling back to a paid template.
- **Template Status Gating**: Templates must be in `APPROVED` status before any outbound transmission. Unapproved templates fail before making network calls.
- **Webhook Security**: `X-Hub-Signature-256` HMAC-SHA256 signature verification rejects tampering with `403 Forbidden` without logging the payload body.
- **Unknown Inbound Safety**: Unknown or non-standard message types are preserved as `InboundContent::Unsupported` rather than erroring or dropping messages.
- **Idempotency & Rate Limiting**: `idempotency_key` deduplication prevents duplicate transmissions, and rate limiter enforces `Capabilities::max_send_rate_per_min`.

## What Was Built
1. **Channel Adapter Trait & Capabilities**: Normalized `OutboundMessage`, `InboundMessage`, `Capabilities`, and async `ChannelAdapter`.
2. **Cloud API Adapter**:
   - 3-tier rich Choice rendering (Buttons <=3, Interactive List 4-10, Numbered Text >10).
   - Confirm intent rendered as interactive reply buttons.
   - Media size limits (Images 5MB, Audio 16MB, Documents 100MB).
3. **Webhook Receiver**:
   - `POST /webhooks/whatsapp/:channel_id` (HMAC verification, fast 200 OK ack).
   - `GET /webhooks/whatsapp/:channel_id` (Meta challenge verification).
4. **Template Registry & Status Management**: Seeded default utility templates (`order_confirmed`, `order_dispatched`, `order_delivered`, `rx_ready_for_review`, `payment_reminder`).
5. **OpenAPI Generation & TypeScript Client**:
   - Webhook routes registered in `crates/api`.
   - `contracts/openapi.json` regenerated.
   - `@shifa/shared` TypeScript SDK regenerated via `pnpm gen:api`.

## Acceptance Tests Verification
- `cargo test --workspace` passed 25 tests with 0 failures:
  - `test_rate_limiter_and_idempotency_prevention` -> ok
  - `test_choice_rendering_three_tiers` -> ok
  - `test_unknown_message_type_is_stored_as_unsupported` -> ok
  - `test_webhook_signature_verification` -> ok
  - `test_freeform_outside_window_fails_loudly` -> ok
  - `test_unapproved_template_fails_before_network_call` -> ok
  - `test_cloud_api_send_success_and_error_handling` -> ok (wiremock mock)
  - `test_api_auth_and_session_lifecycle` -> ok
  - `test_database_migrations_and_rls_suite` -> ok
- `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings.
- `cargo fmt --all --check` clean.
- `pnpm check && pnpm lint && pnpm test` clean.
