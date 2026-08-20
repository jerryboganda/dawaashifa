use crate::error::CatalogError;
use crate::models::*;
use crate::phonetics::normalize_query;
use shifa_core::context::TenantContext;
use shifa_core::id::ProductId;
use shifa_core::money::Money;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CatalogService {
    pool: PgPool,
}

impl CatalogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_products(
        &self,
        ctx: &TenantContext,
        query: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ProductDto>, CatalogError> {
        let rows = if let Some(q) = query {
            let q_param = format!("%{}%", normalize_query(q));
            sqlx::query(
                "SELECT id, tenant_id, brand_name, generic_name, strength, dosage_form, pack_size, mrp, tp, cost_price, is_prescription_only, is_narcotic, is_refrigerated, manufacturer, barcode, status
                 FROM products
                 WHERE tenant_id = $1 AND (lower(brand_name) LIKE $2 OR lower(generic_name) LIKE $2)
                 ORDER BY brand_name ASC
                 LIMIT $3 OFFSET $4"
            )
            .bind(ctx.tenant_id.0)
            .bind(q_param)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, tenant_id, brand_name, generic_name, strength, dosage_form, pack_size, mrp, tp, cost_price, is_prescription_only, is_narcotic, is_refrigerated, manufacturer, barcode, status
                 FROM products
                 WHERE tenant_id = $1
                 ORDER BY brand_name ASC
                 LIMIT $2 OFFSET $3"
            )
            .bind(ctx.tenant_id.0)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        let products = rows
            .into_iter()
            .map(|r| ProductDto {
                id: ProductId::from(r.get::<Uuid, _>("id")),
                tenant_id: ctx.tenant_id,
                brand_name: r.get("brand_name"),
                generic_name: r.get("generic_name"),
                strength: r.get("strength"),
                dosage_form: r.get("dosage_form"),
                pack_size: r.get("pack_size"),
                mrp: Money::from_decimal(r.get::<rust_decimal::Decimal, _>("mrp")),
                tp: r
                    .get::<Option<rust_decimal::Decimal>, _>("tp")
                    .map(Money::from_decimal),
                cost_price: r
                    .get::<Option<rust_decimal::Decimal>, _>("cost_price")
                    .map(Money::from_decimal),
                is_prescription_only: r.get("is_prescription_only"),
                is_narcotic: r.get("is_narcotic"),
                is_refrigerated: r.get("is_refrigerated"),
                manufacturer: r.get("manufacturer"),
                barcode: r.get("barcode"),
                status: r.get("status"),
            })
            .collect();

        Ok(products)
    }

    pub async fn get_product(
        &self,
        ctx: &TenantContext,
        id: ProductId,
    ) -> Result<ProductDto, CatalogError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, brand_name, generic_name, strength, dosage_form, pack_size, mrp, tp, cost_price, is_prescription_only, is_narcotic, is_refrigerated, manufacturer, barcode, status
             FROM products
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id.0)
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(ProductDto {
                id: ProductId::from(r.get::<Uuid, _>("id")),
                tenant_id: ctx.tenant_id,
                brand_name: r.get("brand_name"),
                generic_name: r.get("generic_name"),
                strength: r.get("strength"),
                dosage_form: r.get("dosage_form"),
                pack_size: r.get("pack_size"),
                mrp: Money::from_decimal(r.get::<rust_decimal::Decimal, _>("mrp")),
                tp: r
                    .get::<Option<rust_decimal::Decimal>, _>("tp")
                    .map(Money::from_decimal),
                cost_price: r
                    .get::<Option<rust_decimal::Decimal>, _>("cost_price")
                    .map(Money::from_decimal),
                is_prescription_only: r.get("is_prescription_only"),
                is_narcotic: r.get("is_narcotic"),
                is_refrigerated: r.get("is_refrigerated"),
                manufacturer: r.get("manufacturer"),
                barcode: r.get("barcode"),
                status: r.get("status"),
            }),
            None => Err(CatalogError::ProductNotFound(id)),
        }
    }

    pub async fn create_product(
        &self,
        ctx: &TenantContext,
        req: CreateProductRequest,
    ) -> Result<ProductDto, CatalogError> {
        ctx.require("product.create")
            .map_err(|e| CatalogError::Unauthorized(e.to_string()))?;

        let product_id = ProductId::new();
        sqlx::query(
            "INSERT INTO products (id, tenant_id, brand_name, generic_name, strength, dosage_form, pack_size, mrp, tp, cost_price, is_prescription_only, is_narcotic, is_refrigerated, manufacturer, barcode, category_id, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 'ACTIVE')"
        )
        .bind(product_id.0)
        .bind(ctx.tenant_id.0)
        .bind(&req.brand_name)
        .bind(&req.generic_name)
        .bind(&req.strength)
        .bind(&req.dosage_form)
        .bind(&req.pack_size)
        .bind(req.mrp.amount())
        .bind(req.tp.map(|m| m.amount()))
        .bind(req.cost_price.map(|m| m.amount()))
        .bind(req.is_prescription_only)
        .bind(req.is_narcotic)
        .bind(req.is_refrigerated)
        .bind(&req.manufacturer)
        .bind(&req.barcode)
        .bind(req.category_id.map(|c| c.0))
        .execute(&self.pool)
        .await?;

        let norm_brand = normalize_query(&req.brand_name);
        sqlx::query(
            "INSERT INTO product_aliases (id, tenant_id, product_id, alias, alias_type, script, weight, source, hit_count)
             VALUES ($1, $2, $3, $4, 'BRAND', 'LATIN', 1.0, 'SEED', 0)
             ON CONFLICT DO NOTHING"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id.0)
        .bind(product_id.0)
        .bind(&norm_brand)
        .execute(&self.pool)
        .await?;

        if let Some(ref gen) = req.generic_name {
            let norm_gen = normalize_query(gen);
            sqlx::query(
                "INSERT INTO product_aliases (id, tenant_id, product_id, alias, alias_type, script, weight, source, hit_count)
                 VALUES ($1, $2, $3, $4, 'GENERIC', 'LATIN', 0.95, 'SEED', 0)
                 ON CONFLICT DO NOTHING"
            )
            .bind(Uuid::now_v7())
            .bind(ctx.tenant_id.0)
            .bind(product_id.0)
            .bind(&norm_gen)
            .execute(&self.pool)
            .await?;
        }

        self.get_product(ctx, product_id).await
    }

    pub async fn bulk_import_csv(
        &self,
        ctx: &TenantContext,
        csv_data: &[u8],
    ) -> Result<usize, CatalogError> {
        ctx.require("product.create")
            .map_err(|e| CatalogError::Unauthorized(e.to_string()))?;

        let mut rdr = csv::Reader::from_reader(csv_data);
        let mut count = 0;

        for result in rdr.records() {
            let record = result?;
            if record.len() < 3 {
                continue;
            }

            let brand_name = record.get(0).unwrap_or("").trim().to_string();
            let generic_name = record
                .get(1)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let mrp_str = record.get(2).unwrap_or("0");
            let mrp_dec: rust_decimal::Decimal =
                mrp_str.parse().unwrap_or(rust_decimal::Decimal::ZERO);

            let is_rx = record
                .get(3)
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false);

            let product_id = ProductId::new();
            sqlx::query(
                "INSERT INTO products (id, tenant_id, brand_name, generic_name, mrp, is_prescription_only, status)
                 VALUES ($1, $2, $3, $4, $5, $6, 'ACTIVE')"
            )
            .bind(product_id.0)
            .bind(ctx.tenant_id.0)
            .bind(&brand_name)
            .bind(&generic_name)
            .bind(mrp_dec)
            .bind(is_rx)
            .execute(&self.pool)
            .await?;

            let norm_brand = normalize_query(&brand_name);
            sqlx::query(
                "INSERT INTO product_aliases (id, tenant_id, product_id, alias, alias_type, script, weight, source, hit_count)
                 VALUES ($1, $2, $3, $4, 'BRAND', 'LATIN', 1.0, 'IMPORT', 0)
                 ON CONFLICT DO NOTHING"
            )
            .bind(Uuid::now_v7())
            .bind(ctx.tenant_id.0)
            .bind(product_id.0)
            .bind(&norm_brand)
            .execute(&self.pool)
            .await?;

            count += 1;
        }

        Ok(count)
    }
}
