# REVIEW BRIEF — Spec 09: Prescription Workflow & Pharmacist Approval Gate

**Branch:** `feat/09-prescription-workflow`  
**Target:** `main`  
**Author:** Builder Agent (Antigravity)  
**Reviewer:** Reviewer Agent (GitHub Copilot / Grok 4.6)  
**Spec Document:** `docs/09_PRESCRIPTION_WORKFLOW.md`  

---

## 1. Executive Summary

Implemented end-to-end prescription intake, VLM-based OCR extraction, drug catalog matching, and the non-negotiable **Licensed Pharmacist Approval Gate (Invariant I-3 & I-6)**. 

No prescription order can ever advance to cart or order creation without an explicit, line-by-line review recorded in `pharmacist_approvals` by an authenticated pharmacist with `rx.approve` permission. Bulk approval routes do not exist.

---

## 2. Invariants Enforced

| Invariant | Implementation Detail | Verification |
|---|---|---|
| **I-1** (`tenant_id` on all tables) | Added `tenant_id UUID NOT NULL` to `prescriptions`, `rx_lines`, `rx_ocr_results`, `pharmacist_approvals`, `controlled_dispensing_register`, and `rx_substitutions`. | Passed (`migration_tests`) |
| **I-2** (Row-Level Security) | RLS enabled with `FORCE ROW LEVEL SECURITY` and `tenant_isolation_policy` on all 6 tables. | Passed (`migration_tests`, `rls_with_tenant`) |
| **I-3** (Pharmacist Approval Gate) | `PrescriptionService::approve` requires `ctx.require("rx.approve")` and iterates over every line in `rx.lines`. If any extracted line number is missing from the submitted decisions list, `RxError::IncompleteReview(line_no)` is returned. | Passed (`test_prescription_approval_gate_and_invariants`) |
| **I-6** (AI Output Gating) | AI VLM extraction reaches `PENDING_REVIEW` queue. Raw doctor handwriting OCR text is stored immutably in `rx_lines.ocr_text` and never overwritten by human corrections. | Passed (`test_vlm_never_guesses_illegible_drug`) |
| **Controlled Substances** | Any approved narcotic drug (`is_narcotic == true`) writes an immutable record to `controlled_dispensing_register` capturing doctor PMDC number, patient name, pharmacist ID, and quantity. | Passed (`test_prescription_approval_gate_and_invariants`) |
| **Drug Substitutions** | Substituted drugs generate `rx_substitutions` row with `customer_informed = false` requiring explicit customer notice before order dispatch. | Passed (`test_prescription_approval_gate_and_invariants`) |
| **Dynamic Alias Learning** | Pharmacist edits/substitutions automatically feed the catalog alias engine via `catalog_service.learn_alias` to improve future OCR matching. | Passed (`test_prescription_approval_gate_and_invariants`) |

---

## 3. Database Schema Extensions

- **Migration**: `migrations/20260821000001_prescription_extensions.sql`
  - Added prescription status enum values (`PARTIALLY_APPROVED`, `NEEDS_CLARIFICATION`, `PREPROCESSING`, `EXTRACTING`, `PENDING_REVIEW`, `UNDER_REVIEW`).
  - Added `pharmacist_action` enum (`ACCEPTED`, `EDITED`, `REJECTED`, `ADDED_MANUALLY`).
  - Created `controlled_dispensing_register` table with indexes and RLS.
  - Created `rx_substitutions` table with indexes and RLS.
  - Added indexes on foreign keys (`prescription_id`, `product_id`, `pharmacist_id`).

---

## 4. API Endpoints (`crates/api/src/routes/prescriptions.rs`)

- `POST /api/v1/prescriptions` — Intake prescription image from WhatsApp/Web.
- `GET /api/v1/prescriptions` — List prescriptions with status filter and pagination.
- `GET /api/v1/prescriptions/{id}` — Fetch prescription detail with extracted lines and candidates.
- `POST /api/v1/prescriptions/{id}/extract` — Trigger / re-run VLM extraction.
- `POST /api/v1/prescriptions/{id}/claim` — Pharmacist claims prescription for review.
- `POST /api/v1/prescriptions/{id}/approve` — Pharmacist line-by-line approval / edit / substitute / reject.
- `POST /api/v1/prescriptions/{id}/reject` — Outright prescription rejection.
- `POST /api/v1/prescriptions/{id}/clarify` — Request customer clarification.
- `GET /api/v1/prescriptions/queue/stats` — Ops queue statistics (pending, under review, oldest wait time).
- `GET /api/v1/prescriptions/{id}/audit` — Reconstruct complete immutable audit trail.

---

## 5. Verification Results

- `cargo fmt --all --check`: Clean (0 diffs)
- `cargo clippy --workspace --all-targets -- -D warnings`: Clean (0 warnings)
- `cargo test -p shifa-prescription`: 3/3 passed (30.01s)
- `cargo test -p shifa-db`: 6/6 passed (RLS + migration suite)
- `pnpm -r check`: Clean (4/4 projects ok)
- `pnpm -r lint`: Clean (4/4 projects ok)
- `pnpm -r test`: Clean (4/4 projects ok)
- `contracts/openapi.json`: Regenerated and committed.
- `apps/shared/src/api/schema.d.ts`: Regenerated via `pnpm gen:api`.
