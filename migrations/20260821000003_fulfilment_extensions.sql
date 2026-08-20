-- Migration: 20260821000003_fulfilment_extensions.sql
-- Extensions for Fulfilment, Rider Management, POD, and Cash Reconciliation (Doc 12)

-- 1. Extend delivery_status enum if needed
ALTER TYPE delivery_status ADD VALUE IF NOT EXISTS 'UNASSIGNED';
ALTER TYPE delivery_status ADD VALUE IF NOT EXISTS 'ACCEPTED';
ALTER TYPE delivery_status ADD VALUE IF NOT EXISTS 'IN_TRANSIT';

-- 2. Extend deliveries table
ALTER TABLE deliveries
    ADD COLUMN IF NOT EXISTS branch_id UUID REFERENCES branches(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS recipient_name TEXT,
    ADD COLUMN IF NOT EXISTS recipient_cnic_last4 TEXT,
    ADD COLUMN IF NOT EXISTS prescription_collected BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS reattempt_count INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS tracking_token TEXT NOT NULL DEFAULT encode(gen_random_bytes(16), 'hex'),
    ADD COLUMN IF NOT EXISTS gps_denied_flag BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS idempotency_key TEXT,
    ADD COLUMN IF NOT EXISTS accepted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS in_transit_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS decline_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_deliveries_branch_id ON deliveries(branch_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_deliveries_tracking_token ON deliveries(tracking_token);
CREATE INDEX IF NOT EXISTS idx_deliveries_idempotency ON deliveries(tenant_id, idempotency_key);

-- 3. Extend riders table
ALTER TABLE riders
    ADD COLUMN IF NOT EXISTS decline_count INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS on_shift BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS shift_started_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS shift_ended_at TIMESTAMPTZ;

-- 4. Extend rider_cash_sessions table
ALTER TABLE rider_cash_sessions
    ADD COLUMN IF NOT EXISTS branch_id UUID REFERENCES branches(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'OPEN';

CREATE INDEX IF NOT EXISTS idx_rider_cash_sessions_branch_id ON rider_cash_sessions(branch_id);

-- 5. Picking lists table
CREATE TABLE IF NOT EXISTS picking_lists (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'PENDING',
    items JSONB NOT NULL DEFAULT '[]'::jsonb,
    picked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_picking_lists_tenant_id ON picking_lists(tenant_id);
CREATE INDEX IF NOT EXISTS idx_picking_lists_branch_id ON picking_lists(branch_id);
CREATE INDEX IF NOT EXISTS idx_picking_lists_order_id ON picking_lists(order_id);

-- Enable RLS on picking_lists (Invariant I-1, I-2)
ALTER TABLE picking_lists ENABLE ROW LEVEL SECURITY;
ALTER TABLE picking_lists FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'picking_lists' AND policyname = 'picking_lists_tenant_isolation'
    ) THEN
        CREATE POLICY picking_lists_tenant_isolation ON picking_lists
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;
