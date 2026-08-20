-- Migration 20260821000001: Prescription extensions for Doc 09

-- 1. Alter prescription_status enum to include all workflow statuses
ALTER TYPE prescription_status ADD VALUE IF NOT EXISTS 'RECEIVED';
ALTER TYPE prescription_status ADD VALUE IF NOT EXISTS 'PREPROCESSING';
ALTER TYPE prescription_status ADD VALUE IF NOT EXISTS 'EXTRACTING';
ALTER TYPE prescription_status ADD VALUE IF NOT EXISTS 'PENDING_REVIEW';
ALTER TYPE prescription_status ADD VALUE IF NOT EXISTS 'PARTIALLY_APPROVED';
ALTER TYPE prescription_status ADD VALUE IF NOT EXISTS 'NEEDS_CLARIFICATION';

-- 2. Add assigned_to and branch_id to prescriptions
ALTER TABLE prescriptions ADD COLUMN IF NOT EXISTS branch_id UUID REFERENCES branches(id) ON DELETE SET NULL;
ALTER TABLE prescriptions ADD COLUMN IF NOT EXISTS assigned_to UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE prescriptions ADD COLUMN IF NOT EXISTS preprocessed_image_key TEXT;
ALTER TABLE prescriptions ADD COLUMN IF NOT EXISTS patient_name TEXT;
ALTER TABLE prescriptions ADD COLUMN IF NOT EXISTS clarification_notes TEXT;

CREATE INDEX IF NOT EXISTS idx_prescriptions_branch_id ON prescriptions(branch_id);
CREATE INDEX IF NOT EXISTS idx_prescriptions_assigned_to ON prescriptions(assigned_to);

-- 3. Controlled dispensing register (Doc 09 §10)
CREATE TABLE IF NOT EXISTS controlled_dispensing_register (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    prescription_id UUID NOT NULL REFERENCES prescriptions(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    pharmacist_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    quantity INTEGER NOT NULL,
    prescriber_name TEXT,
    prescriber_pmdc_no TEXT,
    patient_name TEXT,
    patient_phone TEXT,
    dispensed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_controlled_disp_tenant_id ON controlled_dispensing_register(tenant_id);
CREATE INDEX IF NOT EXISTS idx_controlled_disp_rx_id ON controlled_dispensing_register(prescription_id);
CREATE INDEX IF NOT EXISTS idx_controlled_disp_product_id ON controlled_dispensing_register(product_id);
CREATE INDEX IF NOT EXISTS idx_controlled_disp_pharmacist_id ON controlled_dispensing_register(pharmacist_id);

-- 4. Prescription substitutions tracking (Doc 09 §9)
CREATE TABLE IF NOT EXISTS rx_substitutions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    prescription_id UUID NOT NULL REFERENCES prescriptions(id) ON DELETE CASCADE,
    rx_line_id UUID REFERENCES rx_lines(id) ON DELETE CASCADE,
    original_product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    substituted_product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    pharmacist_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    reason TEXT NOT NULL,
    customer_informed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_rx_substitutions_tenant_id ON rx_substitutions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_rx_substitutions_rx_id ON rx_substitutions(prescription_id);

-- 5. Enable RLS on new tables (Invariant I-2)
ALTER TABLE controlled_dispensing_register ENABLE ROW LEVEL SECURITY;
ALTER TABLE controlled_dispensing_register FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS controlled_dispensing_register_tenant_isolation ON controlled_dispensing_register;
CREATE POLICY controlled_dispensing_register_tenant_isolation ON controlled_dispensing_register FOR ALL
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

ALTER TABLE rx_substitutions ENABLE ROW LEVEL SECURITY;
ALTER TABLE rx_substitutions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rx_substitutions_tenant_isolation ON rx_substitutions;
CREATE POLICY rx_substitutions_tenant_isolation ON rx_substitutions FOR ALL
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
