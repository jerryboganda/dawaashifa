use crate::rls::DbError;
use rust_decimal::Decimal;
use shifa_core::id::*;
use sqlx::PgPool;

/// Summary statistics of seeded database entities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedStats {
    pub tenants_count: usize,
    pub branches_count: usize,
    pub users_count: usize,
    pub products_count: usize,
}

/// Generates realistic seed data: 1 tenant, 8 branches, 50 users, 5,000 products.
pub async fn seed_database(pool: &PgPool) -> Result<SeedStats, DbError> {
    let tenant_id = TenantId::new();

    // 1. Insert Tenant
    sqlx::query(
        "INSERT INTO tenants (id, name, legal_name, ntn, strn, status) VALUES ($1, $2, $3, $4, $5, 'ACTIVE'::tenant_status)"
    )
    .bind(tenant_id.0)
    .bind("Shifa Healthcare Network")
    .bind("Shifa Healthcare (Pvt) Ltd")
    .bind("1234567-8")
    .bind("987654321")
    .execute(pool)
    .await?;

    // 2. Insert 8 Branches
    let branch_names = [
        ("Clifton Hub", "KHI-01", "Karachi", 24.8138, 67.0299, true, true),
        ("Gulshan Branch", "KHI-02", "Karachi", 24.9207, 67.0982, false, false),
        ("DHA Phase 5", "LHR-01", "Lahore", 31.4697, 74.3986, true, true),
        ("Gulberg Branch", "LHR-02", "Lahore", 31.5204, 74.3587, false, false),
        ("Blue Area Central", "ISB-01", "Islamabad", 33.7182, 73.0605, true, true),
        ("Saddar Branch", "RWP-01", "Rawalpindi", 33.5973, 73.0538, false, false),
        ("D-Ground Branch", "FSD-01", "Faisalabad", 31.4116, 73.0965, false, false),
        ("Cantt Branch", "MUL-01", "Multan", 30.1984, 71.4687, false, false),
    ];

    for (name, code, city, lat, lon, is_hub, cold_chain) in branch_names {
        let branch_id = BranchId::new();
        sqlx::query(
            "INSERT INTO branches (id, tenant_id, name, code, drap_licence_no, pharmacist_in_charge, address, city, geo, is_hub, cold_chain_capable)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, ST_SetSRID(ST_MakePoint($9, $10), 4326)::geography, $11, $12)"
        )
        .bind(branch_id.0)
        .bind(tenant_id.0)
        .bind(name)
        .bind(code)
        .bind("DRAP/LIC/2026/001")
        .bind("Dr. Farooq Ahmed, RPh")
        .bind(format!("Main Blvd, {}", city))
        .bind(city)
        .bind(lon)
        .bind(lat)
        .bind(is_hub)
        .bind(cold_chain)
        .execute(pool)
        .await?;
    }

    // 3. Insert 50 Users
    for i in 1..=50 {
        let user_id = UserId::new();
        let phone = format!("+92300{:07}", i);
        let email = format!("staff{}@shifa.pk", i);
        let name = format!("Staff Member {}", i);

        sqlx::query(
            "INSERT INTO users (id, tenant_id, phone, email, full_name, password_hash, status, locale)
             VALUES ($1, $2, $3, $4, $5, $6, 'ACTIVE'::user_status, 'en')"
        )
        .bind(user_id.0)
        .bind(tenant_id.0)
        .bind(phone)
        .bind(email)
        .bind(name)
        .bind("$argon2id$v=19$m=19456,t=2,p=1$dummy_hash_for_testing")
        .execute(pool)
        .await?;
    }

    // 4. Insert 5,000 Products in batches
    let forms = ["Tablet", "Capsule", "Syrup", "Injection", "Ointment", "Drops", "Inhaler"];
    let manufacturers = ["Getz Pharma", "GSK Pakistan", "Abbott Laboratories", "Sami Pharmaceuticals", "Searle Company", "Hilton Pharma"];

    for chunk_start in (1..=5000).step_by(500) {
        let chunk_end = (chunk_start + 499).min(5000);
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO products (id, tenant_id, sku, name_en, form, strength, pack_size, manufacturer, drap_registration_no, is_prescription_only, is_controlled, requires_cold_chain, mrp, status) "
        );

        query_builder.push_values(chunk_start..=chunk_end, |mut b, i| {
            let pid = ProductId::new();
            let sku = format!("SKU-{:05}", i);
            let form = forms[i % forms.len()];
            let mfg = manufacturers[i % manufacturers.len()];
            let name = format!("Medicament-{} {}mg", i, (i % 10 + 1) * 50);
            let is_rx = i % 3 == 0;
            let cold = i % 20 == 0;
            let mrp_val = Decimal::from((i % 500 + 50) * 10);

            b.push_bind(pid.0)
             .push_bind(tenant_id.0)
             .push_bind(sku)
             .push_bind(name)
             .push_bind(form)
             .push_bind(format!("{}mg", (i % 10 + 1) * 50))
             .push_bind("Pack of 20")
             .push_bind(mfg)
             .push_bind(format!("DRAP-{:06}", i))
             .push_bind(is_rx)
             .push_bind(false)
             .push_bind(cold)
             .push_bind(mrp_val)
             .push_bind("ACTIVE");
        });

        let query = query_builder.build();
        query.execute(pool).await?;
    }

    Ok(SeedStats {
        tenants_count: 1,
        branches_count: 8,
        users_count: 50,
        products_count: 5000,
    })
}
