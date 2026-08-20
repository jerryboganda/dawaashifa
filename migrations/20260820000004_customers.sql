-- Customers table
CREATE TABLE customers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    msisdn TEXT NOT NULL,
    display_name TEXT,
    preferred_locale TEXT NOT NULL DEFAULT 'en',
    default_address TEXT,
    default_geo GEOGRAPHY(POINT, 4326),
    is_blocked BOOLEAN NOT NULL DEFAULT false,
    notes TEXT,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_customers_tenant_msisdn UNIQUE (tenant_id, msisdn)
);

CREATE INDEX idx_customers_tenant_id ON customers(tenant_id);
CREATE INDEX idx_customers_default_geo ON customers USING GIST(default_geo);

-- Customer Addresses table
CREATE TABLE customer_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    customer_id UUID NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    label TEXT NOT NULL DEFAULT 'Home',
    address_line TEXT NOT NULL,
    city TEXT NOT NULL,
    geo GEOGRAPHY(POINT, 4326),
    is_default BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_customer_addresses_tenant_id ON customer_addresses(tenant_id);
CREATE INDEX idx_customer_addresses_customer_id ON customer_addresses(customer_id);
CREATE INDEX idx_customer_addresses_geo ON customer_addresses USING GIST(geo);
