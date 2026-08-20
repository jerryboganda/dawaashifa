-- Enums for prescriptions
CREATE TYPE prescription_status AS ENUM ('PENDING_OCR', 'RX_UNDER_REVIEW', 'APPROVED', 'REJECTED', 'CANCELLED');
CREATE TYPE pharmacist_action_type AS ENUM ('ACCEPTED', 'EDITED', 'REJECTED', 'ADDED_MANUALLY');
CREATE TYPE rx_approval_decision AS ENUM ('APPROVED', 'REJECTED');

-- Prescriptions table
CREATE TABLE prescriptions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    customer_id UUID NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
    conversation_id UUID REFERENCES conversations(id) ON DELETE SET NULL,
    image_object_key TEXT NOT NULL,
    source_channel TEXT NOT NULL DEFAULT 'WHATSAPP',
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status prescription_status NOT NULL DEFAULT 'PENDING_OCR',
    doctor_name TEXT,
    doctor_pmdc_no TEXT,
    issued_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_prescriptions_tenant_id ON prescriptions(tenant_id);
CREATE INDEX idx_prescriptions_customer_id ON prescriptions(customer_id);
CREATE INDEX idx_prescriptions_status ON prescriptions(status);

-- Rx OCR Results table
CREATE TABLE rx_ocr_results (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    prescription_id UUID NOT NULL REFERENCES prescriptions(id) ON DELETE CASCADE,
    model_name TEXT NOT NULL,
    model_version TEXT NOT NULL,
    raw_output JSONB NOT NULL DEFAULT '{}',
    confidence_overall NUMERIC(4,3) NOT NULL,
    processing_ms INTEGER NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rx_ocr_results_tenant_id ON rx_ocr_results(tenant_id);
CREATE INDEX idx_rx_ocr_results_prescription_id ON rx_ocr_results(prescription_id);

-- Rx Lines table
CREATE TABLE rx_lines (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    prescription_id UUID NOT NULL REFERENCES prescriptions(id) ON DELETE CASCADE,
    line_no INTEGER NOT NULL,
    ocr_text TEXT NOT NULL,
    matched_product_id UUID REFERENCES products(id) ON DELETE SET NULL,
    match_confidence NUMERIC(4,3),
    match_method TEXT,
    qty INTEGER NOT NULL DEFAULT 1,
    dosage_instructions TEXT,
    pharmacist_action pharmacist_action_type,
    pharmacist_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_rx_lines_tenant_id ON rx_lines(tenant_id);
CREATE INDEX idx_rx_lines_prescription_id ON rx_lines(prescription_id);
CREATE INDEX idx_rx_lines_product_id ON rx_lines(matched_product_id);

-- Pharmacist Approvals table (Mandatory human approval audit record)
CREATE TABLE pharmacist_approvals (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    prescription_id UUID NOT NULL REFERENCES prescriptions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    decision rx_approval_decision NOT NULL,
    reason TEXT,
    approved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip TEXT,
    device TEXT
);

CREATE INDEX idx_pharmacist_approvals_tenant_id ON pharmacist_approvals(tenant_id);
CREATE INDEX idx_pharmacist_approvals_rx_id ON pharmacist_approvals(prescription_id);
CREATE INDEX idx_pharmacist_approvals_user_id ON pharmacist_approvals(user_id);

