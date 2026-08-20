use crate::error::CatalogError;
use crate::models::SubstitutionCandidate;
use shifa_core::context::TenantContext;
use shifa_core::id::ProductId;
use shifa_core::money::Money;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Look up generic and therapeutic substitution candidates for a given product per Doc 05 §8.
/// Returns candidates ordered by same-generic-same-strength first, then therapeutic equivalents.
/// Invariant: Data lookup only, never generates unverified candidate. Every candidate carries `requires_pharmacist_approval: true`.
pub async fn substitution_candidates(
    ctx: &TenantContext,
    pool: &PgPool,
    product_id: ProductId,
) -> Result<Vec<SubstitutionCandidate>, CatalogError> {
    let original = sqlx::query(
        "SELECT id, brand_name, generic_name, strength, mrp
         FROM products
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(ctx.tenant_id.0)
    .bind(product_id.0)
    .fetch_optional(pool)
    .await?;

    let orig = match original {
        Some(row) => row,
        None => return Err(CatalogError::ProductNotFound(product_id)),
    };

    let orig_mrp: rust_decimal::Decimal = orig.get("mrp");
    let orig_generic: Option<String> = orig.get("generic_name");
    let orig_strength: Option<String> = orig.get("strength");

    let mut candidates = Vec::new();

    if let (Some(generic), Some(strength)) = (orig_generic, orig_strength) {
        let rows = sqlx::query(
            "SELECT id, brand_name, generic_name, strength, mrp
             FROM products
             WHERE tenant_id = $1 AND id != $2 AND generic_name = $3 AND strength = $4 AND status = 'ACTIVE'
             ORDER BY mrp ASC"
        )
        .bind(ctx.tenant_id.0)
        .bind(product_id.0)
        .bind(&generic)
        .bind(&strength)
        .fetch_all(pool)
        .await?;

        for row in rows {
            let pid: Uuid = row.get("id");
            let brand_name: String = row.get("brand_name");
            let gname: String = row.get("generic_name");
            let strn: String = row.get("strength");
            let mrp: rust_decimal::Decimal = row.get("mrp");

            let savings = if orig_mrp > mrp {
                orig_mrp - mrp
            } else {
                rust_decimal::Decimal::ZERO
            };

            candidates.push(SubstitutionCandidate {
                product_id: ProductId::from(pid),
                brand_name,
                generic_name: gname,
                strength: strn,
                mrp: Money::from_decimal(mrp),
                savings_vs_original: Money::from_decimal(savings),
                equivalence_type: "SAME_GENERIC_SAME_STRENGTH".to_string(),
                requires_pharmacist_approval: true,
            });
        }
    }

    let equiv_rows = sqlx::query(
        "SELECT p.id, p.brand_name, p.generic_name, p.strength, p.mrp
         FROM generic_equivalents ge
         JOIN products p ON p.generic_id = ge.equivalent_generic_id AND p.tenant_id = ge.tenant_id
         WHERE ge.tenant_id = $1 AND ge.source_generic_id = (
             SELECT generic_id FROM products WHERE id = $2 AND tenant_id = $1
         ) AND p.id != $2 AND p.status = 'ACTIVE'
         ORDER BY p.mrp ASC",
    )
    .bind(ctx.tenant_id.0)
    .bind(product_id.0)
    .fetch_all(pool)
    .await?;

    for row in equiv_rows {
        let pid: Uuid = row.get("id");
        let brand_name: String = row.get("brand_name");
        let gname: Option<String> = row.get("generic_name");
        let strn: Option<String> = row.get("strength");
        let mrp: rust_decimal::Decimal = row.get("mrp");

        let savings = if orig_mrp > mrp {
            orig_mrp - mrp
        } else {
            rust_decimal::Decimal::ZERO
        };

        candidates.push(SubstitutionCandidate {
            product_id: ProductId::from(pid),
            brand_name,
            generic_name: gname.unwrap_or_default(),
            strength: strn.unwrap_or_default(),
            mrp: Money::from_decimal(mrp),
            savings_vs_original: Money::from_decimal(savings),
            equivalence_type: "THERAPEUTIC".to_string(),
            requires_pharmacist_approval: true,
        });
    }

    Ok(candidates)
}
