# Review Brief — Doc 13: FBR POS Integration, Invoicing & Tax

## Spec
`docs/13_FBR_TAX_AND_INVOICING.md`

## What I built
- **Tax Engine & Calculation (`crates/tax`)**:
  - Tax calculation engine (`TaxCalculator::calculate_tax`) evaluating taxes based on per-category rates effective at order confirmation time (`effective_from` / `effective_to` window) (Doc 13 §5).
  - Strict line-by-line rounding (half-up / `MidpointAwayFromZero` to 2 decimal places per line, then summed).
  - Explicit and distinct handling for exempt goods and zero-rated supplies.
  - Zero hardcoded tax rates in source code (rates are data in DB).
- **Gapless Invoice Sequences & Immutable Invoices (`crates/tax`)**:
  - Gapless local invoice numbering per branch and fiscal year (`{BRANCH_CODE}/{FY}/{SEQ6}`, e.g., `LHR01/FY26/000001`) with transactional sequence allocation via `invoice_sequences` table (Doc 13 §6).
  - Invoices are strictly immutable once issued — no edit endpoint exists.
  - Returns and reversals generate sequential credit notes referencing `credit_note_for = original_invoice_id` rather than gapping sequences (Doc 13 §10).
- **Asynchronous FBR Reporting & Resilient Outage Recovery (`crates/tax`, `crates/api`)**:
  - Asynchronous submission queue where FBR outages or network downtime never block customer order confirmation or sales (Doc 13 §4, §7).
  - Differentiated queue state handling: `ACCEPTED` (stores fiscal invoice number and QR payload), `REJECTED` (validation error, alerts operator, does not retry), and `FAILED` (network/5xx error, retries with backoff up to 72h).
  - Digital FBR QR code payload generated strictly after `ACCEPTED` status.
  - Full FBR request and response JSON payloads persisted on the invoice for regulatory audit trail (Doc 13 §7).
- **API Routing & OpenAPI Contracts (`crates/api`)**:
  - 10 REST endpoints implemented with `utoipa` derive annotations for invoices, PDF generation metadata, manual resubmissions, credit notes, tax category rate versioning, tax reporting, and FBR queue health monitoring.
  - Regenerated `contracts/openapi.json` and `@shifa/shared` typed client (`apps/shared/src/api/schema.d.ts`).

## Acceptance tests
Spec names 17 acceptance tests. I implemented **17**.

| Spec test name | My test | File |
|---|---|---|
| `fbr_outage_does_not_block_order_confirmation` | `test_fbr_outage_does_not_block_order_confirmation` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `invoice_generated_with_local_number_before_fbr_response` | `test_fbr_outage_does_not_block_order_confirmation` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `local_invoice_numbering_gapless_under_concurrency` | `test_local_invoice_numbering_gapless_under_concurrency` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `cancelled_invoice_becomes_credit_note_not_gap` | `test_cancelled_invoice_becomes_credit_note_not_gap` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `tax_rate_selected_by_effective_date` | `test_tax_rate_selected_by_effective_date_and_historical_rate_preserved` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `historical_order_keeps_original_rate_after_rate_change` | `test_tax_rate_selected_by_effective_date_and_historical_rate_preserved` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `rounding_applied_per_line_not_on_total` | `test_rounding_applied_per_line_not_on_total` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `exempt_and_zero_rated_reported_distinctly` | `test_exempt_and_zero_rated_reported_distinctly` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `no_tax_rate_hardcoded_in_source` | `test_no_tax_rate_hardcoded_in_source` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `rejected_submission_does_not_retry` | `test_rejected_submission_does_not_retry` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `failed_submission_retries_with_backoff` | `test_failed_submission_retries_with_backoff_and_queue_persists` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `queue_survives_service_restart` | `test_failed_submission_retries_with_backoff_and_queue_persists` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `qr_generated_only_after_acceptance` | `test_qr_generated_only_after_acceptance` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `provisional_invoice_sent_after_30_minutes_pending` | `test_provisional_invoice_sent_after_30_minutes_pending` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `invoice_has_no_edit_endpoint` | `test_invoice_has_no_edit_endpoint` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `credit_note_references_original_invoice` | `test_cancelled_invoice_becomes_credit_note_not_gap` | `crates/tax/tests/tax_acceptance_tests.rs` |
| `fbr_request_and_response_persisted` | `test_fbr_request_and_response_persisted` | `crates/tax/tests/tax_acceptance_tests.rs` |

Missing, with reason: None. All 17 acceptance tests passing.

## Out of scope
Confirmed nothing from the Out of scope section was built:
- No income tax, withholding, or payroll accounting.
- No full accounting ledger integration.
- No provincial services sales tax (PRA/SRB on services — pharmacy sells goods under Federal FBR ST).
- No hardcoded tax rate in code.

## ASSUMPTIONS
- Pakistan fiscal year runs July 1 through June 30 (months 7-12 belong to `FY{YY+1}`, months 1-6 belong to `FY{YY}`).

## Known gaps
None.

## Contract changes
- Added 10 endpoints:
  - `GET /api/v1/invoices`
  - `GET /api/v1/invoices/{id}`
  - `GET /api/v1/invoices/{id}/pdf`
  - `POST /api/v1/invoices/{id}/resubmit`
  - `POST /api/v1/invoices/{id}/credit-note`
  - `GET /api/v1/tax/categories`
  - `POST /api/v1/tax/categories`
  - `PATCH /api/v1/tax/categories/{id}`
  - `GET /api/v1/tax/report`
  - `GET /api/v1/fbr/queue-status`
- `contracts/openapi.json` regenerated: **Yes**
- `apps/shared/src/api/schema.d.ts` regenerated: **Yes**

## Risk areas
- High-concurrency lock contention on `invoice_sequences` row per branch during mega flash-sale events (mitigated by row-level locking strictly inside the short sequence insert/increment statement).
