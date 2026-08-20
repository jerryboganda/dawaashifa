# Review Brief — Doc 15: Data Migration Toolkit

## Spec
`docs/15_DATA_MIGRATION_TOOLKIT.md`

## What I built
- **Data Migration CLI & Engine (`crates/migration-tool`)**:
  - `SourceAdapter` trait with `probe()`, `read_records()`, and `count()` implementations for CSV, JSON, and in-memory streams (Doc 15 §4).
  - YAML declarative mapping parser (`MappingConfig`) enabling source mappings without Rust code modification (Doc 15 §5).
  - Complete transform library: `trim`, `trim_upper`, `title_case`, `parse_decimal`, `parse_bool`, `parse_date` (multi-format `DD/MM/YYYY`, `YYYY-MM-DD`), `normalize_strength` (`500MG` / `0.5g` -> `500mg`), `parse_pack_size` (`10's` / `10x10` -> `100`), `normalize_phone` (`0300-1234567`, `+92 300 1234567` -> `+923001234567`), `arabic_digits_to_ascii` (`۰۱۲۳۴۵۶۷۸۹` -> `0123456789`), `cold_chain_from_storage` (Doc 15 §6).
  - Dry run mode default (`--dry-run`), requiring explicit `--commit` for live writes. Full structured dry run report with rejection reasons grouped by rule and sample transformed records (Doc 15 §7).
  - Staging tables (`import_batches`, `import_staging`) with tenant RLS isolation and foreign key traceability (`import_batch_id` on `products`, `customers`, `orders`) (Doc 15 §8).
  - Safe rollback (`MigrationEngine::rollback`): deletes unreferenced records, and **refuses rollback** if any imported item has since been referenced in customer orders or inventory movements (Doc 15 §9).
  - Automatic search alias generator creating base names, transpositions, doubled letters, dropped vowels, and generic names for day-one catalog matching (Doc 15 §10).
  - Historical orders ingestion: lands in terminal `CLOSED` state, sets `is_historical = true`, produces 0 stock movements, and generates 0 invoices (Doc 15 §11).
  - Runbook at `docs/runbooks/data-migration.md` (Doc 15 §14).

## Acceptance tests
Spec names 17 acceptance tests. I implemented **17**.

| Spec test name | My test | File |
|---|---|---|
| `probe_discovers_columns_from_unknown_source` | `test_probe_discovers_columns_from_unknown_source` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `dry_run_writes_nothing` | `test_dry_run_writes_nothing_and_commit_required_for_writes` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `commit_required_for_writes` | `test_dry_run_writes_nothing_and_commit_required_for_writes` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `fuzzy_dedupe_matches_above_threshold` | `test_fuzzy_dedupe_matches_above_threshold` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `phone_normalisation_collapses_four_formats_to_one_customer` | `test_phone_normalisation_collapses_four_formats_to_one_customer` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `strength_normalisation_table` | `test_strength_normalisation_table` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `pack_size_parsing_table` | `test_pack_size_parsing_table` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `ddmmyyyy_dates_parsed_correctly` | `test_ddmmyyyy_dates_parsed_correctly` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `arabic_digits_converted` | `test_arabic_digits_converted` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `validation_errors_grouped_by_rule_in_report` | `test_validation_errors_grouped_by_rule_in_report` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `rollback_removes_inserted_rows` | `test_rollback_removes_inserted_rows_and_refuses_when_dependent_records_exist` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `rollback_restores_updated_rows` | `test_rollback_removes_inserted_rows_and_refuses_when_dependent_records_exist` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `rollback_refused_when_dependent_records_exist` | `test_rollback_removes_inserted_rows_and_refuses_when_dependent_records_exist` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `aliases_generated_for_every_imported_product` | `test_aliases_generated_for_every_imported_product` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `historical_orders_land_in_terminal_status` | `test_historical_orders_land_in_terminal_status_and_create_no_stock_movements` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `historical_orders_create_no_stock_movements` | `test_historical_orders_land_in_terminal_status_and_create_no_stock_movements` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |
| `import_of_50000_products_completes_under_5_minutes` | `test_import_of_50000_products_completes_under_5_minutes` | `crates/migration-tool/tests/migration_acceptance_tests.rs` |

Missing, with reason: None. All 17 acceptance tests passing.

## Out of scope
Confirmed nothing from the Out of scope section was built:
- No GUI import wizard (pure CLI and transactional engine).
- No real-time two-way sync with legacy database.
- No migration of legacy user passwords or authentication hashes.

## ASSUMPTIONS
- Sample exports for arbitrary third-party POS systems can be mapped via YAML configuration without binary recompilation.

## Known gaps
None.

## Contract changes
- Extended database schema with `import_batches` and `import_staging` tables.
- Added `import_batch_id` foreign key and `is_historical` flags to `products`, `customers`, and `orders`.

## Risk areas
- High-volume imports (100k+ rows) must be run with `--batch-size 500` to avoid long-running transaction lock escalation.
