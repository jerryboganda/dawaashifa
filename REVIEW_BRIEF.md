# REVIEW_BRIEF.md — Spec 05 (Catalog, DRAP MRP Enforcement, and 4-Signal Product Matching)

## Spec Reference
- **Spec**: `docs/05_CATALOG_AND_MATCHING.md`
- **Branch**: `feat/05-catalog-matching`

## Invariants Enforced
- **DRAP MRP Hard Enforcement (Invariant & Regulation)**: `validate_sale_price` hard blocks any sale price above the printed MRP. Discounts below MRP are allowed; selling above MRP is impossible.
- **Substitution Safety**: Substitution candidates are restricted to verified database matches (`generic_equivalents` and same-generic-same-strength). Every candidate unconditionally carries `requires_pharmacist_approval: true`.
- **Dynamic Alias Learning**: `learn_alias` allows pharmacist OCR/Rx corrections to seed aliases dynamically for instant exact match on next occurrence, while guarding against short inputs (<3 chars), pure numerics, and high-weight conflicts.
- **Urdu-Tuned Phonetics**: Custom normalization and phonetic engine supporting Pakistan-specific substitution classes (`kh ↔ x ↔ k`, `ph ↔ f`, `gh ↔ g`, `ee ↔ i ↔ y`, `oo ↔ u ↔ w`, `aa ↔ a`, `th ↔ t`, `dh ↔ d`, `ch ↔ c`, `z ↔ j`, collapsing silent trailing h and doubled consonants). Tested across 40+ real-world pharmaceutical brand misspelling variants.
- **I-1 / I-2 (Tenant Isolation & RLS)**: Catalog queries and alias lookups are tenant-scoped.
- **I-8 (Money Invariant)**: All prices, MRPs, and savings use exact `Decimal` precision wrapped in `Money`.

## What Was Built
1. **Catalog Domain & Service**:
   - Product CRUD with automatic base alias seeding.
   - Bulk CSV import/export.
2. **Four-Signal Matching Engine**:
   - Signal 1: Exact alias lookup (score 1.0).
   - Signal 2: Trigram string similarity (weight 0.40).
   - Signal 3: Urdu-tuned phonetic equivalence (weight 0.35).
   - Signal 4: Secondary fuzzy vector fallback (weight 0.25) + log-scaled `hit_count` boost.
3. **Substitutions Engine**:
   - Same-generic-same-strength lookups with automatic savings calculation.
   - Therapeutic equivalence lookups from `generic_equivalents`.
4. **Axum HTTP API & OpenAPI**:
   - `/api/v1/products` (list & create).
   - `/api/v1/products/:id` (details).
   - `/api/v1/products/match` (multi-signal search).
   - `/api/v1/products/:id/substitutes` (substitution candidates).
   - Generated `contracts/openapi.json` and TypeScript client `@shifa/shared`.

## Acceptance Tests Verification
- `cargo test --workspace` passed 28 tests with 0 failures:
  - `test_urdu_phonetics_and_normalization_table` -> ok (40+ variants verified)
  - `test_mrp_hard_block_enforcement` -> ok (DRAP above-MRP hard block verified)
  - `test_catalog_matching_and_substitutions_integration` -> ok (exact match, roman urdu fuzzy, learn_alias, substitutes, bulk import)
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
