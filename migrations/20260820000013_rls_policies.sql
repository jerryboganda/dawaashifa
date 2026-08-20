-- Row-Level Security Policies (Invariant I-2: Enabled on every tenant-scoped table)

DO $$
DECLARE
    t text;
    tenant_tables text[] := ARRAY[
        'branches',
        'users',
        'roles',
        'permissions',
        'role_permissions',
        'user_roles',
        'user_branches',
        'sessions',
        'customers',
        'customer_addresses',
        'product_categories',
        'generics',
        'generic_equivalents',
        'products',
        'product_generics',
        'product_aliases',
        'suppliers',
        'batches',
        'stock_movements',
        'stock_current',
        'cold_chain_logs',
        'business_identities',
        'channels',
        'conversations',
        'messages',
        'prescriptions',
        'rx_ocr_results',
        'rx_lines',
        'pharmacist_approvals',
        'orders',
        'order_items',
        'order_events',
        'payments',
        'payment_proofs',
        'transaction_id_ledger',
        'riders',
        'deliveries',
        'rider_cash_sessions',
        'tax_categories',
        'invoices',
        'audit_log'
    ];
BEGIN
    FOREACH t IN ARRAY tenant_tables
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY;', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY;', t);
        EXECUTE format('DROP POLICY IF EXISTS %I_tenant_isolation ON %I;', t, t);
        EXECUTE format('CREATE POLICY %I_tenant_isolation ON %I FOR ALL USING (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid) WITH CHECK (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid);', t, t);
    END LOOP;
END;
$$;
