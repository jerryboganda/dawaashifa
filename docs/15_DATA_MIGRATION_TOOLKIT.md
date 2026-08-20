# DOC 15 — DATA MIGRATION TOOLKIT

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 05, 06
**Produces:** `crates/migration-tool` (separate binary)
**Branch:** `feat/15-migration-toolkit`

> **Blocked until sample exports are supplied.** Build the framework and the generic importers now; write source-specific mappings only against real files. Guessing at another system's schema wastes the effort.

---

## 1. Objective

Import product masters, stock, customers and historical orders from heterogeneous legacy sources — SQL dumps, Excel workbooks, CSV, and POS exports — into the platform, with validation and full reversibility.

## 2. In scope

- Pluggable source adapters (MySQL, MSSQL, Postgres, Excel, CSV, JSON)
- Column mapping configuration in YAML, not code
- Dry-run mode producing a full report with no writes
- Staging tables with validation before promotion
- Deduplication and fuzzy matching against existing records
- Automatic alias generation from legacy names
- Rollback by import batch
- Progress reporting for long imports

## 3. Out of scope — do NOT build

- A GUI import wizard (CLI plus a status endpoint is enough)
- Real-time two-way sync with a legacy system
- Migration of legacy user accounts or password hashes

## 4. Architecture

```
source file/db → SourceAdapter → RawRecord → Mapper → StagedRecord
              → Validator → [dry-run report] → Promoter → live tables
```

Every stage is inspectable. An operator must be able to see exactly why record 4,812 was rejected.

```rust
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn kind(&self) -> SourceKind;
    async fn probe(&self) -> Result<SourceSchema, MigrationError>;  // discover columns
    async fn stream(&self) -> BoxStream<Result<RawRecord, MigrationError>>;
    async fn count(&self) -> Result<u64, MigrationError>;
}
```

`probe()` matters: the operator points the tool at an unknown file and it reports what it found before any mapping is written.

## 5. Mapping configuration

```yaml
# mappings/legacy_pos_products.yaml
source:
  kind: mysql
  table: tbl_medicine
  connection_env: LEGACY_DB_URL
target: products
batch_size: 500

fields:
  sku:                  { from: med_code, required: true, transform: trim_upper }
  name_en:              { from: med_name, required: true, transform: title_case }
  name_ur:              { from: med_name_urdu }
  manufacturer:         { from: company_name }
  strength:             { from: strength, transform: normalize_strength }
  pack_size:            { from: pack, transform: parse_pack_size }
  mrp:                  { from: retail_price, required: true, transform: parse_decimal }
  is_prescription_only: { from: is_rx, transform: parse_bool, default: false }
  drap_registration_no: { from: reg_no }
  requires_cold_chain:  { from: storage_type, transform: cold_chain_from_storage }

aliases:
  generate_from: [med_name, med_name_urdu, short_name, alt_name]

dedupe:
  strategy: fuzzy
  match_on: [sku, name_en]
  threshold: 0.92
  on_match: skip     # skip | update | create_duplicate_flagged

validations:
  - field: mrp
    rule: greater_than_zero
  - field: sku
    rule: unique_within_batch
```

New sources need a YAML file, not Rust changes. If a source needs a new `transform`, add it to the shared transform library — do not write a bespoke importer.

## 6. Built-in transforms

`trim`, `trim_upper`, `title_case`, `parse_decimal`, `parse_bool`, `parse_date` (multi-format, including DD/MM/YYYY which is standard locally), `normalize_strength` (`500MG` / `500 mg` / `0.5g` → `500mg`), `parse_pack_size` (`10's` / `1x10` / `Strip of 10` → `10`), `normalize_phone` (local → E.164), `arabic_digits_to_ascii`, `cold_chain_from_storage`.

`normalize_phone` matters more than it looks: legacy customer data will contain `0300-1234567`, `+92 300 1234567`, `92 3001234567`, and `03001234567` for the same person. Without normalisation you import four customers.

## 7. Dry run

`--dry-run` is the default. Writing requires an explicit `--commit`.

Report contains: total records, would-insert, would-update, would-skip, rejected with reasons grouped by rule, fuzzy-match candidates for operator review, and a sample of 20 transformed records for eyeballing.

**No operator should ever run a commit without reading a dry-run report first.** Document this in the runbook.

## 8. Staging and promotion

```sql
import_batches(id, tenant_id, source_kind, mapping_name, started_at,
               completed_at, status, total, inserted, updated, skipped,
               rejected, dry_run, initiated_by, rollback_at)
import_staging(id, batch_id, row_no, raw JSONB, mapped JSONB,
               validation_errors JSONB, status, target_id)
```

Promotion runs in transactional chunks. Every promoted row records `import_batch_id` on the target table, which is what makes rollback possible.

## 9. Rollback

`migration-tool rollback --batch-id <uuid>`

- Deletes rows inserted by that batch, if they have not since been referenced
- Restores previous values for rows updated by that batch, from the staging snapshot
- **Refuses** to roll back if any affected row now has dependent records — an imported product that has since been sold cannot be un-imported. Report which rows blocked it.
- Rollback of a rollback is not supported; take a database backup before large imports

## 10. Alias generation

For every imported product, generate aliases from every legacy name column, plus:
- Common misspellings (single-character transposition, doubled letters, dropped vowels)
- Urdu transliteration of the English name where a name is Latin-only
- The generic name where it can be resolved

This front-loads the matching engine's accuracy so day-one performance is not cold-start bad.

## 11. Historical orders

Optional and lower priority. If imported:
- Land in a terminal status (`Closed`), never in a state the machine would try to advance
- Do not generate stock movements — history is a record, not an event to replay
- Do not create invoices or attempt FBR submission
- Flag with `is_historical = true`

## 12. CLI

```bash
migration-tool probe --source mysql --connection-env LEGACY_DB_URL --table tbl_medicine
migration-tool validate --mapping mappings/legacy_pos_products.yaml
migration-tool run --mapping mappings/legacy_pos_products.yaml --dry-run
migration-tool run --mapping mappings/legacy_pos_products.yaml --commit
migration-tool status --batch-id <uuid>
migration-tool rollback --batch-id <uuid>
```

## 13. Acceptance tests

- `probe_discovers_columns_from_unknown_source`
- `dry_run_writes_nothing` — assert row counts unchanged
- `commit_required_for_writes`
- `fuzzy_dedupe_matches_above_threshold`
- `phone_normalisation_collapses_four_formats_to_one_customer`
- `strength_normalisation_table`
- `pack_size_parsing_table`
- `ddmmyyyy_dates_parsed_correctly`
- `arabic_digits_converted`
- `validation_errors_grouped_by_rule_in_report`
- `rollback_removes_inserted_rows`
- `rollback_restores_updated_rows`
- `rollback_refused_when_dependent_records_exist`
- `aliases_generated_for_every_imported_product`
- `historical_orders_land_in_terminal_status`
- `historical_orders_create_no_stock_movements`
- `import_of_50000_products_completes_under_5_minutes`

## 14. Done checklist

- [ ] Six source adapters with working `probe()`
- [ ] YAML mapping; new sources need no Rust changes
- [ ] Full transform library including phone and strength normalisation
- [ ] Dry-run default, explicit commit flag
- [ ] Staging tables with per-row validation errors retained
- [ ] Rollback with dependency safety refusal
- [ ] Alias generation on every product import
- [ ] Historical orders inert — terminal status, no movements, no invoices
- [ ] All 17 acceptance tests green
- [ ] Runbook at `docs/runbooks/data-migration.md`
