-- Migration: 20260821000005_migration_staging.sql
-- Tables for Data Migration Toolkit, Import Batches, Staging, and Rollbacks (Doc 15)

-- 1. Import Batches Table (Doc 15 §8)
CREATE TABLE IF NOT EXISTS import_batches (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    source_kind TEXT NOT NULL,
    mapping_name TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'RUNNING',
    total BIGINT NOT NULL DEFAULT 0,
    inserted BIGINT NOT NULL DEFAULT 0,
    updated BIGINT NOT NULL DEFAULT 0,
    skipped BIGINT NOT NULL DEFAULT 0,
    rejected BIGINT NOT NULL DEFAULT 0,
    dry_run BOOLEAN NOT NULL DEFAULT true,
    initiated_by UUID,
    rollback_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_import_batches_tenant_id ON import_batches(tenant_id);
CREATE INDEX IF NOT EXISTS idx_import_batches_status ON import_batches(status);

ALTER TABLE import_batches ENABLE ROW LEVEL SECURITY;
ALTER TABLE import_batches FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'import_batches' AND policyname = 'import_batches_tenant_isolation'
    ) THEN
        CREATE POLICY import_batches_tenant_isolation ON import_batches
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 2. Import Staging Table (Doc 15 §8)
CREATE TABLE IF NOT EXISTS import_staging (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    batch_id UUID NOT NULL REFERENCES import_batches(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    row_no BIGINT NOT NULL,
    raw JSONB NOT NULL,
    mapped JSONB NOT NULL,
    validation_errors JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'STAGED',
    target_id UUID,
    previous_snapshot JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_import_staging_batch_id ON import_staging(batch_id);
CREATE INDEX IF NOT EXISTS idx_import_staging_tenant_id ON import_staging(tenant_id);
CREATE INDEX IF NOT EXISTS idx_import_staging_status ON import_staging(status);

ALTER TABLE import_staging ENABLE ROW LEVEL SECURITY;
ALTER TABLE import_staging FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'import_staging' AND policyname = 'import_staging_tenant_isolation'
    ) THEN
        CREATE POLICY import_staging_tenant_isolation ON import_staging
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 3. Extend Live Tables with import_batch_id and is_historical flags for rollback traceability
ALTER TABLE products
    ADD COLUMN IF NOT EXISTS import_batch_id UUID REFERENCES import_batches(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_products_import_batch_id ON products(import_batch_id);

ALTER TABLE customers
    ADD COLUMN IF NOT EXISTS import_batch_id UUID REFERENCES import_batches(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_customers_import_batch_id ON customers(import_batch_id);

ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS import_batch_id UUID REFERENCES import_batches(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS is_historical BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS idx_orders_import_batch_id ON orders(import_batch_id);
CREATE INDEX IF NOT EXISTS idx_orders_is_historical ON orders(is_historical);
