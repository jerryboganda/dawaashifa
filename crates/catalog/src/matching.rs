use crate::error::CatalogError;
use crate::models::*;
use crate::phonetics::{encode_urdu_phonetic, normalize_query};
use shifa_core::context::TenantContext;
use shifa_core::id::ProductId;
use shifa_core::money::Money;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use strsim::jaro_winkler;
use uuid::Uuid;

/// Four-signal product matching engine per Doc 05 §6
pub async fn match_product(
    ctx: &TenantContext,
    pool: &PgPool,
    req: &MatchRequest,
) -> Result<Vec<MatchCandidate>, CatalogError> {
    let normalized = normalize_query(&req.query);
    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    let query_phonetic = encode_urdu_phonetic(&normalized);

    // 1. Step 1: Exact alias lookup (score = 1.0)
    let exact_alias_rows = sqlx::query(
        "SELECT a.product_id, a.alias, p.brand_name, p.strength, p.is_prescription_only, p.mrp, a.hit_count
         FROM product_aliases a
         JOIN products p ON p.id = a.product_id AND p.tenant_id = a.tenant_id
         WHERE a.tenant_id = $1 AND lower(a.alias) = $2 AND p.status = 'ACTIVE'"
    )
    .bind(ctx.tenant_id.0)
    .bind(&normalized)
    .fetch_all(pool)
    .await?;

    if !exact_alias_rows.is_empty() {
        let candidates: Vec<MatchCandidate> = exact_alias_rows
            .into_iter()
            .map(|r| {
                let pid: Uuid = r.get("product_id");
                let brand: String = r.get("brand_name");
                let strength: Option<String> = r.get("strength");
                let is_rx: bool = r.get("is_prescription_only");
                let mrp: rust_decimal::Decimal = r.get("mrp");
                let alias: String = r.get("alias");

                MatchCandidate {
                    product_id: ProductId::from(pid),
                    brand_name: brand,
                    strength,
                    score: 1.0,
                    method: MatchMethod::Exact,
                    matched_on: alias,
                    is_prescription_only: is_rx,
                    mrp: Money::from_decimal(mrp),
                }
            })
            .collect();

        if candidates.len() == 1 {
            return Ok(candidates);
        }
    }

    // 2. Step 2: Multi-signal fuzzy candidate retrieval (Trigram, Phonetic, Alias)
    let candidate_rows = sqlx::query(
        "SELECT a.product_id, a.alias, p.brand_name, p.strength, p.is_prescription_only, p.mrp, a.hit_count
         FROM product_aliases a
         JOIN products p ON p.id = a.product_id AND p.tenant_id = a.tenant_id
         WHERE a.tenant_id = $1 AND p.status = 'ACTIVE'
         LIMIT 200"
    )
    .bind(ctx.tenant_id.0)
    .fetch_all(pool)
    .await?;

    let mut scored_map: HashMap<Uuid, MatchCandidate> = HashMap::new();

    for row in candidate_rows {
        let pid: Uuid = row.get("product_id");
        let brand: String = row.get("brand_name");
        let strength: Option<String> = row.get("strength");
        let is_rx: bool = row.get("is_prescription_only");
        let mrp: rust_decimal::Decimal = row.get("mrp");
        let alias: String = row.get("alias");
        let hit_count: i32 = row.get("hit_count");

        let alias_norm = normalize_query(&alias);
        let alias_phonetic = encode_urdu_phonetic(&alias_norm);

        let trigram_score = jaro_winkler(&normalized, &alias_norm) as f32;

        let phonetic_score = if !query_phonetic.is_empty() && query_phonetic == alias_phonetic {
            1.0
        } else if !query_phonetic.is_empty() && !alias_phonetic.is_empty() {
            jaro_winkler(&query_phonetic, &alias_phonetic) as f32
        } else {
            0.0
        };

        let secondary_score = jaro_winkler(&normalized, &normalize_query(&brand)) as f32;

        let mut final_score =
            (trigram_score * 0.40) + (phonetic_score * 0.35) + (secondary_score * 0.25);

        let hit_boost = ((hit_count as f32 + 1.0).ln() * 0.01).min(0.05);
        final_score = (final_score + hit_boost).min(1.0);

        if final_score > 0.40 {
            let candidate = MatchCandidate {
                product_id: ProductId::from(pid),
                brand_name: brand,
                strength,
                score: final_score,
                method: if phonetic_score >= 0.95 {
                    MatchMethod::Phonetic
                } else {
                    MatchMethod::Hybrid
                },
                matched_on: alias,
                is_prescription_only: is_rx,
                mrp: Money::from_decimal(mrp),
            };

            scored_map
                .entry(pid)
                .and_modify(|existing| {
                    if candidate.score > existing.score {
                        *existing = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }
    }

    let mut results: Vec<MatchCandidate> = scored_map.into_values().collect();
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(req.limit);

    Ok(results)
}
