# DOC 05 — CATALOG, DRUG MASTER & PRODUCT MATCHING ENGINE

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 04
**Produces:** `crates/catalog`
**Branch:** `feat/05-catalog-matching`

---

## 1. Objective

The product master and the matching engine that turns "mujhe panadal chahiye" into a specific SKU. **The matching engine is the highest-leverage component in the platform** — everything downstream depends on resolving messy customer text to a real product.

## 2. In scope

- Product CRUD, categories, MRP enforcement
- Generics, generic equivalence, substitution candidates
- `product_aliases` table and alias management
- Four-signal matching engine (exact, trigram, phonetic, vector)
- Urdu-tuned phonetic algorithm
- Alias learning hook (called by Doc 09)
- Bulk import/export
- Search endpoints

## 3. Out of scope — do NOT build

- Stock levels or availability (Doc 06)
- LLM calls (Doc 08 — this module is deterministic)
- Prescription parsing (Doc 09)
- Pricing rules beyond MRP (later)

## 4. MRP enforcement

<cite>DRAP sets maximum retail prices and pharmacies may not charge above the printed MRP.</cite>

```rust
pub fn validate_sale_price(p: &Product, price: Money) -> Result<(), CatalogError> {
    if price > p.mrp { return Err(CatalogError::AboveMrp { mrp: p.mrp, attempted: price }); }
    Ok(())
}
```

Called on every order line creation. **Hard block, not a warning.** Discounts below MRP are always allowed. An override requires `product.price` permission, writes `audit_log`, and is still capped at MRP — there is no path to sell above MRP.

## 5. The alias table — the core asset

```sql
product_aliases(
  id, tenant_id, product_id,
  alias TEXT NOT NULL,              -- normalised lowercase
  alias_type,                       -- BRAND | GENERIC | MISSPELLING | URDU
                                    -- | ROMAN_URDU | ABBREVIATION | LOCAL_NAME
  script,                           -- LATIN | ARABIC
  weight NUMERIC(3,2) DEFAULT 1.0,
  source,                           -- SEED | PHARMACIST_CORRECTION | IMPORT | MANUAL
  hit_count INTEGER DEFAULT 0,
  created_at)
UNIQUE (tenant_id, alias, product_id)
CREATE INDEX ON product_aliases USING GIN (alias gin_trgm_ops);
```

Seed for every product: brand name, generic name, Urdu name, and generated common misspellings (character transposition, doubled letters, vowel substitution).

## 6. Matching engine

```rust
pub struct MatchCandidate {
    pub product_id: ProductId,
    pub score: f32,             // 0.0–1.0
    pub method: MatchMethod,    // Exact | Alias | Trigram | Phonetic | Vector | Hybrid
    pub matched_on: String,
}

pub async fn match_product(
    ctx: &TenantContext, pool: &PgPool,
    query: &str, limit: usize,
) -> Result<Vec<MatchCandidate>, CatalogError>;
```

Pipeline:
1. **Normalise** — lowercase, strip punctuation, collapse whitespace, normalise Arabic-Indic digits to ASCII, strip Urdu diacritics, unify ی/ي and ک/ك.
2. **Exact alias lookup** → score 1.00, short-circuit if unique.
3. **Trigram** via `pg_trgm` `similarity()` → score × 0.40
4. **Phonetic** via Urdu-tuned Double Metaphone → score × 0.35
5. **Vector** via `pgvector` cosine on `bge-m3` embeddings → score × 0.25
6. **Combine** — max of exact/alias, else weighted sum of the other three, normalised.
7. **Boost** by `hit_count` (log-scaled, max +0.05) and by in-stock status when the caller supplies a branch.

Thresholds:
| Score | Action |
|---|---|
| ≥ 0.85 | auto-suggest — **non-Rx items only** |
| 0.55–0.85 | present top 3 as a choice |
| < 0.55 | escalate to human |

### 6.1 Urdu-tuned phonetics

Standard Double Metaphone assumes English. Apply these equivalence classes before encoding:

```
kh ↔ x ↔ k        ph ↔ f          gh ↔ g
ee ↔ i ↔ y        oo ↔ u ↔ w      aa ↔ a
th ↔ t            dh ↔ d          ch ↔ c
z ↔ j (common PK substitution)
silent trailing h dropped
doubled consonants collapsed
```
`panadol` / `pandol` / `panadal` / `panadole` / `pinadol` must all encode identically. Table-driven test with at least 40 real-world variants.

## 7. Alias learning — build now, used by Doc 09

```rust
pub async fn learn_alias(
    ctx: &TenantContext, pool: &PgPool,
    raw_text: &str, confirmed: ProductId, source: AliasSource,
) -> Result<(), CatalogError>;
```

When a pharmacist corrects an OCR line, this inserts the normalised raw text as a new alias with `weight = 0.9`, `source = PHARMACIST_CORRECTION`. Next occurrence is an exact hit.

Guards: skip if the alias already maps to a *different* product with higher weight — flag for review instead. Skip aliases under 3 characters. Skip pure numerics.

**This is what makes the system improve on your specific doctors' handwriting over time.** Do not defer it.

## 8. Substitution candidates

```rust
pub async fn substitution_candidates(
    ctx: &TenantContext, pool: &PgPool, product_id: ProductId,
) -> Result<Vec<SubstitutionCandidate>, CatalogError>;
```

Returns products sharing a generic at the same strength, or linked via `generic_equivalents`. Ordered by same-generic-same-strength first, then therapeutic equivalents.

**This is a data lookup, not a generation task.** The AI layer may only propose from this function's output. It may never invent an equivalence. Every candidate carries `requires_pharmacist_approval = true` unconditionally.

## 9. Endpoints

```
GET    /api/v1/products                ?q&category&rx_only&status&page
GET    /api/v1/products/:id
POST   /api/v1/products                [product.create]
PATCH  /api/v1/products/:id            [product.edit]
POST   /api/v1/products/match          {query, limit, branch_id?} → candidates
GET    /api/v1/products/:id/substitutes
GET    /api/v1/products/:id/aliases
POST   /api/v1/products/:id/aliases    [product.edit]
DELETE /api/v1/aliases/:id             [product.edit]
POST   /api/v1/products/import         CSV/XLSX  [product.create]
GET    /api/v1/products/export
GET    /api/v1/generics                ?q
```

## 10. Acceptance tests

- `match_exact_brand_returns_score_one`
- `match_urdu_script_query_finds_product`
- `match_roman_urdu_misspelling_finds_product` — 40+ variant table
- `phonetic_equivalence_table` — all listed equivalence classes
- `match_below_threshold_escalates`
- `match_ambiguous_returns_multiple_candidates`
- `learn_alias_creates_exact_hit_next_time` — match, fail, learn, match, succeed
- `learn_alias_conflict_flags_for_review_not_overwrite`
- `learn_alias_rejects_short_and_numeric_input`
- `sale_price_above_mrp_rejected`
- `mrp_override_still_capped_and_audited`
- `substitutes_only_from_generic_equivalents_table`
- `substitutes_always_require_pharmacist_approval`
- `cross_tenant_product_invisible`
- `bulk_import_5000_products_under_30s`

## 11. Done checklist

- [ ] Product/category/generic CRUD with MRP hard block
- [ ] Alias table with GIN trigram index, seeded for every product
- [ ] Four-signal matching with documented weights and thresholds
- [ ] Urdu-tuned phonetic encoder, 40+ variant test passing
- [ ] `learn_alias` implemented with conflict guards
- [ ] Substitution restricted to `generic_equivalents`
- [ ] Bulk import/export
- [ ] All 15 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
