-- Enums for fulfilment
CREATE TYPE rider_status AS ENUM ('AVAILABLE', 'BUSY', 'OFF_DUTY', 'SUSPENDED');
CREATE TYPE delivery_status AS ENUM ('PENDING', 'ASSIGNED', 'PICKED_UP', 'OUT_FOR_DELIVERY', 'DELIVERED', 'FAILED', 'RETURNED');

-- Riders table
CREATE TABLE riders (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    vehicle_type TEXT NOT NULL DEFAULT 'MOTORBIKE',
    cnic TEXT NOT NULL,
    licence_no TEXT NOT NULL,
    status rider_status NOT NULL DEFAULT 'OFF_DUTY',
    current_geo GEOGRAPHY(POINT, 4326),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_riders_tenant_id ON riders(tenant_id);
CREATE INDEX idx_riders_branch_id ON riders(branch_id);
CREATE INDEX idx_riders_user_id ON riders(user_id);

-- Deliveries table
CREATE TABLE deliveries (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    rider_id UUID REFERENCES riders(id) ON DELETE SET NULL,
    status delivery_status NOT NULL DEFAULT 'PENDING',
    assigned_at TIMESTAMPTZ,
    picked_up_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    failed_reason TEXT,
    pod_image_object_key TEXT,
    pod_signature_object_key TEXT,
    cash_collected NUMERIC(14,4),
    delivery_geo GEOGRAPHY(POINT, 4326),
    distance_km NUMERIC(6,2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_deliveries_tenant_id ON deliveries(tenant_id);
CREATE INDEX idx_deliveries_order_id ON deliveries(order_id);
CREATE INDEX idx_deliveries_rider_id ON deliveries(rider_id);

-- Rider Cash Sessions (Daily cash-on-delivery reconciliation)
CREATE TABLE rider_cash_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    rider_id UUID NOT NULL REFERENCES riders(id) ON DELETE RESTRICT,
    opened_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at TIMESTAMPTZ,
    expected_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    collected_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    deposited_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    variance NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    reconciled_by UUID REFERENCES users(id) ON DELETE SET NULL,
    note TEXT
);

CREATE INDEX idx_rider_cash_sessions_tenant_id ON rider_cash_sessions(tenant_id);
CREATE INDEX idx_rider_cash_sessions_rider_id ON rider_cash_sessions(rider_id);

