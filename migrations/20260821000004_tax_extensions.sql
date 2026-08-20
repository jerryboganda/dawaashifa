-- Migration: 20260821000004_tax_extensions.sql
-- Extensions for Tax Categories, Gapless Invoice Sequences, FBR Queue, and Credit Notes (Doc 13)

-- 1. Extend tax_categories table
ALTER TABLE tax_categories
    ADD COLUMN IF NOT EXISTS is_zero_rated BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS effective_from TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS effective_to TIMESTAMPTZ;

-- 2. Extend invoices table
ALTER TABLE invoices
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'ISSUED',
    ADD COLUMN IF NOT EXISTS subtotal NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    ADD COLUMN IF NOT EXISTS tax_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    ADD COLUMN IF NOT EXISTS total_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    ADD COLUMN IF NOT EXISTS lines JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS is_provisional BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS fbr_submitted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS fbr_error TEXT,
    ADD COLUMN IF NOT EXISTS credit_note_for UUID REFERENCES invoices(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS credit_note_reason TEXT,
    ADD COLUMN IF NOT EXISTS fbr_queue_status TEXT NOT NULL DEFAULT 'PENDING';

CREATE INDEX IF NOT EXISTS idx_invoices_fbr_queue_status ON invoices(fbr_queue_status);
CREATE INDEX IF NOT EXISTS idx_invoices_credit_note_for ON invoices(credit_note_for);

-- 3. Invoice Sequences table for gapless numbering per branch and fiscal year (Doc 13 §6)
CREATE TABLE IF NOT EXISTS invoice_sequences (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    fiscal_year TEXT NOT NULL,
    last_seq BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (tenant_id, branch_id, fiscal_year)
);

CREATE INDEX IF NOT EXISTS idx_invoice_sequences_tenant_id ON invoice_sequences(tenant_id);

-- Enable RLS on invoice_sequences (Invariant I-1, I-2)
ALTER TABLE invoice_sequences ENABLE ROW LEVEL SECURITY;
ALTER TABLE invoice_sequences FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'invoice_sequences' AND policyname = 'invoice_sequences_tenant_isolation'
    ) THEN
        CREATE POLICY invoice_sequences_tenant_isolation ON invoice_sequences
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;
