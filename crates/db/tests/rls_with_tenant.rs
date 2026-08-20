use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, TenantId, UserId};
use shifa_db::rls::{with_tenant, DbError};
use sqlx::PgPool;

fn ctx(tenant: TenantId) -> TenantContext {
    TenantContext::from_verified_claims(
        tenant,
        UserId::new(),
        vec![],
        Default::default(),
        vec![],
        false,
    )
}

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn with_tenant_guc_applies_to_queries_inside_closure() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant = TenantId::new();
    let setting = with_tenant(&pool, &ctx(tenant), |conn| async move {
        let row: (Option<String>,) =
            sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
                .fetch_one(&mut *conn)
                .await?;
        Ok(row.0)
    })
    .await
    .expect("with_tenant");
    assert_eq!(setting.as_deref(), Some(tenant.0.to_string().as_str()));
}

#[tokio::test]
async fn with_tenant_rolls_back_on_error() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant = TenantId::new();
    sqlx::query(
        "INSERT INTO tenants (id, name, legal_name, status) VALUES ($1, 'rb-a', 'rb-a', 'ACTIVE') ON CONFLICT DO NOTHING",
    )
    .bind(tenant.0)
    .execute(&pool)
    .await
    .expect("insert tenant");

    let branch = BranchId::new();
    let err = with_tenant(&pool, &ctx(tenant), |conn| async move {
        sqlx::query(
            "INSERT INTO branches (id, tenant_id, name, code, drap_licence_no, pharmacist_in_charge, address, city, geo)
             VALUES ($1, $2, 'RB', 'RB-X', 'DRAP', 'P', 'A', 'Karachi', ST_SetSRID(ST_MakePoint(67.0, 24.8), 4326)::geography)",
        )
        .bind(branch.0)
        .bind(tenant.0)
        .execute(&mut *conn)
        .await?;
        Err(DbError::Sqlx(sqlx::Error::Protocol("forced".into())))
    })
    .await;
    assert!(err.is_err());

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM branches WHERE id = $1")
        .bind(branch.0)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 0, "error path must roll back the insert");
}

#[tokio::test]
async fn guc_does_not_leak_to_next_pool_checkout() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant = TenantId::new();
    with_tenant(&pool, &ctx(tenant), |conn| async move {
        let _: (Option<String>,) =
            sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
                .fetch_one(&mut *conn)
                .await?;
        Ok(())
    })
    .await
    .expect("with_tenant");

    let mut fresh = pool.acquire().await.expect("checkout");
    let leaked: (Option<String>,) =
        sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
            .fetch_one(&mut *fresh)
            .await
            .expect("guc");
    assert!(
        leaked.0.is_none() || leaked.0.as_deref() == Some(""),
        "GUC leaked: {:?}",
        leaked.0
    );
}

#[tokio::test]
async fn rls_blocks_cross_tenant_read_via_with_tenant() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    for (t, name) in [(tenant_a, "A-wt"), (tenant_b, "B-wt")] {
        sqlx::query(
            "INSERT INTO tenants (id, name, legal_name, status) VALUES ($1, $2, $2, 'ACTIVE')",
        )
        .bind(t.0)
        .bind(name)
        .execute(&pool)
        .await
        .expect("tenant");
    }
    let branch_a = BranchId::new();
    with_tenant(&pool, &ctx(tenant_a), |conn| async move {
        sqlx::query(
            "INSERT INTO branches (id, tenant_id, name, code, drap_licence_no, pharmacist_in_charge, address, city, geo)
             VALUES ($1, $2, 'A', 'WT-A', 'DRAP', 'P', 'A', 'Karachi', ST_SetSRID(ST_MakePoint(67.0, 24.8), 4326)::geography)",
        )
        .bind(branch_a.0)
        .bind(tenant_a.0)
        .execute(&mut *conn)
        .await?;
        Ok(())
    })
    .await
    .expect("insert A");

    let seen = with_tenant(&pool, &ctx(tenant_b), |conn| async move {
        let count: (i64,) = sqlx::query_as("SELECT count(*) FROM branches WHERE id = $1")
            .bind(branch_a.0)
            .fetch_one(&mut *conn)
            .await?;
        Ok(count.0)
    })
    .await
    .expect("query B");
    assert_eq!(seen, 0);
}

/// Adversarial probe B1/E3: using the pool inside with_tenant bypasses the GUC.
#[tokio::test]
async fn with_tenant_pool_capture_bypasses_rls_guc() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    for (t, name) in [(tenant_a, "A-probe"), (tenant_b, "B-probe")] {
        sqlx::query(
            "INSERT INTO tenants (id, name, legal_name, status) VALUES ($1, $2, $2, 'ACTIVE')",
        )
        .bind(t.0)
        .bind(name)
        .execute(&pool)
        .await
        .expect("tenant");
    }
    let branch_a = BranchId::new();
    with_tenant(&pool, &ctx(tenant_a), |conn| async move {
        sqlx::query(
            "INSERT INTO branches (id, tenant_id, name, code, drap_licence_no, pharmacist_in_charge, address, city, geo)
             VALUES ($1, $2, 'A', 'PR-A', 'DRAP', 'P', 'A', 'Karachi', ST_SetSRID(ST_MakePoint(67.0, 24.8), 4326)::geography)",
        )
        .bind(branch_a.0)
        .bind(tenant_a.0)
        .execute(&mut *conn)
        .await?;
        Ok(())
    })
    .await
    .expect("insert");

    let pool_seen = with_tenant(&pool, &ctx(tenant_b), |conn| {
        let pool = pool.clone();
        async move {
            let _guc: (Option<String>,) =
                sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
                    .fetch_one(&mut *conn)
                    .await?;
            let count: (i64,) = sqlx::query_as("SELECT count(*) FROM branches WHERE id = $1")
                .bind(branch_a.0)
                .fetch_one(&pool)
                .await?;
            Ok(count.0)
        }
    })
    .await
    .expect("probe");

    // If this is > 0, RLS GUC does not apply to pool checkouts inside the closure.
    eprintln!("adversarial pool-inside-with_tenant row count: {pool_seen}");
}
