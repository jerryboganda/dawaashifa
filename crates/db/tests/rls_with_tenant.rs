use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, TenantId, UserId};
use shifa_db::rls::{with_tenant, DbError};
use sqlx::PgPool;

fn ctx(tenant: TenantId) -> TenantContext {
    TenantContext::from_authenticated_session(
        tenant,
        UserId::new(),
        vec![],
        Default::default(),
        vec![],
    )
}

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

async fn ensure_tenant(pool: &PgPool, tenant: TenantId, name: &str) {
    sqlx::query(
        "INSERT INTO tenants (id, name, legal_name, status) VALUES ($1, $2, $2, 'ACTIVE') ON CONFLICT (id) DO NOTHING",
    )
    .bind(tenant.0)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert tenant");
}

#[tokio::test]
async fn with_tenant_guc_applies_to_queries_inside_closure() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant = TenantId::new();
    let setting = with_tenant(&pool, &ctx(tenant), |conn| {
        Box::pin(async move {
            let row: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
                .fetch_one(&mut *conn)
                .await?;
            Ok(row.0)
        })
    })
    .await
    .expect("with_tenant");
    assert_eq!(setting, tenant.0.to_string());
}

#[tokio::test]
async fn with_tenant_rolls_back_on_error() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant = TenantId::new();
    ensure_tenant(&pool, tenant, "rb-a").await;
    let branch = BranchId::new();
    let err: Result<(), DbError> = with_tenant(&pool, &ctx(tenant), |conn| {
        Box::pin(async move {
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
    with_tenant(&pool, &ctx(tenant), |conn| {
        Box::pin(async move {
            let _: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
                .fetch_one(&mut *conn)
                .await?;
            Ok(())
        })
    })
    .await
    .expect("with_tenant");

    let mut fresh = pool.acquire().await.expect("checkout");
    let leaked: (String,) = sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
        .fetch_one(&mut *fresh)
        .await
        .expect("guc");
    assert!(
        leaked.0.is_empty(),
        "GUC leaked onto next checkout: {:?}",
        leaked.0
    );
}

#[tokio::test]
async fn rls_blocks_cross_tenant_read() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    ensure_tenant(&pool, tenant_a, "A-wt").await;
    ensure_tenant(&pool, tenant_b, "B-wt").await;
    let branch_a = BranchId::new();
    with_tenant(&pool, &ctx(tenant_a), |conn| {
        Box::pin(async move {
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
    })
    .await
    .expect("insert A");

    let seen = with_tenant(&pool, &ctx(tenant_b), |conn| {
        Box::pin(async move {
            let count: (i64,) = sqlx::query_as("SELECT count(*) FROM branches WHERE id = $1")
                .bind(branch_a.0)
                .fetch_one(&mut *conn)
                .await?;
            Ok(count.0)
        })
    })
    .await
    .expect("query B");
    assert_eq!(seen, 0);
}

/// Using the pool inside with_tenant checks out a different connection (no GUC).
#[tokio::test]
async fn with_tenant_pool_capture_does_not_use_txn_guc() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL not set; skipping");
        return;
    };
    shifa_db::run_migrations(&pool).await.expect("migrate");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    ensure_tenant(&pool, tenant_a, "A-probe").await;
    ensure_tenant(&pool, tenant_b, "B-probe").await;
    let branch_a = BranchId::new();
    with_tenant(&pool, &ctx(tenant_a), |conn| {
        Box::pin(async move {
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
    })
    .await
    .expect("insert");

    let pool_for_probe = pool.clone();
    let txn_seen = with_tenant(&pool, &ctx(tenant_b), |conn| {
        Box::pin(async move {
            let on_txn: (i64,) = sqlx::query_as("SELECT count(*) FROM branches WHERE id = $1")
                .bind(branch_a.0)
                .fetch_one(&mut *conn)
                .await?;
            let on_pool: (i64,) = sqlx::query_as("SELECT count(*) FROM branches WHERE id = $1")
                .bind(branch_a.0)
                .fetch_one(&pool_for_probe)
                .await?;
            Ok((on_txn.0, on_pool.0))
        })
    })
    .await
    .expect("probe");

    assert_eq!(txn_seen.0, 0i64, "txn handle must be RLS-scoped to B");
    // Pool checkout has no GUC; table owner/bypass may still see the row.
    let _ = txn_seen.1;
}
