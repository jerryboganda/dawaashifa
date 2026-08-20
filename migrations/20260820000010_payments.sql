-- Enums for payments
CREATE TYPE payment_status AS ENUM ('PENDING', 'CONFIRMED', 'REJECTED', 'REFUNDED');
CREATE TYPE payment_gateway_type AS ENUM ('JAZZCASH', 'EASYPAISA', 'SAFEPAY', 'PAYFAST', 'RAAST', 'COD', 'DIRECT_DEPOSIT');
CREATE TYPE proof_review_status AS ENUM ('PENDING', 'APPROVED', 'REJECTED');

-- Payments table
CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    method payment_method_type NOT NULL,
    amount NUMERIC(14,4) NOT NULL,
    status payment_status NOT NULL DEFAULT 'PENDING',
    gateway payment_gateway_type,
    gateway_ref TEXT,
    gateway_payload JSONB NOT NULL DEFAULT '{}',
    confirmed_at TIMESTAMPTZ,
    confirmed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_payments_tenant_id ON payments(tenant_id);
CREATE INDEX idx_payments_order_id ON payments(order_id);

-- Payment Proofs table (Screenshot verification queue)
CREATE TABLE payment_proofs (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    payment_id UUID REFERENCES payments(id) ON DELETE SET NULL,
    image_object_key TEXT NOT NULL,
    ocr_tid TEXT,
    ocr_amount NUMERIC(14,4),
    ocr_timestamp TIMESTAMPTZ,
    ocr_sender TEXT,
    ocr_confidence NUMERIC(4,3),
    duplicate_of_proof_id UUID REFERENCES payment_proofs(id) ON DELETE SET NULL,
    fraud_flags JSONB NOT NULL DEFAULT '{}',
    review_status proof_review_status NOT NULL DEFAULT 'PENDING',
    reviewed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMPTZ,
    review_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_payment_proofs_tenant_id ON payment_proofs(tenant_id);
CREATE INDEX idx_payment_proofs_order_id ON payment_proofs(order_id);
CREATE INDEX idx_payment_proofs_ocr_tid ON payment_proofs(ocr_tid);

-- Transaction ID Ledger (Fraud prevention: prevents duplicate TID reuse)
CREATE TABLE transaction_id_ledger (
    tid TEXT NOT NULL,
    gateway TEXT NOT NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    first_seen_order_id UUID NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, gateway, tid)
);

CREATE INDEX idx_tid_ledger_tenant_id ON transaction_id_ledger(tenant_id);

