-- Enums for orders
CREATE TYPE order_status AS ENUM (
    'DRAFT', 'CART_CONFIRMED', 'AWAITING_RX', 'RX_UNDER_REVIEW',
    'RX_APPROVED', 'RX_REJECTED', 'AWAITING_PAYMENT', 'PAYMENT_UNDER_REVIEW',
    'PAYMENT_REJECTED', 'CONFIRMED', 'PICKING', 'PACKED', 'DISPATCHED',
    'OUT_FOR_DELIVERY', 'DELIVERED', 'CASH_RECONCILED', 'CLOSED',
    'CANCELLED', 'FAILED_DELIVERY', 'RETURNED', 'REFUNDED'
);

CREATE TYPE order_type AS ENUM ('RETAIL', 'B2B');
CREATE TYPE payment_method_type AS ENUM ('COD', 'GATEWAY', 'BANK_TRANSFER_SCREENSHOT', 'B2B_CREDIT');

-- Orders table
CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    branch_id UUID NOT NULL REFERENCES branches(id) ON DELETE RESTRICT,
    customer_id UUID NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
    conversation_id UUID REFERENCES conversations(id) ON DELETE SET NULL,
    prescription_id UUID REFERENCES prescriptions(id) ON DELETE SET NULL,
    order_no TEXT NOT NULL,
    status order_status NOT NULL DEFAULT 'DRAFT',
    order_type order_type NOT NULL DEFAULT 'RETAIL',
    subtotal NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    discount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    delivery_fee NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    tax_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    total NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    payment_method payment_method_type NOT NULL DEFAULT 'COD',
    delivery_address TEXT NOT NULL,
    delivery_geo GEOGRAPHY(POINT, 4326),
    placed_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_orders_tenant_no UNIQUE (tenant_id, order_no)
);

CREATE INDEX idx_orders_tenant_id ON orders(tenant_id);
CREATE INDEX idx_orders_branch_id ON orders(branch_id);
CREATE INDEX idx_orders_customer_id ON orders(customer_id);
CREATE INDEX idx_orders_prescription_id ON orders(prescription_id);
CREATE INDEX idx_orders_status ON orders(status);

-- Order Items table
CREATE TABLE order_items (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    batch_id UUID REFERENCES batches(id) ON DELETE SET NULL,
    qty INTEGER NOT NULL,
    unit_price NUMERIC(14,4) NOT NULL,
    mrp_at_sale NUMERIC(14,4) NOT NULL,
    discount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    line_total NUMERIC(14,4) NOT NULL,
    tax_rate NUMERIC(5,2) NOT NULL DEFAULT 0.00,
    tax_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    is_prescription_only BOOLEAN NOT NULL DEFAULT false,
    substituted_from_product_id UUID REFERENCES products(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_order_items_tenant_id ON order_items(tenant_id);
CREATE INDEX idx_order_items_order_id ON order_items(order_id);
CREATE INDEX idx_order_items_product_id ON order_items(product_id);
CREATE INDEX idx_order_items_batch_id ON order_items(batch_id);

-- Order Events table (Immutable state transition audit trail)
CREATE TABLE order_events (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    from_status order_status NOT NULL,
    to_status order_status NOT NULL,
    actor_id UUID,
    actor_type TEXT NOT NULL DEFAULT 'SYSTEM',
    reason TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_order_events_tenant_id ON order_events(tenant_id);
CREATE INDEX idx_order_events_order_id ON order_events(order_id);

