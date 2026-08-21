-- Migration 20260821000008: Standardize RLS policies across all tenant tables (Invariant I-1, I-2)

DO $$
DECLARE
    t text;
    all_tables text[] := ARRAY[
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
        'audit_log',
        'controlled_dispensing_register',
        'rx_substitutions',
        'payment_reconciliations',
        'picking_lists',
        'invoice_sequences',
        'migration_batches',
        'migration_staging_rows',
        'wa_sessions',
        'business_accounts',
        'price_contracts',
        'consignment_inventory',
        'b2b_invoices',
        'device_traceability_ledger'
    ];
BEGIN
    FOREACH t IN ARRAY all_tables
    LOOP
        -- Ensure RLS is active and forced on table if table exists
        IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = t) THEN
            EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY;', t);
            EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY;', t);
            EXECUTE format('DROP POLICY IF EXISTS %I_tenant_isolation ON %I;', t, t);
            EXECUTE format('DROP POLICY IF EXISTS tenant_isolation_policy ON %I;', t);
            EXECUTE format('CREATE POLICY %I_tenant_isolation ON %I FOR ALL USING (tenant_id = COALESCE(NULLIF(current_setting(''app.tenant_id'', true), ''''), NULLIF(current_setting(''app.current_tenant_id'', true), ''''))::uuid) WITH CHECK (tenant_id = COALESCE(NULLIF(current_setting(''app.tenant_id'', true), ''''), NULLIF(current_setting(''app.current_tenant_id'', true), ''''))::uuid);', t, t);
        END IF;
    END LOOP;
END;
$$;
