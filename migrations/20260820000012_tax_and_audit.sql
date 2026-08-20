-- Enums for FBR & Invoicing
CREATE TYPE fbr_status_type AS ENUM ('PENDING', 'TRANSMITTED', 'FAILED', 'EXEMPT');

-- Tax Categories table
CREATE TABLE tax_categories (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    rate NUMERIC(5,2) NOT NULL,
    fbr_code TEXT,
    is_exempt BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_tax_categories_tenant_name UNIQUE (tenant_id, name)
);

CREATE INDEX idx_tax_categories_tenant_id ON tax_categories(tenant_id);

-- Invoices table (FBR digital fiscal invoicing)
CREATE TABLE invoices (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    invoice_no TEXT NOT NULL,
    fiscal_invoice_no TEXT,
    fbr_status fbr_status_type NOT NULL DEFAULT 'PENDING',
    fbr_request JSONB,
    fbr_response JSONB,
    fbr_qr_payload TEXT,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    pdf_object_key TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_invoices_tenant_no UNIQUE (tenant_id, invoice_no)
);

CREATE INDEX idx_invoices_tenant_id ON invoices(tenant_id);
CREATE INDEX idx_invoices_branch_id ON invoices(branch_id);
CREATE INDEX idx_invoices_order_id ON invoices(order_id);
CREATE INDEX idx_invoices_fiscal_no ON invoices(fiscal_invoice_no);

-- Audit Log table (Immutable regulatory audit trail, partitioned by month)
CREATE TABLE audit_log (
    id UUID DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    actor_id UUID,
    actor_type TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    action TEXT NOT NULL,
    before JSONB,
    after JSONB,
    reason TEXT,
    ip TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

CREATE TABLE audit_log_default PARTITION OF audit_log DEFAULT;

CREATE INDEX idx_audit_log_tenant_id ON audit_log(tenant_id);
CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_actor ON audit_log(actor_id);

