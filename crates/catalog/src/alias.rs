use crate::error::CatalogError;
use crate::phonetics::normalize_query;
use shifa_core::context::TenantContext;
use shifa_core::id::ProductId;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Learn alias dynamically from pharmacist corrections (called by OCR/Rx module per Doc 05 §7).
pub async fn learn_alias(
    ctx: &TenantContext,
    pool: &PgPool,
    raw_text: &str,
    confirmed_product: ProductId,
    source: &str,
) -> Result<(), CatalogError> {
    let normalized = normalize_query(raw_text);

    if normalized.len() < 3 {
        return Err(CatalogError::InvalidAlias(
            raw_text.to_string(),
            "Alias must be at least 3 characters",
        ));
    }

    if normalized
        .chars()
        .all(|c| c.is_numeric() || c.is_whitespace())
    {
        return Err(CatalogError::InvalidAlias(
            raw_text.to_string(),
            "Pure numeric aliases are not allowed",
        ));
    }

    let existing = sqlx::query(
        "SELECT product_id, weight FROM product_aliases
         WHERE tenant_id = $1 AND alias = $2",
    )
    .bind(ctx.tenant_id.0)
    .bind(&normalized)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = existing {
        let existing_pid: Uuid = row.get("product_id");
        let weight: rust_decimal::Decimal = row.get("weight");
        let existing_pid = ProductId::from(existing_pid);

        if existing_pid != confirmed_product && weight >= rust_decimal::Decimal::new(8, 1) {
            return Err(CatalogError::AliasConflict(normalized, existing_pid));
        }

        if existing_pid == confirmed_product {
            sqlx::query(
                "UPDATE product_aliases SET hit_count = hit_count + 1
                 WHERE tenant_id = $1 AND alias = $2 AND product_id = $3",
            )
            .bind(ctx.tenant_id.0)
            .bind(&normalized)
            .bind(confirmed_product.0)
            .execute(pool)
            .await?;
            return Ok(());
        }
    }

    sqlx::query(
        "INSERT INTO product_aliases (id, tenant_id, product_id, alias, alias_type, script, weight, source, hit_count)
         VALUES ($1, $2, $3, $4, 'MISPELLING', 'LATIN', 0.90, $5, 1)
         ON CONFLICT (tenant_id, alias, product_id)
         DO UPDATE SET hit_count = product_aliases.hit_count + 1"
    )
    .bind(Uuid::now_v7())
    .bind(ctx.tenant_id.0)
    .bind(confirmed_product.0)
    .bind(&normalized)
    .bind(source)
    .execute(pool)
    .await?;

    Ok(())
}
