# DOC 13 — FBR POS INTEGRATION, INVOICING & TAX

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 04, 05, 10
**Produces:** `crates/tax`
**Branch:** `feat/13-fbr-tax`

---

## 1. Objective

Fiscal invoicing with real-time FBR POS reporting, correct per-category tax treatment, and PDF invoices delivered over WhatsApp.

**Before implementing, confirm with the business owner:** FBR tier, whether branches are already POS-integrated, and the sales-tax treatment applying to each product category. This spec builds the machinery; the rates are configuration, not code.

## 2. In scope

- Tax categories with per-category rates
- Tax calculation on order confirmation
- Fiscal invoice generation and numbering
- FBR POS real-time submission with a durable retry queue
- FBR QR code payload
- Invoice PDF generation and WhatsApp delivery
- Credit notes for returns
- Tax reporting

## 3. Out of scope — do NOT build

- Income tax, withholding, or payroll
- Full accounting integration
- Provincial services tax (this business sells goods)
- Hardcoding any tax rate in Rust

## 4. Non-negotiable design rule

**An FBR outage must never block a sale.**

Submission is asynchronous. The order confirms, the invoice is generated with a local number, and FBR submission is queued. If FBR is unreachable, the queue retries. The customer is served either way.

Any design where a failed FBR call blocks order confirmation is wrong and must be rejected in review.

## 5. Tax model

```sql
tax_categories(id, tenant_id, name, rate NUMERIC(5,2), fbr_code,
               is_exempt, is_zero_rated, effective_from, effective_to)
```

Rates are **data, never constants in code.** Different treatment applies to medicines, cosmetics, medical devices, and general goods — model each as its own category linked from `product_categories.tax_category`.

```rust
pub struct TaxLine { pub taxable_amount: Money, pub rate: Decimal,
                     pub tax_amount: Money, pub category_id: Uuid, pub fbr_code: String }

pub fn calculate_tax(items: &[OrderItem], cats: &TaxCategoryMap, at: DateTime<Utc>)
    -> Result<TaxResult, TaxError>;
```

- Rate selected by the category's `effective_from`/`effective_to` window at the order's confirmation time. **Historical orders keep their original rate** when rates change.
- Rounding: half-up to 2 decimals, applied **per line**, then summed. Never round the total only — FBR reconciles line by line.
- Exempt and zero-rated are distinct states and must be reported differently.

## 6. Invoice numbering

Two numbers per invoice:
- `invoice_no` — local, `{BRANCH_CODE}/{FY}/{SEQ6}`, generated immediately, never gapped
- `fiscal_invoice_no` — returned by FBR on successful submission, nullable until then

Local numbering comes from a Postgres sequence per branch per fiscal year. Gapless is a regulatory expectation: a cancelled invoice becomes a credit note, it does not vacate its number.

## 7. FBR submission

```rust
#[async_trait]
pub trait FiscalReporter: Send + Sync {
    async fn submit(&self, inv: &Invoice) -> Result<FiscalResponse, FbrError>;
    async fn status(&self, ref_: &str) -> Result<FiscalStatus, FbrError>;
    async fn void(&self, ref_: &str, reason: &str) -> Result<(), FbrError>;
}
```

State: `PENDING → SUBMITTING → ACCEPTED | REJECTED | FAILED`

- Queue on NATS JetStream, durable, survives restarts
- Retry with backoff: 30s, 2m, 10m, 1h, 6h, then hourly to a 72-hour cap
- `REJECTED` (a validation error) does **not** retry — it alerts an operator, because retrying invalid data forever is pointless
- `FAILED` (network or 5xx) retries
- Store the full request and response payloads on the invoice. This is your audit position if FBR ever queries a transaction.
- Alert when the queue depth exceeds a threshold or an invoice has been pending over 6 hours

## 8. QR code

Encode the FBR-specified payload — invoice number, POS ID, timestamp, total, tax total — as a QR on both the PDF and the printed receipt. Generate after `ACCEPTED`. Before acceptance, print the invoice with a "fiscal number pending" marker rather than a fake QR.

## 9. Invoice PDF

Bilingual, English and Urdu. Contains: branch name, address, STRN, DRAP licence number, invoice and fiscal numbers, date and time, customer name and phone, line items with quantity, unit price, MRP, discount, tax rate and tax amount, totals, payment method, FBR QR, and the pharmacist's name where the order included prescription items.

Generated on `ACCEPTED`, stored in MinIO, sent to the customer as a WhatsApp document message. If FBR is still pending after 30 minutes, send a provisional invoice and follow with the fiscal one on acceptance — do not leave the customer without a receipt.

## 10. Credit notes

Returns and cancellations after invoicing create a credit note, not an edit. Credit notes carry their own number series, reference the original invoice, and are submitted to FBR as a void or credit per the API's requirement.

**Invoices are immutable once issued.** There is no edit endpoint. Do not add one.

## 11. Endpoints

```
GET    /api/v1/invoices                    ?branch&status&fbr_status&from&to
GET    /api/v1/invoices/:id
GET    /api/v1/invoices/:id/pdf
POST   /api/v1/invoices/:id/resubmit       [report.view] manual retry
POST   /api/v1/invoices/:id/credit-note    [order.refund]
GET    /api/v1/tax/categories              [product.view]
POST   /api/v1/tax/categories              [tenant.settings]
PATCH  /api/v1/tax/categories/:id          [tenant.settings] creates a new rate period
GET    /api/v1/tax/report                  ?from&to&branch  [report.view]
GET    /api/v1/fbr/queue-status            [report.view]
```

## 12. Acceptance tests

- `fbr_outage_does_not_block_order_confirmation` — the critical test
- `invoice_generated_with_local_number_before_fbr_response`
- `local_invoice_numbering_gapless_under_concurrency`
- `cancelled_invoice_becomes_credit_note_not_gap`
- `tax_rate_selected_by_effective_date`
- `historical_order_keeps_original_rate_after_rate_change`
- `rounding_applied_per_line_not_on_total`
- `exempt_and_zero_rated_reported_distinctly`
- `no_tax_rate_hardcoded_in_source` — source sweep
- `rejected_submission_does_not_retry`
- `failed_submission_retries_with_backoff`
- `queue_survives_service_restart`
- `qr_generated_only_after_acceptance`
- `provisional_invoice_sent_after_30_minutes_pending`
- `invoice_has_no_edit_endpoint` — route table assertion
- `credit_note_references_original_invoice`
- `fbr_request_and_response_persisted`

## 13. Done checklist

- [ ] Tax categories as data with effective-date windows
- [ ] Per-line rounding, half-up, historical rates preserved
- [ ] Gapless local numbering; fiscal number nullable until accepted
- [ ] Async submission on a durable queue; sale never blocked
- [ ] Rejected vs failed handled differently
- [ ] Full request/response persisted per invoice
- [ ] QR only after acceptance; provisional invoice fallback
- [ ] Bilingual PDF delivered over WhatsApp
- [ ] Immutable invoices; credit notes for reversals
- [ ] All 17 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
