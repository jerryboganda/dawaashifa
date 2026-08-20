# REVIEW_BRIEF.md — Spec 08 (AI Orchestration & Language Pipeline)

## Specification & Scope
- **Spec**: `docs/08_AI_ORCHESTRATION.md`
- **Branch**: `feat/08-ai-orchestration`
- **Scope**: AI gateway with versioned prompt templates, deterministic Roman Urdu normalizer, script detection, multi-signal confidence gating matrix, voice note transcription escalation, circuit breaker resiliency, invocation cost logging (`ai_invocations`), and feedback loop with dynamic alias learning (`ai_feedback` -> `product_aliases`).

## Invariants Enforced
- **I-6**: AI output never reaches a customer unmodified in Rx flows. Pharmacist approval required at any confidence score (even 0.99).
- **I-9**: Every AI invocation logs full metadata: tenant_id, conversation_id, message_id, prompt_version, token counts, and execution latency.
- **I-1**: `tenant_id` enforced on `ai_invocations` and `ai_feedback` tables with Postgres RLS.

## Key Changes
1. **Prompt Versioning**: Versioned templates in `crates/ai/prompts/` (`intent_classify.v3.md`, `reply_generate.v2.md`, `entity_extract.v2.md`, `rx_extract.v4.md`).
2. **Language Pipeline (`shifa-ai::language`)**:
   - `detect_script`: Unicode character block detection for Urdu (`\u0600`-\u06FF`, `\u0750`-\u077F`, `\uFB50`-\uFDFF`, `\uFE70`-\uFEFF`), Latin/Roman Urdu, and Code-Mixed.
   - `normalise_roman_urdu`: Rule-based normalizer converting Eastern Arabic numerals (`٠-٩`, `۰-۹`) to standard digits, mapping 60+ dialect variants (e.g. `mujhe`/`mujay`/`mjhe` -> `muje`, `chahiye`/`chaiye` -> `caye`, `kitne`/`kitnay` -> `kitne`), letter transforms (`kh`->`k`, `ph`->`f`, `gh`->`g`, `th`->`t`, `ee`->`i`, `oo`->`u`), and collapsing doubled consonants without affecting numbers. Runs strictly *before* any model invocation.
3. **Confidence Gating Matrix (`shifa-ai::gating`)**:
   - Immediate human escalation for `HumanRequest` and `Complaint` regardless of confidence score.
   - Automatic pharmacist queue for all prescription (`is_rx_context`) and controlled substances.
   - Low confidence (< 0.60) human escalation with courteous customer acknowledgment draft.
   - Auto-send strictly disabled by default; when enabled, forbidden on pricing enquiries and Rx.
4. **Resilience & Circuit Breaker (`shifa-ai::provider`)**:
   - Atomic circuit breaker trips after 5 consecutive provider failures with a 30-second half-open cooldown.
   - Provider failures automatically escalate to human queue without message loss.
5. **Voice Note Audio Pipeline**:
   - Transcription with script detection and normalisation.
   - Forced human escalation for audio > 180s (3 minutes) or transcription confidence < 0.70 with original audio attached.
6. **Continuous Feedback Loop & Active Learning**:
   - Human overrides write `ai_feedback` with Levenshtein-based edit distance.
   - Corrected drug brand/generic names dynamically insert high-confidence alias records into `product_aliases` via `CatalogService::learn_alias`.
7. **REST Endpoints (`crates/api/src/routes/ai.rs`)**:
   - `POST /api/v1/ai/analyse`
   - `POST /api/v1/ai/draft-reply`
   - `POST /api/v1/ai/transcribe`
   - `POST /api/v1/ai/feedback`
   - `GET /api/v1/ai/health`
8. **API Contracts**: OpenAPI specification updated and TypeScript client regenerated in `@shifa/shared`.

## Acceptance Verification Results
- 4 comprehensive integration test suites in `crates/ai/tests/ai_tests.rs`:
  - `test_script_detection_table`: PASSED
  - `test_roman_urdu_normaliser_table`: PASSED
  - `test_confidence_gating_rules`: PASSED
  - `test_ai_pipeline_voice_notes_and_feedback_integration`: PASSED
- `cargo fmt --all --check`: CLEAN
- `cargo clippy --workspace --all-targets -- -D warnings`: CLEAN (0 warnings)
- `pnpm check`, `pnpm lint`, `pnpm test`: CLEAN
