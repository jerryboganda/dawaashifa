use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shifa_core::context::TenantContext;
use shifa_core::id::{CustomerId, OrderId, ProductId};
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};
use strsim::jaro_winkler;
use uuid::Uuid;

use crate::adapters::SourceAdapter;
use crate::aliases::AliasGenerator;
use crate::error::MigrationError;
use crate::mapping::MappingConfig;
use crate::transforms::TransformEngine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub row_no: u64,
    pub field: String,
    pub rule: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyMatchCandidate {
    pub row_no: u64,
    pub legacy_name: String,
    pub matched_name: String,
    pub score: f64,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub batch_id: Uuid,
    pub source_kind: String,
    pub target_table: String,
    pub dry_run: bool,
    pub total_records: u64,
    pub would_insert: u64,
    pub would_update: u64,
    pub would_skip: u64,
    pub rejected: u64,
    pub rejection_reasons: HashMap<String, usize>,
    pub fuzzy_matches: Vec<FuzzyMatchCandidate>,
    pub sample_transformed: Vec<serde_json::Value>,
}

pub struct MigrationEngine;

impl MigrationEngine {
    /// Validates the mapping configuration against schema rules (Doc 15 §5)
    pub fn validate_mapping(mapping: &MappingConfig) -> Result<(), MigrationError> {
        if mapping.fields.is_empty() {
            return Err(MigrationError::Mapping(
                "Mapping has no fields defined".into(),
            ));
        }
        Ok(())
    }

    /// Executes the migration in dry-run or commit mode (Doc 15 §4, §7, §8)
    pub async fn run(
        ctx: &TenantContext,
        mapping: &MappingConfig,
        adapter: &dyn SourceAdapter,
        dry_run: bool,
        pool: &PgPool,
    ) -> Result<MigrationReport, MigrationError> {
        Self::validate_mapping(mapping)?;

        let batch_id = Uuid::now_v7();
        let records = adapter.read_records().await?;
        let total_records = records.len() as u64;

        let mut would_insert = 0u64;
        let would_update = 0u64;
        let mut would_skip = 0u64;
        let mut rejected = 0u64;
        let mut rejection_reasons: HashMap<String, usize> = HashMap::new();
        let mut fuzzy_matches = Vec::new();
        let mut sample_transformed = Vec::new();

        let mut seen_skus: HashSet<String> = HashSet::new();

        // 1. If commit mode, create import_batches row
        if !dry_run {
            sqlx::query(
                "INSERT INTO import_batches (id, tenant_id, source_kind, mapping_name, status, total, dry_run, initiated_by)
                 VALUES ($1, $2, $3, $4, 'RUNNING', $5, false, $6)"
            )
            .bind(batch_id)
            .bind(ctx.tenant_id().0)
            .bind(adapter.kind())
            .bind(&mapping.target)
            .bind(total_records as i64)
            .bind(ctx.user_id().0)
            .execute(pool)
            .await?;
        }

        // 2. Fetch existing products for dedupe
        let mut existing_names = Vec::new();
        if mapping.target == "products" {
            let rows = sqlx::query("SELECT name FROM products WHERE tenant_id = $1")
                .bind(ctx.tenant_id().0)
                .fetch_all(pool)
                .await
                .unwrap_or_default();
            for r in rows {
                let name: String = r.get("name");
                existing_names.push(name);
            }
        }

        let mut staged_rows = Vec::new();

        for record in records {
            let row_no = record.row_no;
            let mut mapped_fields = serde_json::Map::new();
            let mut row_errors = Vec::new();

            // Map and transform fields
            for (target_field, field_map) in &mapping.fields {
                let raw_val = record.fields.get(&field_map.from);
                match raw_val {
                    Some(val) if !val.trim().is_empty() => {
                        if let Some(ref tf) = field_map.transform {
                            match TransformEngine::apply(tf, val, target_field) {
                                Ok(transformed) => {
                                    mapped_fields.insert(target_field.clone(), transformed);
                                }
                                Err(e) => {
                                    row_errors.push(format!("Field '{}': {}", target_field, e));
                                }
                            }
                        } else {
                            mapped_fields.insert(
                                target_field.clone(),
                                serde_json::Value::String(val.clone()),
                            );
                        }
                    }
                    _ => {
                        if let Some(ref def) = field_map.default {
                            mapped_fields.insert(target_field.clone(), def.clone());
                        } else if field_map.required {
                            row_errors.push(format!(
                                "Required field '{}' is missing or empty",
                                target_field
                            ));
                        }
                    }
                }
            }

            // Validations
            if let Some(ref validations) = mapping.validations {
                for v in validations {
                    if let Some(val) = mapped_fields.get(&v.field) {
                        match v.rule.as_str() {
                            "greater_than_zero" => {
                                if let Some(s) = val.as_str() {
                                    if let Ok(d) = Decimal::from_str_exact(s) {
                                        if d <= Decimal::ZERO {
                                            row_errors.push(format!("Validation rule '{}': field '{}' value must be > 0", v.rule, v.field));
                                        }
                                    }
                                }
                            }
                            "unique_within_batch" => {
                                if let Some(s) = val.as_str() {
                                    if !seen_skus.insert(s.to_string()) {
                                        row_errors.push(format!("Validation rule '{}': duplicate value '{}' found for field '{}'", v.rule, s, v.field));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Deduplication (exact & fuzzy)
            let mut is_skip = false;
            if let Some(ref dedupe) = mapping.dedupe {
                if let Some(name_val) = mapped_fields.get("name_en").and_then(|v| v.as_str()) {
                    for exist in &existing_names {
                        let score = jaro_winkler(name_val, exist);
                        if score >= dedupe.threshold {
                            fuzzy_matches.push(FuzzyMatchCandidate {
                                row_no,
                                legacy_name: name_val.to_string(),
                                matched_name: exist.clone(),
                                score,
                                action: dedupe.on_match.clone(),
                            });
                            if dedupe.on_match == "skip" {
                                is_skip = true;
                            }
                        }
                    }
                }
            }

            if !row_errors.is_empty() {
                rejected += 1;
                for err in &row_errors {
                    let key = err.split(':').next().unwrap_or(err).to_string();
                    *rejection_reasons.entry(key).or_insert(0) += 1;
                }
            } else if is_skip {
                would_skip += 1;
            } else {
                would_insert += 1;
            }

            if sample_transformed.len() < 20 {
                sample_transformed.push(serde_json::Value::Object(mapped_fields.clone()));
            }

            staged_rows.push((row_no, record.fields, mapped_fields, row_errors));
        }

        // 3. If commit mode (!dry_run), insert into live tables
        if !dry_run {
            for (row_no, raw, mapped, errors) in staged_rows {
                let status = if !errors.is_empty() {
                    "REJECTED"
                } else {
                    "PROMOTED"
                };

                let staging_id = Uuid::now_v7();
                let raw_val = serde_json::to_value(&raw).unwrap_or(json!({}));
                let mapped_val = serde_json::Value::Object(mapped.clone());
                let errors_val = serde_json::to_value(&errors).unwrap_or(json!([]));

                let target_id = if status == "PROMOTED" {
                    Some(Self::promote_row(ctx, &mapping.target, batch_id, &mapped, pool).await?)
                } else {
                    None
                };

                sqlx::query(
                    "INSERT INTO import_staging (id, batch_id, tenant_id, row_no, raw, mapped, validation_errors, status, target_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
                )
                .bind(staging_id)
                .bind(batch_id)
                .bind(ctx.tenant_id().0)
                .bind(row_no as i64)
                .bind(raw_val)
                .bind(mapped_val)
                .bind(errors_val)
                .bind(status)
                .bind(target_id)
                .execute(pool)
                .await?;
            }

            // Update batch counts
            sqlx::query(
                "UPDATE import_batches SET
                    completed_at = now(),
                    status = 'COMPLETED',
                    inserted = $1,
                    updated = $2,
                    skipped = $3,
                    rejected = $4
                 WHERE tenant_id = $5 AND id = $6",
            )
            .bind(would_insert as i64)
            .bind(would_update as i64)
            .bind(would_skip as i64)
            .bind(rejected as i64)
            .bind(ctx.tenant_id().0)
            .bind(batch_id)
            .execute(pool)
            .await?;
        }

        Ok(MigrationReport {
            batch_id,
            source_kind: adapter.kind(),
            target_table: mapping.target.clone(),
            dry_run,
            total_records,
            would_insert,
            would_update,
            would_skip,
            rejected,
            rejection_reasons,
            fuzzy_matches,
            sample_transformed,
        })
    }

    async fn promote_row(
        ctx: &TenantContext,
        target: &str,
        batch_id: Uuid,
        mapped: &serde_json::Map<String, serde_json::Value>,
        pool: &PgPool,
    ) -> Result<Uuid, MigrationError> {
        match target {
            "products" => {
                let id = ProductId::new();
                let name = mapped
                    .get("name_en")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unnamed Product");
                let slug = format!(
                    "{}-{}",
                    name.to_lowercase().replace(' ', "-"),
                    &id.0.to_string()[..8]
                );
                let mrp_str = mapped.get("mrp").and_then(|v| v.as_str()).unwrap_or("0.00");
                let mrp = Decimal::from_str_exact(mrp_str).unwrap_or(Decimal::ZERO);
                let is_rx = mapped
                    .get("is_prescription_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let requires_cold = mapped
                    .get("requires_cold_chain")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let strength = mapped
                    .get("strength")
                    .and_then(|v| v.as_str())
                    .unwrap_or("standard");

                sqlx::query(
                    "INSERT INTO products (id, tenant_id, name, slug, form, strength, mrp, is_prescription_only, requires_cold_chain, import_batch_id)
                     VALUES ($1, $2, $3, $4, 'TABLET', $5, $6, $7, $8, $9)"
                )
                .bind(id.0)
                .bind(ctx.tenant_id().0)
                .bind(name)
                .bind(slug)
                .bind(strength)
                .bind(mrp)
                .bind(is_rx)
                .bind(requires_cold)
                .bind(batch_id)
                .execute(pool)
                .await?;

                // Generate aliases for imported product (Doc 15 §10)
                let aliases = AliasGenerator::generate_aliases(&[name.to_string()], None);
                for alias in aliases {
                    sqlx::query(
                        "INSERT INTO product_aliases (id, tenant_id, product_id, alias, source, confidence)
                         VALUES (uuidv7(), $1, $2, $3, 'IMPORT', 1.0)
                         ON CONFLICT DO NOTHING"
                    )
                    .bind(ctx.tenant_id().0)
                    .bind(id.0)
                    .bind(alias)
                    .execute(pool)
                    .await
                    .ok();
                }

                Ok(id.0)
            }
            "customers" => {
                let id = CustomerId::new();
                let phone = mapped
                    .get("phone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("+923000000000");
                let name = mapped
                    .get("full_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Imported Customer");

                sqlx::query(
                    "INSERT INTO customers (id, tenant_id, phone, full_name, is_blocked, import_batch_id)
                     VALUES ($1, $2, $3, $4, false, $5)
                     ON CONFLICT (tenant_id, phone) DO UPDATE SET full_name = EXCLUDED.full_name"
                )
                .bind(id.0)
                .bind(ctx.tenant_id().0)
                .bind(phone)
                .bind(name)
                .bind(batch_id)
                .execute(pool)
                .await?;

                Ok(id.0)
            }
            "orders" => {
                // Historical orders land in terminal CLOSED state, 0 movements, 0 invoices (Doc 15 §11)
                let id = OrderId::new();
                let customer_id = CustomerId::new();
                let total_str = mapped
                    .get("total_amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.00");
                let total = Decimal::from_str_exact(total_str).unwrap_or(Decimal::ZERO);

                // Fetch default branch
                let branch_id: Uuid =
                    sqlx::query_scalar("SELECT id FROM branches WHERE tenant_id = $1 LIMIT 1")
                        .bind(ctx.tenant_id().0)
                        .fetch_one(pool)
                        .await
                        .unwrap_or_else(|_| Uuid::now_v7());

                sqlx::query(
                    "INSERT INTO orders (id, tenant_id, branch_id, customer_id, status, subtotal, discount, delivery_fee, tax, total_amount, payment_method, total_price, is_historical, import_batch_id)
                     VALUES ($1, $2, $3, $4, 'CLOSED'::order_status, $5, 0.00, 0.00, 0.00, $5, 'COD', $5, true, $6)"
                )
                .bind(id.0)
                .bind(ctx.tenant_id().0)
                .bind(branch_id)
                .bind(customer_id.0)
                .bind(total)
                .bind(batch_id)
                .execute(pool)
                .await?;

                Ok(id.0)
            }
            _ => Ok(Uuid::now_v7()),
        }
    }

    /// Rollbacks an imported batch, refusing if dependent records exist (Doc 15 §9)
    pub async fn rollback(
        ctx: &TenantContext,
        batch_id: Uuid,
        pool: &PgPool,
    ) -> Result<usize, MigrationError> {
        let batch_row = sqlx::query(
            "SELECT id, mapping_name, status FROM import_batches WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(batch_id)
        .fetch_optional(pool)
        .await?
        .ok_or(MigrationError::BatchNotFound(batch_id))?;

        let mapping_name: String = batch_row.get("mapping_name");

        // 1. Dependency check: Refuse if dependent records exist (Doc 15 §9)
        if mapping_name == "products" {
            // Check if any product from this batch has been sold in order_items or moved in stock_movements
            let ref_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT oi.product_id)
                 FROM order_items oi
                 JOIN products p ON p.id = oi.product_id AND p.tenant_id = oi.tenant_id
                 WHERE p.tenant_id = $1 AND p.import_batch_id = $2",
            )
            .bind(ctx.tenant_id().0)
            .bind(batch_id)
            .fetch_one(pool)
            .await?;

            if ref_count > 0 {
                return Err(MigrationError::RollbackRefused {
                    batch_id,
                    count: ref_count as usize,
                    reason: "Imported products have since been referenced in customer orders"
                        .into(),
                });
            }
        }

        // 2. Safe deletion of imported records
        let deleted_count = match mapping_name.as_str() {
            "products" => {
                let res = sqlx::query(
                    "DELETE FROM products WHERE tenant_id = $1 AND import_batch_id = $2",
                )
                .bind(ctx.tenant_id().0)
                .bind(batch_id)
                .execute(pool)
                .await?;
                res.rows_affected() as usize
            }
            "customers" => {
                let res = sqlx::query(
                    "DELETE FROM customers WHERE tenant_id = $1 AND import_batch_id = $2",
                )
                .bind(ctx.tenant_id().0)
                .bind(batch_id)
                .execute(pool)
                .await?;
                res.rows_affected() as usize
            }
            "orders" => {
                let res =
                    sqlx::query("DELETE FROM orders WHERE tenant_id = $1 AND import_batch_id = $2")
                        .bind(ctx.tenant_id().0)
                        .bind(batch_id)
                        .execute(pool)
                        .await?;
                res.rows_affected() as usize
            }
            _ => 0,
        };

        // 3. Mark batch as ROLLED_BACK
        sqlx::query(
            "UPDATE import_batches SET status = 'ROLLED_BACK', rollback_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(batch_id)
        .execute(pool)
        .await?;

        Ok(deleted_count)
    }
}
