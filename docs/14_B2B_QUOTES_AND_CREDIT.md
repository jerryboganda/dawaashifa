# DOC 14 — B2B MODULE: QUOTES, CREDIT & AR AGING

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 04, 05, 06, 10, 11, 13
**Produces:** `crates/b2b`
**Branch:** `feat/14-b2b`

---

## 1. Objective

Sell implants, surgical consumables and devices to hospitals and surgeons. This is a **different business** from retail pharmacy — quotes instead of carts, purchase orders instead of WhatsApp confirmations, credit terms instead of COD, and receivables that must be chased.

## 2. In scope

- Business accounts (hospitals, clinics, surgeon practices)
- Contacts and approval hierarchies per account
- Quotations with versioning, validity and approval
- Purchase order intake and matching
- Credit limits and payment terms
- Accounts receivable with aging buckets
- Consignment stock at hospital sites
- Device traceability (UDI / lot / serial)

## 3. Out of scope — do NOT build

- Public tender or e-procurement portal integration
- Full general ledger
- Commission tracking for sales representatives
- Retail order flows (Doc 10 covers those)

## 4. Why this is separate from retail

| | Retail | B2B |
|---|---|---|
| Trigger | WhatsApp message | RFQ or standing agreement |
| Pricing | MRP-capped, fixed | Negotiated per account, per contract |
| Commitment | Order confirmation | Signed quote plus purchase order |
| Payment | COD or gateway, immediate | Net 30/60/90 credit |
| Delivery | Rider | Scheduled, often to a hospital store |
| Traceability | Batch | **Serial or UDI per implanted device** |

Do not attempt to force B2B through the retail state machine. `orders.order_type = 'B2B'` shares the table; the workflow before `Confirmed` is entirely different.

## 5. Accounts

```sql
business_accounts(id, tenant_id, name, account_type, ntn, strn,
                  billing_address, shipping_addresses JSONB,
                  credit_limit NUMERIC(14,4), payment_terms_days,
                  price_list_id, status, on_hold, hold_reason,
                  created_at, updated_at)
business_contacts(id, tenant_id, account_id, name, designation,
                  phone, email, can_approve_po, approval_limit)
price_lists(id, tenant_id, name, valid_from, valid_to)
price_list_items(price_list_id, product_id, price NUMERIC(14,4), min_qty)
```

Account-specific pricing overrides the standard price. **The MRP cap still applies** — a negotiated price may be below MRP, never above.

## 6. Quotations

```
DRAFT → SENT → UNDER_NEGOTIATION → ACCEPTED | REJECTED | EXPIRED
      ↘ REVISED (new version, previous superseded)
```

```sql
quotations(id, tenant_id, account_id, quote_no, version, parent_quote_id,
           status, valid_until, subtotal, discount, tax_amount, total,
           terms_text, prepared_by, approved_by, sent_at, responded_at)
quotation_items(id, quotation_id, product_id, qty, unit_price, discount,
                line_total, lead_time_days, notes)
```

- Quote number `Q-{BRANCH}-{YY}-{SEQ5}`; revisions increment `version`, never overwrite
- Discount beyond a configurable threshold requires approval by a user with a sufficient `approval_limit`
- Quotes expire automatically at `valid_until`; an expired quote cannot be converted
- Accepted quote converts to an order with `order_type = 'B2B'`, at `Confirmed`, bypassing cart and payment stages

## 7. Purchase orders

```sql
purchase_orders(id, tenant_id, account_id, quotation_id, po_number,
                po_document_key, received_at, verified_by, amount, status)
```

Hospitals send a PO document. Staff upload it, the system matches it against the quote, and flags any variance in amount, quantity or item. **A variance blocks fulfilment until resolved** — shipping against a mismatched PO is how receivables become disputes.

## 8. Credit control

```rust
pub fn credit_check(acct: &BusinessAccount, ar: &ArSummary, new: Money)
    -> Result<(), CreditError> {
    if acct.on_hold { return Err(CreditError::AccountOnHold(acct.hold_reason.clone())); }
    if ar.overdue_over_90 > Money::ZERO { return Err(CreditError::OverdueBalance); }
    if ar.outstanding + new > acct.credit_limit { return Err(CreditError::LimitExceeded { .. }); }
    Ok(())
}
```

Runs before quote acceptance and again before dispatch — an account's position can change between the two. Override requires `b2b.credit` permission, a documented reason, and an audit entry.

## 9. Accounts receivable

Aging buckets: current, 1–30, 31–60, 61–90, 90+ days.

- Invoice due date = invoice date + `payment_terms_days`
- Automatic reminders at 7 days before due, on due date, and at 7/30/60 days overdue
- 90+ days overdue automatically sets `on_hold`
- Partial payments allocate oldest-invoice-first by default, overridable with a reason
- AR aging report by account, by branch, by sales owner

## 10. Consignment stock

Stock physically held at a hospital but still owned by the pharmacy until used.

```sql
consignment_locations(id, tenant_id, account_id, name, address, managed_by)
consignment_stock(id, tenant_id, location_id, product_id, batch_id,
                  serial_no, qty, placed_at, consumed_at, invoiced_at)
```

- Placement creates a `TRANSFER_OUT` movement to a virtual consignment branch (Doc 06), not a sale
- Consumption reported by the hospital triggers invoicing
- Periodic reconciliation counts against the system's expectation; discrepancies are flagged, never auto-adjusted
- Expiry monitoring applies at consignment locations too — expiring consignment stock is recalled

## 11. Device traceability

Implants require unit-level tracking, not batch-level.

```sql
device_units(id, tenant_id, product_id, batch_id, serial_no, udi,
             status, location_type, location_id,
             implanted_at, patient_ref, surgeon_name, order_id)
UNIQUE (tenant_id, serial_no)
```

Every implantable unit is individually tracked from receipt to implantation. On a manufacturer recall you must be able to answer, within minutes, which units were affected and where each one went. Build the recall query as a first-class endpoint, not an ad-hoc report.

## 12. Endpoints

```
GET/POST/PATCH  /api/v1/b2b/accounts                     [b2b.quote]
GET/POST        /api/v1/b2b/accounts/:id/contacts
GET             /api/v1/b2b/accounts/:id/ar-summary
POST            /api/v1/b2b/accounts/:id/hold            [b2b.credit]
GET/POST        /api/v1/b2b/quotations                   ?account&status
POST            /api/v1/b2b/quotations/:id/revise        creates a new version
POST            /api/v1/b2b/quotations/:id/send
POST            /api/v1/b2b/quotations/:id/accept        runs credit check
POST            /api/v1/b2b/quotations/:id/approve       [b2b.credit] discount approval
POST            /api/v1/b2b/purchase-orders              upload and match
GET             /api/v1/b2b/ar/aging                     [report.view]
GET/POST        /api/v1/b2b/consignment/stock
POST            /api/v1/b2b/consignment/:id/consume
POST            /api/v1/b2b/consignment/:id/reconcile
GET             /api/v1/b2b/devices/:serial              full history
GET             /api/v1/b2b/devices/recall               ?product_id&batch_no
```

## 13. Acceptance tests

- `negotiated_price_above_mrp_rejected`
- `quote_revision_creates_new_version_preserving_original`
- `expired_quote_cannot_convert`
- `discount_above_threshold_requires_approval`
- `approver_below_limit_cannot_approve`
- `credit_check_blocks_on_limit_exceeded`
- `credit_check_blocks_on_90_day_overdue`
- `credit_check_runs_again_before_dispatch`
- `credit_override_requires_permission_and_audits`
- `po_variance_blocks_fulfilment`
- `partial_payment_allocates_oldest_first`
- `ninety_day_overdue_sets_account_on_hold`
- `consignment_placement_is_transfer_not_sale`
- `consignment_discrepancy_flagged_not_auto_adjusted`
- `device_serial_unique_per_tenant`
- `recall_query_returns_all_affected_units_with_locations`
- `b2b_order_bypasses_retail_cart_stages`

## 14. Done checklist

- [ ] Business accounts with contacts, approval limits, price lists
- [ ] MRP cap enforced on negotiated pricing
- [ ] Versioned quotations with expiry and discount approval
- [ ] PO upload with variance blocking
- [ ] Credit check at acceptance and dispatch, override audited
- [ ] AR aging with automatic reminders and 90-day hold
- [ ] Consignment as transfer, with reconciliation and expiry monitoring
- [ ] Unit-level device traceability with a first-class recall query
- [ ] All 17 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
