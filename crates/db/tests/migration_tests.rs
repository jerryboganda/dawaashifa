use shifa_core::id::*;
use shifa_db::rls::set_tenant_context;
use shifa_db::seed::seed_database;
use sqlx::PgPool;

#[tokio::test]
async fn test_database_migrations_and_rls_suite() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("DATABASE_URL not set; skipping live Postgres integration test");
            return;
        }
    };

    let pool = PgPool::connect(&database_url)
        .await
        .expect("connect to database");

    // 1. Run migrations
    shifa_db::run_migrations(&pool)
        .await
        .expect("run embedded migrations");

    // 2. Test money_column_types: assert no money column is float4 or float8
    let float_money_cols: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT table_name, column_name, data_type 
         FROM information_schema.columns 
         WHERE table_schema = 'public' 
           AND (column_name LIKE '%mrp%' OR column_name LIKE '%price%' OR column_name LIKE '%amount%' OR column_name LIKE '%total%' OR column_name LIKE '%fee%')
           AND data_type IN ('real', 'double precision')"
    )
    .fetch_all(&pool)
    .await
    .expect("query money columns");

    assert!(
        float_money_cols.is_empty(),
        "Detected float/double money columns: {:?}",
        float_money_cols
    );

    // 3. Test every_tenant_table_has_rls
    let tables_missing_rls: Vec<(String,)> = sqlx::query_as(
        "SELECT c.relname::text 
         FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         JOIN pg_attribute a ON a.attrelid = c.oid
         WHERE n.nspname = 'public' 
           AND c.relkind = 'r'
           AND a.attname = 'tenant_id'
           AND c.relrowsecurity = false
           AND c.relname NOT LIKE 'pg_%'",
    )
    .fetch_all(&pool)
    .await
    .expect("query tables with tenant_id missing RLS");

    assert!(
        tables_missing_rls.is_empty(),
        "Tables with tenant_id missing RLS: {:?}",
        tables_missing_rls
    );

    // 4. Test every_fk_has_index
    let unindexed_fks: Vec<(String, String)> = sqlx::query_as(
        "SELECT conrelid::regclass::text AS table_name, conname::text AS constraint_name
         FROM pg_constraint c
         WHERE c.contype = 'f'
           AND c.connamespace = 'public'::regnamespace
           AND NOT EXISTS (
             SELECT 1 FROM pg_index i
             WHERE i.indrelid = c.conrelid
               AND i.indkey[0] = c.conkey[1]
           )",
    )
    .fetch_all(&pool)
    .await
    .expect("query unindexed foreign keys");

    assert!(
        unindexed_fks.is_empty(),
        "Detected unindexed foreign keys: {:?}",
        unindexed_fks
    );

    // 5. Test rls_blocks_cross_tenant_read
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    // Insert tenants
    sqlx::query("INSERT INTO tenants (id, name, legal_name, status) VALUES ($1, 'Tenant A', 'Tenant A Ltd', 'ACTIVE')")
        .bind(tenant_a.0)
        .execute(&pool)
        .await
        .expect("insert tenant a");

    sqlx::query("INSERT INTO tenants (id, name, legal_name, status) VALUES ($1, 'Tenant B', 'Tenant B Ltd', 'ACTIVE')")
        .bind(tenant_b.0)
        .execute(&pool)
        .await
        .expect("insert tenant b");

    // Insert branch for tenant A
    let branch_a = BranchId::new();
    let mut tx_a = pool.begin().await.expect("begin tx_a");
    set_tenant_context(&mut tx_a, tenant_a)
        .await
        .expect("set tenant a context");

    sqlx::query(
        "INSERT INTO branches (id, tenant_id, name, code, drap_licence_no, pharmacist_in_charge, address, city, geo)
         VALUES ($1, $2, 'Branch A', 'BR-A', 'DRAP-01', 'Pharmacist', 'Address', 'Karachi', ST_SetSRID(ST_MakePoint(67.0, 24.8), 4326)::geography)"
    )
    .bind(branch_a.0)
    .bind(tenant_a.0)
    .execute(&mut *tx_a)
    .await
    .expect("insert branch a");
    tx_a.commit().await.expect("commit tx_a");

    // Query from tenant B context
    let mut tx_b = pool.begin().await.expect("begin tx_b");
    set_tenant_context(&mut tx_b, tenant_b)
        .await
        .expect("set tenant b context");

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM branches WHERE id = $1")
        .bind(branch_a.0)
        .fetch_one(&mut *tx_b)
        .await
        .expect("query branch under tenant B context");

    assert_eq!(
        count.0, 0,
        "Tenant B must not see Tenant A branch under RLS"
    );
    tx_b.commit().await.expect("commit tx_b");

    // 6. Test seed_generator_runs
    let stats = seed_database(&pool).await.expect("seed generator run");
    assert_eq!(stats.tenants_count, 1);
    assert_eq!(stats.branches_count, 8);
    assert_eq!(stats.users_count, 50);
    assert_eq!(stats.products_count, 5000);
}
