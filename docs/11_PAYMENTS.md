# DOC 11 — PAYMENTS: GATEWAYS, SCREENSHOTS & COD

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 04, 08, 10
**Produces:** `crates/payments`
**Branch:** `feat/11-payments`

---

## 1. Objective

Two payment paths with very different trust levels: gateway webhooks (trusted, server-verified) and customer screenshots (untrusted, always human-approved). Plus COD, the primary method.

## 2. In scope

- Gateway integrations: JazzCash, EasyPaisa, Raast, and one aggregator (Safepay or PayFast)
- Signed webhook verification and auto-confirmation
- Payment link generation and delivery over WhatsApp
- Screenshot intake, OCR, fraud flagging, human review queue
- Transaction ID ledger preventing reuse
- COD collection and refunds
- Payment reconciliation reporting

## 3. Out of scope — do NOT build

- Any auto-approval of a screenshot (invariant I-4)
- Rider cash reconciliation (Doc 12)
- FBR invoice submission (Doc 13)
- Card processing or recurring billing

## 4. Two paths, one ledger

```rust
pub enum PaymentMethod { Cod, JazzCash, EasyPaisa, Raast, BankTransfer, Aggregator }
pub enum PaymentStatus {
    Pending, AwaitingProof, UnderReview, Confirmed, Rejected, Refunded, Failed
}
```

### 4.1 Path A — gateway webhook (trusted)

1. Create payment intent, generate a payment link
2. Send the link over WhatsApp
3. Customer pays in their banking or wallet app
4. Gateway posts a **signed server-to-server callback**
5. Verify HMAC → verify amount matches order total exactly → verify order is in `AwaitingPayment` → auto-confirm

**Never treat a client-side redirect as proof of payment.** The browser return URL updates the UI only. Only the signed server callback moves money state. A redirect handler that confirms a payment is a critical security defect.

Webhook requirements: idempotent on gateway reference, replay-protected with a timestamp window, raw payload stored in `payments.gateway_payload`, unrecognised references logged and queued for review rather than rejected silently.

### 4.2 Path B — screenshot (untrusted)

A screenshot is a JPEG. Anyone can edit the amount in thirty seconds.

```sql
payment_proofs(id, tenant_id, order_id, payment_id, image_object_key,
               ocr_tid, ocr_amount NUMERIC(14,4), ocr_timestamp,
               ocr_sender, ocr_bank, ocr_confidence,
               duplicate_of_proof_id, fraud_flags JSONB,
               review_status, reviewed_by, reviewed_at, review_note)

transaction_id_ledger(tenant_id, gateway, tid, first_seen_order_id,
                      first_seen_at, PRIMARY KEY (tenant_id, gateway, tid))
```

Flow: intake → OCR via Doc 08 vision → extract TID, amount, timestamp, sender, bank → run fraud checks → **queue for human review**. Always.

## 5. Fraud flags — surfaced, never auto-rejecting

| Flag | Severity | Rule |
|---|---|---|
| `DUPLICATE_TID` | **Critical** | TID already in `transaction_id_ledger` |
| `AMOUNT_MISMATCH` | High | OCR amount ≠ order total |
| `TIMESTAMP_BEFORE_ORDER` | High | Payment predates order creation |
| `TIMESTAMP_STALE` | Medium | Older than 48 hours |
| `EDITED_IMAGE` | High | EXIF shows editing software, or no EXIF where expected |
| `SENDER_REUSED_ACROSS_CUSTOMERS` | High | Same sender account on unrelated customer numbers |
| `LOW_OCR_CONFIDENCE` | Medium | Below 0.70 |
| `UNKNOWN_BANK_LAYOUT` | Low | Screenshot template not recognised |

Flags inform the reviewer. **The system never auto-rejects and never auto-approves.** A human decides every time. Invariant I-4.

The reviewer sees the screenshot, the flags with explanations, and the order side by side. Approve and reject are one click each. This screen is used hundreds of times a day — optimise it.

On approval, the TID is written to `transaction_id_ledger`. `DUPLICATE_TID` on a later proof is then automatic.

## 6. COD

Primary method. On `Confirmed` with `Cod`, create a payment with status `Pending` and `amount = order.total`. It moves to `Confirmed` when the rider reconciles cash (Doc 12).

COD refusal at the door → `FailedDelivery`, payment `Failed`, stock returns via Doc 06 with pharmacist certification per Doc 10.

Per-customer COD limits: configurable ceiling on total outstanding COD value. New customers get a lower ceiling. Repeated refusals set `customers.is_blocked` for COD specifically, not globally.

## 7. Gateway abstraction

```rust
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    fn method(&self) -> PaymentMethod;
    async fn create_intent(&self, req: IntentRequest) -> Result<PaymentIntent, PaymentError>;
    fn verify_webhook(&self, headers: &HeaderMap, body: &[u8]) -> Result<WebhookEvent, PaymentError>;
    async fn refund(&self, payment_id: PaymentId, amount: Money) -> Result<RefundResult, PaymentError>;
    async fn status(&self, gateway_ref: &str) -> Result<PaymentStatus, PaymentError>;
}
```

One implementation per gateway. Adding a gateway must require **zero** changes to `crates/orders`.

Each gateway's credentials come from env, never from the database, never committed.

## 8. Reconciliation

Daily job: fetch settlement reports where the gateway supports it, match against `payments`, flag discrepancies.

Report: expected vs settled vs fees, per gateway, per day. Unmatched payments in either direction are surfaced for manual resolution — never auto-adjusted.

## 9. Endpoints

```
POST   /api/v1/payments/intent              {order_id, method} → payment link
POST   /api/v1/payments/webhooks/:gateway   signed callback, public, no auth
POST   /api/v1/payments/proofs              screenshot upload from a message
GET    /api/v1/payments/proofs/queue        ?branch&severity  [payment.view]
GET    /api/v1/payments/proofs/:id          image, flags, order side by side
POST   /api/v1/payments/proofs/:id/approve  [payment.approve]
POST   /api/v1/payments/proofs/:id/reject   {reason}  [payment.reject]
POST   /api/v1/payments/:id/refund          [payment.refund]
GET    /api/v1/payments                     ?order&method&status&from&to
GET    /api/v1/payments/reconciliation      ?date&gateway  [report.view]
```

## 10. Acceptance tests

- `webhook_rejects_invalid_signature`
- `webhook_rejects_replayed_timestamp`
- `webhook_is_idempotent_on_gateway_ref`
- `redirect_url_alone_never_confirms_payment` — critical
- `amount_mismatch_on_webhook_does_not_confirm`
- `no_screenshot_auto_approval_path_exists` — route and code sweep
- `duplicate_tid_flagged_critical`
- `approved_proof_writes_tid_to_ledger`
- `second_proof_with_same_tid_flags_duplicate`
- `amount_mismatch_flagged_not_rejected`
- `timestamp_before_order_flagged`
- `edited_image_exif_flagged`
- `sender_reused_across_customers_flagged`
- `flags_never_cause_automatic_decision` — assert both approve and reject remain available on every flag combination
- `cod_limit_blocks_order_above_ceiling`
- `cod_refusal_marks_failed_and_triggers_return`
- `refund_requires_permission`
- `adding_a_gateway_requires_no_orders_crate_change` — architectural assertion
- `reconciliation_flags_unmatched_both_directions`

## 11. Done checklist

- [ ] Four gateways behind one trait, credentials from env only
- [ ] Webhook signature, replay protection, idempotency, raw payload retained
- [ ] No path where a client redirect confirms payment
- [ ] Screenshot flow always terminating in human review
- [ ] All eight fraud flags implemented with severities
- [ ] TID ledger with unique constraint and duplicate detection
- [ ] COD with per-customer limits and refusal handling
- [ ] Reconciliation report surfacing unmatched items
- [ ] All 19 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
