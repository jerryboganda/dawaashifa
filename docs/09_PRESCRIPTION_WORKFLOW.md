# DOC 09 — PRESCRIPTION WORKFLOW & PHARMACIST APPROVAL

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 04, 05, 07, 08
**Produces:** `crates/prescription`
**Branch:** `feat/09-prescription-workflow`

---

## 1. Objective

Prescription intake, AI-assisted extraction, and the pharmacist approval gate. **This is the most legally sensitive module in the platform.** A regulator may one day ask you to prove what a pharmacist saw and what they changed. The audit chain is the deliverable, not a side effect.

## 2. In scope

- Prescription intake from WhatsApp images and PDFs
- Image preprocessing (deskew, contrast, crop)
- VLM extraction into structured lines
- Product matching per line via Doc 05
- Pharmacist review queue and per-line decisions
- Approval gate enforcing invariant I-3
- Alias learning on correction
- Immutable audit chain
- Controlled substance handling

## 3. Out of scope — do NOT build

- Any automatic approval path (there must be none)
- Order creation (Doc 10 consumes an approved prescription)
- The review UI (Doc 16)
- Doctor verification against a PMDC registry (record the number; do not verify)

## 4. Reality check for the implementing agent

90% of prescriptions here are handwritten Pakistani doctor scripts. **No model reads these reliably.** Expected per-line accuracy is 40–70%.

The system's job is to cut pharmacist typing time, not to replace the pharmacist. Design every interface around fast correction, not around trusting extraction. An agent that builds this module assuming high OCR accuracy will build the wrong UI and the wrong thresholds.

## 5. Status flow

```
RECEIVED → PREPROCESSING → EXTRACTING → PENDING_REVIEW
         → UNDER_REVIEW → APPROVED | PARTIALLY_APPROVED | REJECTED
                        ↘ NEEDS_CLARIFICATION → PENDING_REVIEW
```

- `PARTIALLY_APPROVED` — some lines approved, others rejected. Common and must be fully supported.
- `NEEDS_CLARIFICATION` — pharmacist requests a clearer photo or asks the customer a question. Returns to the queue on reply.
- Extraction failure goes to `PENDING_REVIEW` with empty lines, **not** to an error state. The pharmacist types it manually. The workflow must never dead-end on a bad photo.

## 6. Preprocessing

Before the VLM call: EXIF rotation, auto-deskew, adaptive contrast, border crop, upscale if under 1000px on the long edge. Reject and ask for a retake if, after preprocessing, the image is under 300×300, over 20MB, or blur-scored below threshold.

Store the **original** untouched. Store the preprocessed version separately. The original is the legal record.

## 7. Extraction contract

```rust
pub struct RxExtraction {
    pub doctor_name: Option<String>,
    pub doctor_pmdc_no: Option<String>,
    pub issued_date: Option<NaiveDate>,
    pub patient_name: Option<String>,
    pub lines: Vec<RxExtractedLine>,
    pub overall_confidence: f32,
    pub warnings: Vec<String>,
}

pub struct RxExtractedLine {
    pub line_no: i32,
    pub raw_text: String,
    pub drug_text: Option<String>,
    pub strength_text: Option<String>,
    pub form_text: Option<String>,
    pub qty_text: Option<String>,
    pub dosage_text: Option<String>,
    pub confidence: f32,
}
```

The VLM returns strict JSON. **Never guess a drug name.** If a line is illegible, return the raw text with `drug_text: None` and confidence 0. A wrong guess is more dangerous than an admitted gap — the pharmacist may not catch a plausible-looking error.

Each extracted line then goes through `catalog::match_product` to produce ranked candidates.

## 8. The approval gate — invariant I-3

```rust
pub async fn approve(
    ctx: &TenantContext, pool: &PgPool,
    rx_id: PrescriptionId, decisions: Vec<LineDecision>, note: Option<String>,
) -> Result<ApprovalResult, RxError> {
    ctx.require("rx.approve")?;               // PHARMACIST or SUPER_ADMIN only
    // ... every line must carry an explicit decision
    // ... writes pharmacist_approvals with user_id, ip, device, timestamp
    // ... writes audit_log
}
```

Non-negotiable:
- Only `rx.approve` holders may call it
- **Every line requires an explicit decision.** No defaulting to accept. A submission missing a decision for any line is rejected with `IncompleteReview`.
- No bulk-approve-all endpoint exists. Do not add one.
- No time-based, confidence-based, or volume-based auto-approval exists under any configuration.
- The approving user, IP, device and timestamp are recorded immutably.

```rust
pub enum LineAction {
    Accept,                                   // as matched
    Edit { product_id: ProductId, qty: i32, dosage: String },
    Substitute { product_id: ProductId, reason: String },
    Reject { reason: String },
    AddManual { product_id: ProductId, qty: i32, dosage: String },
}
```

## 9. Substitution

Candidates come only from `catalog::substitution_candidates` (Doc 05). The AI may rank them; it may not invent one.

Every substitution records: original product, substituted product, pharmacist reason, and whether the customer was informed. Customer notification of a substitution is **mandatory** before dispatch — the customer must know what they are actually receiving.

## 10. Controlled substances

- Any line matching a product with `is_controlled = true` forces `PENDING_REVIEW` regardless of confidence
- Requires the original physical prescription to be collected on delivery — flag on the order
- Written into a separate `controlled_dispensing_register` with pharmacist, quantity, prescriber, and patient
- Never eligible for any auto-suggest path

## 11. Alias learning

On every `Edit`, `Substitute`, or `AddManual`, call:
```rust
catalog::learn_alias(ctx, pool, &line.raw_text, chosen_product_id,
                     AliasSource::PharmacistCorrection).await?;
```

**This is the compounding asset.** Every correction makes the next prescription from that doctor easier to read. Skipping this turns a system that improves into one that does not.

## 12. Immutability

- `prescriptions.image_object_key` is write-once. No endpoint replaces it.
- `rx_ocr_results` rows are never updated. Re-extraction inserts a new row.
- `rx_lines.ocr_text` is never overwritten. Corrections populate `matched_product_id` and `pharmacist_action`, leaving the original text intact.
- `pharmacist_approvals` rows are append-only. A reversal is a new row, not an edit.

MinIO bucket for prescriptions has object-lock retention enabled.

## 13. Endpoints

```
POST   /api/v1/prescriptions                    intake from a message
GET    /api/v1/prescriptions                    ?status&branch&assigned&page  [rx.view]
GET    /api/v1/prescriptions/:id                full detail with lines and candidates
POST   /api/v1/prescriptions/:id/extract        re-run extraction  [rx.view]
POST   /api/v1/prescriptions/:id/claim          take it off the queue
POST   /api/v1/prescriptions/:id/approve        [rx.approve]
POST   /api/v1/prescriptions/:id/reject         [rx.reject]
POST   /api/v1/prescriptions/:id/clarify        request a better photo
GET    /api/v1/prescriptions/queue/stats        queue depth, oldest waiting
GET    /api/v1/prescriptions/:id/audit          full chain  [audit.view]
```

## 14. Acceptance tests

- `approve_without_rx_approve_permission_returns_403`
- `approve_with_missing_line_decision_returns_incomplete_review`
- `no_bulk_approve_endpoint_exists` — route table assertion
- `no_configuration_enables_auto_approval` — exhaustive settings sweep
- `approval_writes_immutable_record_with_user_ip_device`
- `partial_approval_supported`
- `extraction_failure_reaches_pending_review_not_error`
- `illegible_line_returns_null_drug_not_a_guess`
- `original_image_is_write_once`
- `rx_lines_ocr_text_never_overwritten_by_correction`
- `reversal_creates_new_row_not_update`
- `edit_triggers_learn_alias`
- `substitute_only_from_generic_equivalents`
- `substitution_requires_customer_notification_before_dispatch`
- `controlled_substance_forces_review_at_any_confidence`
- `controlled_dispensing_writes_register`
- `cross_tenant_prescription_returns_404`
- `full_audit_chain_reconstructable` — extraction → every edit → approval

## 15. Done checklist

- [ ] Intake from image and PDF with preprocessing and retake rejection
- [ ] Original image immutable, object-lock retention on the bucket
- [ ] VLM extraction returning strict JSON, never guessing illegible drugs
- [ ] Per-line matching through Doc 05
- [ ] Approval gate: permission-checked, every line explicit, no bulk path
- [ ] No configuration anywhere enables auto-approval
- [ ] Substitution restricted to the equivalents table, customer notified
- [ ] Controlled substance register
- [ ] `learn_alias` called on every correction
- [ ] All 18 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
