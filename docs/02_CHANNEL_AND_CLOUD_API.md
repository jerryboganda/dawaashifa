# DOC 02 — CHANNEL ABSTRACTION & CLOUD API ADAPTER

**Agent:** Backend (Copilot)
**Depends on:** Doc 01
**Produces:** `crates/channel`
**Branch:** `feat/02-channel-cloud-api`

---

## 1. Objective

Build the WhatsApp abstraction and the Meta Cloud API adapter. After this spec, the platform can receive and send WhatsApp messages. This is the primary transport in production.

## 2. In scope

- `ChannelAdapter` trait + `Capabilities`
- `OutboundMessage` / `InboundMessage` normalised types
- `CloudApiAdapter` — Graph API v21+
- Webhook receiver: signature verification, dedup, publish to NATS
- Media download and upload to MinIO
- Template registry and send path
- 24-hour service window tracking
- Outbound rate limiting and retry

## 3. Out of scope — do NOT build

- Baileys / unofficial adapter (Doc 03)
- Number pool failover logic (Doc 03)
- Conversation threading, inbox, assignment (Doc 07)
- Any AI (Doc 08)
- Any business reaction to message content

## 4. Contracts

```rust
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn channel_id(&self) -> ChannelId;
    fn transport(&self) -> Transport;
    fn capabilities(&self) -> Capabilities;
    async fn send(&self, msg: OutboundMessage) -> Result<MessageReceipt, ChannelError>;
    async fn download_media(&self, r: &MediaRef) -> Result<MediaBytes, ChannelError>;
    async fn health(&self) -> ChannelHealth;
}

pub enum Transport { CloudApi, Unofficial }

pub struct Capabilities {
    pub interactive_buttons: bool,
    pub list_messages: bool,
    pub templates: bool,
    pub outside_window: bool,
    pub delivery_receipts: bool,
    pub max_send_rate_per_min: u32,
    pub max_buttons: u8,
}
```

### 4.1 Outbound intent — authored rich, rendered per transport

```rust
pub enum OutboundBody {
    Text { body: String },
    Choice { prompt: String, options: Vec<ChoiceOption>, min: u8, max: u8 },
    Confirm { prompt: String, yes: String, no: String },
    Media { kind: MediaKind, object_key: String, caption: Option<String> },
    Document { object_key: String, filename: String, caption: Option<String> },
    Template { name: String, language: String, params: Vec<TemplateParam> },
}

pub struct OutboundMessage {
    pub tenant_id: TenantId,
    pub conversation_id: ConversationId,
    pub to: Msisdn,
    pub body: OutboundBody,
    pub idempotency_key: Uuid,   // required — dedup on retry
    pub locale: Locale,
}
```

**Callers construct intent. Only the adapter decides rendering.** Invariant I-10.

Cloud API rendering:
| Intent | Rendering |
|---|---|
| `Choice` ≤3 options | interactive reply buttons |
| `Choice` 4–10 options | interactive list message |
| `Choice` >10 | numbered text list, reply with a number |
| `Confirm` | two reply buttons |
| `Template` | template message (only path valid outside the window) |

### 4.2 Inbound normalisation

```rust
pub struct InboundMessage {
    pub tenant_id: TenantId,
    pub channel_id: ChannelId,
    pub from: Msisdn,
    pub transport_message_id: String,
    pub content: InboundContent,   // Text | Image | Audio | Document | Location
                                   // | ButtonReply | ListReply | Unsupported
    pub timestamp: DateTime<Utc>,
    pub raw: serde_json::Value,     // always retained
}
```

## 5. Webhook receiver

`POST /webhooks/whatsapp/:channel_id`

1. Verify `X-Hub-Signature-256` HMAC-SHA256 against the app secret. **Reject on mismatch — do not log the body.**
2. Return `200 OK` within 200ms, before any processing. Meta retries aggressively on slow responses.
3. Dedup on `transport_message_id` via Redis `SETNX`, 24h TTL. Meta redelivers.
4. Publish to NATS `wa.inbound.{tenant_id}`.
5. Persist the raw payload to `messages.raw` regardless of parse success.

`GET /webhooks/whatsapp/:channel_id` handles Meta's `hub.challenge` verification.

**Unknown message types must not error.** Store as `Unsupported`, acknowledge to the customer politely, queue for human.

## 6. Service window tracking

```rust
// Inbound message opens/refreshes a 24h window
conversations.window_expires_at = inbound.timestamp + 24h;

pub fn can_send_freeform(conv: &Conversation) -> bool {
    conv.window_expires_at > Utc::now()
}
```

`send()` returns `Err(WindowClosed)` if a free-form body is sent outside the window. The caller must switch to a `Template`. **This must fail loudly, not silently fall back** — a silent fallback to a paid template is an unbudgeted cost.

Service messages inside the window are free. Templates outside it are billed per message.

## 7. Templates

```sql
message_templates(id, tenant_id, name, language, category, body_text,
                  variable_count, meta_status, meta_template_id,
                  approved_at, created_at)
```

Seed these (utility category):
- `order_confirmed` — order no, total, branch
- `order_dispatched` — order no, rider name, ETA
- `order_delivered` — order no, amount collected
- `rx_ready_for_review` — customer name
- `payment_reminder` — order no, amount

Sending a template not in `APPROVED` state returns `Err(TemplateNotApproved)` before any network call.

## 8. Rate limiting & retry

- Token bucket per channel in Redis, sized from `Capabilities::max_send_rate_per_min`
- Outbound send queue on NATS JetStream, one consumer per channel
- Retry with jittered exponential backoff on `429` and `5xx`: 1s, 4s, 15s, 60s, 300s
- **No retry on `4xx` other than 429** — those are permanent
- After 5 failures mark the message `FAILED` and alert ops
- Idempotency key prevents duplicate sends on retry

## 9. Media

Inbound: download from Meta within the media TTL, store in MinIO at `{tenant_id}/media/{yyyy}/{mm}/{message_id}.{ext}`, record `media_object_key`. Meta's media URLs expire — download immediately, do not store the URL.

Outbound: upload to Meta, cache the returned media ID keyed by object hash so repeat sends skip re-upload.

Enforce limits: images 5MB, documents 100MB, audio 16MB. Reject oversize before upload with a clear customer-facing message.

## 10. Configuration

```
WA_CLOUD_API_VERSION=v21.0
WA_APP_SECRET=
WA_VERIFY_TOKEN=
WA_ACCESS_TOKEN=
WA_PHONE_NUMBER_ID=
WA_BUSINESS_ACCOUNT_ID=
```
Per-channel values live in the `channels` table; env holds the app-level secrets.

## 11. Acceptance tests

- `webhook_rejects_bad_signature` — 403, body not logged
- `webhook_responds_under_200ms` — processing is async
- `webhook_dedups_redelivery` — same message id twice produces one NATS publish
- `unknown_message_type_is_stored_not_dropped`
- `freeform_outside_window_returns_error` — not a silent template fallback
- `choice_three_options_renders_buttons`
- `choice_eight_options_renders_list`
- `choice_fifteen_options_renders_numbered_text`
- `unapproved_template_fails_before_network_call`
- `rate_limiter_respects_capability_ceiling`
- `retry_backoff_on_429_then_succeeds`
- `no_retry_on_400`
- `inbound_media_downloaded_to_minio`
- `idempotency_key_prevents_duplicate_send`

All Meta calls mocked with `wiremock`. No test touches a real number.

## 12. Done checklist

- [ ] `ChannelAdapter` trait defined; `CloudApiAdapter` implements it fully
- [ ] Webhook verifies signature, dedups, responds fast, publishes to NATS
- [ ] All three `Choice` rendering tiers implemented and tested
- [ ] Window tracking enforced; free-form outside window errors
- [ ] Template registry with `meta_status` gating
- [ ] Rate limiter and retry with idempotency
- [ ] Media download/upload to MinIO with size limits
- [ ] All 14 acceptance tests green
- [ ] `contracts/openapi.json` regenerated (webhook routes)
- [ ] Clippy clean, `cargo sqlx prepare` run
