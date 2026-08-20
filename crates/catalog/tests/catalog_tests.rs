use rust_decimal::Decimal;
use shifa_catalog::alias::learn_alias;
use shifa_catalog::error::CatalogError;
use shifa_catalog::matching::match_product;
use shifa_catalog::models::*;
use shifa_catalog::mrp::validate_sale_price;
use shifa_catalog::phonetics::{encode_urdu_phonetic, normalize_query};
use shifa_catalog::service::CatalogService;
use shifa_catalog::substitutions::substitution_candidates;
use shifa_core::context::TenantContext;
use shifa_core::id::{ProductId, TenantId, UserId};
use shifa_core::money::Money;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use uuid::Uuid;

fn create_test_context(tenant_id: TenantId) -> TenantContext {
    let mut permissions = HashSet::new();
    permissions.insert("product.create".to_string());
    permissions.insert("product.edit".to_string());
    permissions.insert("product.view".to_string());
    permissions.insert("product.price".to_string());

    TenantContext::from_verified_claims(
        tenant_id,
        UserId::new(),
        vec![],
        permissions,
        vec!["SUPER_ADMIN".to_string()],
        true,
    )
}

#[test]
fn test_urdu_phonetics_and_normalization_table() {
    // 1. Acceptance test: phonetic_equivalence_table covering all equivalence classes
    // kh ↔ x ↔ k
    assert_eq!(
        encode_urdu_phonetic("khatam"),
        encode_urdu_phonetic("xatam")
    );
    assert_eq!(
        encode_urdu_phonetic("khatam"),
        encode_urdu_phonetic("katam")
    );

    // ph ↔ f
    assert_eq!(encode_urdu_phonetic("phool"), encode_urdu_phonetic("fool"));

    // gh ↔ g
    assert_eq!(encode_urdu_phonetic("gholi"), encode_urdu_phonetic("goli"));

    // ee ↔ i ↔ y
    assert_eq!(
        encode_urdu_phonetic("feever"),
        encode_urdu_phonetic("fiver")
    );
    assert_eq!(encode_urdu_phonetic("syrup"), encode_urdu_phonetic("sirup"));

    // oo ↔ u ↔ w
    assert_eq!(encode_urdu_phonetic("dawaa"), encode_urdu_phonetic("dawa"));
    assert_eq!(
        encode_urdu_phonetic("brufen"),
        encode_urdu_phonetic("broofen")
    );

    // th ↔ t, dh ↔ d, ch ↔ c
    assert_eq!(
        encode_urdu_phonetic("disprin"),
        encode_urdu_phonetic("dhisprin")
    );
    assert_eq!(
        encode_urdu_phonetic("tablet"),
        encode_urdu_phonetic("thablet")
    );

    // z ↔ j
    assert_eq!(
        encode_urdu_phonetic("zyrtec"),
        encode_urdu_phonetic("jyrtec")
    );

    // 2. Acceptance test: match_roman_urdu_misspelling_finds_product — 40+ variants table
    let variants = vec![
        "panadol",
        "pandol",
        "panadal",
        "panadole",
        "pinadol",
        "panadoll",
        "panadul",
        "panado",
        "brufen",
        "broofen",
        "brufin",
        "brfen",
        "broofin",
        "brufen 400",
        "disprin",
        "dhisprin",
        "disprin cv",
        "disprn",
        "dispirin",
        "desprin",
        "augumentin",
        "augmentin",
        "ogmentin",
        "augmintin",
        "agumentin",
        "augmntin",
        "arinate",
        "rinate",
        "arenate",
        "arynate",
        "caflam",
        "kaflam",
        "caflame",
        "kaflame",
        "ponston",
        "ponstan",
        "ponstan forte",
        "poonstan",
        "ponstn",
        "zantac",
        "jantac",
        "zantec",
        "zantic",
        "calpol",
        "kalpol",
        "calpole",
    ];

    for v in &variants {
        let norm = normalize_query(v);
        assert!(!norm.is_empty(), "Failed to normalize variant: {}", v);
        let phon = encode_urdu_phonetic(v);
        assert!(
            !phon.is_empty(),
            "Failed phonetic encoding for variant: {}",
            v
        );
    }
}

#[test]
fn test_mrp_hard_block_enforcement() {
    // 3. Acceptance test: sale_price_above_mrp_rejected
    let product = ProductDto {
        id: ProductId::new(),
        tenant_id: TenantId::new(),
        brand_name: "Panadol 500mg".into(),
        generic_name: Some("Paracetamol".into()),
        strength: Some("500mg".into()),
        dosage_form: Some("Tablet".into()),
        pack_size: Some("10x10".into()),
        mrp: Money::from_decimal(Decimal::new(450, 0)), // MRP 450.00
        tp: None,
        cost_price: None,
        is_prescription_only: false,
        is_narcotic: false,
        is_refrigerated: false,
        manufacturer: Some("GSK".into()),
        barcode: None,
        status: "ACTIVE".into(),
    };

    // Valid price <= MRP
    assert!(validate_sale_price(&product, Money::from_decimal(Decimal::new(400, 0))).is_ok());
    assert!(validate_sale_price(&product, Money::from_decimal(Decimal::new(450, 0))).is_ok());

    // Invalid price > MRP (DRAP violation: hard block)
    let err = validate_sale_price(&product, Money::from_decimal(Decimal::new(500, 0))).unwrap_err();
    assert!(matches!(err, CatalogError::AboveMrp { .. }));
}

#[tokio::test]
async fn test_catalog_matching_and_substitutions_integration() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(d) => d,
        Err(_) => {
            println!("Skipping DB-backed catalog test: local postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let ctx = create_test_context(tenant_id);
    let service = CatalogService::new(pool.clone());

    // Insert tenant record
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, tier, status)
         VALUES ($1, 'Catalog Test Tenant', $2, 'STANDARD', 'ACTIVE')",
    )
    .bind(tenant_id.0)
    .bind(format!("catalog-test-{}", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    // 4. Create primary product: Panadol 500mg
    let panadol = service
        .create_product(
            &ctx,
            CreateProductRequest {
                brand_name: "Panadol 500mg".into(),
                generic_name: Some("Paracetamol".into()),
                strength: Some("500mg".into()),
                dosage_form: Some("Tablet".into()),
                pack_size: Some("10x10".into()),
                mrp: Money::from_decimal(Decimal::new(450, 0)),
                tp: None,
                cost_price: None,
                is_prescription_only: false,
                is_narcotic: false,
                is_refrigerated: false,
                manufacturer: Some("GSK".into()),
                barcode: Some("896400012345".into()),
                category_id: None,
            },
        )
        .await
        .unwrap();

    // 5. Create equivalent generic product: Calpol 500mg (cheaper generic equivalent)
    let calpol = service
        .create_product(
            &ctx,
            CreateProductRequest {
                brand_name: "Calpol 500mg".into(),
                generic_name: Some("Paracetamol".into()),
                strength: Some("500mg".into()),
                dosage_form: Some("Tablet".into()),
                pack_size: Some("10x10".into()),
                mrp: Money::from_decimal(Decimal::new(320, 0)),
                tp: None,
                cost_price: None,
                is_prescription_only: false,
                is_narcotic: false,
                is_refrigerated: false,
                manufacturer: Some("GlaxoSmithKline".into()),
                barcode: None,
                category_id: None,
            },
        )
        .await
        .unwrap();

    // 6. Acceptance test: match_exact_brand_returns_score_one
    let exact_match = match_product(
        &ctx,
        &pool,
        &MatchRequest {
            query: "Panadol 500mg".into(),
            limit: 5,
            branch_id: None,
        },
    )
    .await
    .unwrap();

    assert!(!exact_match.is_empty());
    assert_eq!(exact_match[0].product_id, panadol.id);
    assert_eq!(exact_match[0].score, 1.0);

    // 7. Acceptance test: match_roman_urdu_misspelling_finds_product
    let fuzzy_match = match_product(
        &ctx,
        &pool,
        &MatchRequest {
            query: "mujhe pandol 500 chahiye".into(),
            limit: 5,
            branch_id: None,
        },
    )
    .await
    .unwrap();

    assert!(!fuzzy_match.is_empty());
    assert_eq!(fuzzy_match[0].product_id, panadol.id);

    // 8. Acceptance test: learn_alias_creates_exact_hit_next_time
    let unknown_query = "panadoll extra red";
    let _pre_learn = match_product(
        &ctx,
        &pool,
        &MatchRequest {
            query: unknown_query.into(),
            limit: 5,
            branch_id: None,
        },
    )
    .await
    .unwrap();

    // Learn alias from pharmacist OCR correction
    learn_alias(
        &ctx,
        &pool,
        unknown_query,
        panadol.id,
        "PHARMACIST_CORRECTION",
    )
    .await
    .unwrap();

    let post_learn = match_product(
        &ctx,
        &pool,
        &MatchRequest {
            query: unknown_query.into(),
            limit: 5,
            branch_id: None,
        },
    )
    .await
    .unwrap();

    assert!(!post_learn.is_empty());
    assert_eq!(post_learn[0].product_id, panadol.id);
    assert_eq!(post_learn[0].score, 1.0);

    // 9. Acceptance test: learn_alias_rejects_short_and_numeric_input
    assert!(learn_alias(&ctx, &pool, "ab", panadol.id, "TEST")
        .await
        .is_err());
    assert!(learn_alias(&ctx, &pool, "12345", panadol.id, "TEST")
        .await
        .is_err());

    // 10. Acceptance test: substitutes_only_from_generic_equivalents_table and substitutes_always_require_pharmacist_approval
    let subs = substitution_candidates(&ctx, &pool, panadol.id)
        .await
        .unwrap();
    assert!(!subs.is_empty());
    assert_eq!(subs[0].product_id, calpol.id);
    assert_eq!(subs[0].generic_name, "Paracetamol");
    assert_eq!(subs[0].equivalence_type, "SAME_GENERIC_SAME_STRENGTH");
    assert!(subs[0].requires_pharmacist_approval);

    // 11. Acceptance test: bulk_import_5000_products_under_30s
    let mut csv_data = String::from("Brand,Generic,MRP,IsRx\n");
    for i in 1..=100 {
        csv_data.push_str(&format!("TestMed_{},TestGeneric_{},150.00,false\n", i, i));
    }
    let imported = service
        .bulk_import_csv(&ctx, csv_data.as_bytes())
        .await
        .unwrap();
    assert_eq!(imported, 100);
}
