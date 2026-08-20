-- Extend payment_status enum
ALTER TYPE payment_status ADD VALUE IF NOT EXISTS 'AWAITING_PROOF';
ALTER TYPE payment_status ADD VALUE IF NOT EXISTS 'UNDER_REVIEW';
ALTER TYPE payment_status ADD VALUE IF NOT EXISTS 'FAILED';

-- Add ocr_bank to payment_proofs
ALTER TABLE payment_proofs ADD COLUMN IF NOT EXISTS ocr_bank TEXT;

-- Add refund tracking columns to payments
ALTER TABLE payments ADD COLUMN IF NOT EXISTS refund_reason TEXT;
ALTER TABLE payments ADD COLUMN IF NOT EXISTS refunded_at TIMESTAMPTZ;

-- Payment Reconciliations Table (Settlement reports vs Payments ledger)
CREATE TABLE IF NOT EXISTS payment_reconciliations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    report_date DATE NOT NULL,
    gateway payment_gateway_type NOT NULL,
    expected_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    settled_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    fee_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    unmatched_count INT NOT NULL DEFAULT 0,
    discrepancies JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, report_date, gateway)
);

CREATE INDEX IF NOT EXISTS idx_payment_reconciliations_tenant_date ON payment_reconciliations(tenant_id, report_date);

ALTER TABLE payment_reconciliations ENABLE ROW LEVEL SECURITY;
ALTER TABLE payment_reconciliations FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies 
        WHERE tablename = 'payment_reconciliations' AND policyname = 'tenant_isolation_policy'
    ) THEN
        CREATE POLICY tenant_isolation_policy ON payment_reconciliations
            FOR ALL
            USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);
    END IF;
END $$;
