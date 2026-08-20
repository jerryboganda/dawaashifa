-- Suppliers table
CREATE TABLE suppliers (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    contact TEXT,
    ntn TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_suppliers_tenant_id ON suppliers(tenant_id);

-- Batches table
CREATE TABLE batches (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    batch_no TEXT NOT NULL,
    expiry_date DATE NOT NULL,
    cost_price NUMERIC(14,4) NOT NULL,
    mrp_at_receipt NUMERIC(14,4) NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    supplier_id UUID REFERENCES suppliers(id) ON DELETE SET NULL,
    qty_received INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_batches_branch_product_no UNIQUE (tenant_id, product_id, branch_id, batch_no)
);

CREATE INDEX idx_batches_tenant_id ON batches(tenant_id);
CREATE INDEX idx_batches_product_id ON batches(product_id);
CREATE INDEX idx_batches_branch_id ON batches(branch_id);
CREATE INDEX idx_batches_expiry ON batches(expiry_date);

-- Stock Movement Type enum
CREATE TYPE stock_movement_type AS ENUM (
    'RECEIPT', 'SALE', 'RETURN', 'TRANSFER_OUT', 'TRANSFER_IN',
    'ADJUSTMENT', 'EXPIRY_WRITEOFF', 'DAMAGE', 'RESERVATION', 'RELEASE'
);

-- Stock Movements table (Append-only ledger, Partitioned by month)
CREATE TABLE stock_movements (
    id UUID DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    batch_id UUID NOT NULL REFERENCES batches(id) ON DELETE RESTRICT,
    qty_delta INTEGER NOT NULL,
    movement_type stock_movement_type NOT NULL,
    ref_type TEXT,
    ref_id UUID,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor_id UUID,
    note TEXT,
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

-- Default partition for stock movements
CREATE TABLE stock_movements_default PARTITION OF stock_movements DEFAULT;

CREATE INDEX idx_stock_movements_tenant_id ON stock_movements(tenant_id);
CREATE INDEX idx_stock_movements_branch_id ON stock_movements(branch_id);
CREATE INDEX idx_stock_movements_product_id ON stock_movements(product_id);
CREATE INDEX idx_stock_movements_batch_id ON stock_movements(batch_id);

-- Current Stock cache table (maintained by trigger / ledger sync)
CREATE TABLE stock_current (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    batch_id UUID NOT NULL REFERENCES batches(id) ON DELETE RESTRICT,
    qty INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, branch_id, product_id, batch_id)
);

CREATE INDEX idx_stock_current_tenant_id ON stock_current(tenant_id);
CREATE INDEX idx_stock_current_product ON stock_current(product_id);
CREATE INDEX idx_stock_current_branch ON stock_current(branch_id);

-- Cold Chain Logs table
CREATE TABLE cold_chain_logs (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    batch_id UUID NOT NULL REFERENCES batches(id) ON DELETE RESTRICT,
    temperature_c NUMERIC(5,2) NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    recorded_by UUID REFERENCES users(id) ON DELETE SET NULL,
    is_excursion BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_cold_chain_logs_tenant_id ON cold_chain_logs(tenant_id);
CREATE INDEX idx_cold_chain_logs_branch_id ON cold_chain_logs(branch_id);
CREATE INDEX idx_cold_chain_logs_batch_id ON cold_chain_logs(batch_id);

