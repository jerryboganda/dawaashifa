-- Migration: 20260821000007_b2b_schema.sql
-- B2B Accounts, Contacts, Price Lists, Quotes, Purchase Orders, Consignment Stock & Device Traceability (Doc 14)

-- 1. Business Accounts (Doc 14 §5)
CREATE TABLE IF NOT EXISTS business_accounts (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    account_type TEXT NOT NULL DEFAULT 'HOSPITAL',
    ntn TEXT,
    strn TEXT,
    billing_address TEXT NOT NULL,
    shipping_addresses JSONB NOT NULL DEFAULT '[]'::jsonb,
    credit_limit NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    payment_terms_days INT NOT NULL DEFAULT 30,
    price_list_id UUID,
    status TEXT NOT NULL DEFAULT 'ACTIVE',
    on_hold BOOLEAN NOT NULL DEFAULT false,
    hold_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_business_accounts_tenant_id ON business_accounts(tenant_id);
CREATE INDEX IF NOT EXISTS idx_business_accounts_status ON business_accounts(status);
CREATE INDEX IF NOT EXISTS idx_business_accounts_on_hold ON business_accounts(on_hold);

ALTER TABLE business_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE business_accounts FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'business_accounts' AND policyname = 'business_accounts_tenant_isolation'
    ) THEN
        CREATE POLICY business_accounts_tenant_isolation ON business_accounts
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 2. Business Contacts (Doc 14 §5)
CREATE TABLE IF NOT EXISTS business_contacts (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    account_id UUID NOT NULL REFERENCES business_accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    designation TEXT NOT NULL,
    phone TEXT NOT NULL,
    email TEXT,
    can_approve_po BOOLEAN NOT NULL DEFAULT false,
    approval_limit NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_business_contacts_account_id ON business_contacts(account_id);
CREATE INDEX IF NOT EXISTS idx_business_contacts_tenant_id ON business_contacts(tenant_id);

ALTER TABLE business_contacts ENABLE ROW LEVEL SECURITY;
ALTER TABLE business_contacts FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'business_contacts' AND policyname = 'business_contacts_tenant_isolation'
    ) THEN
        CREATE POLICY business_contacts_tenant_isolation ON business_contacts
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 3. Price Lists & Price List Items (Doc 14 §5)
CREATE TABLE IF NOT EXISTS price_lists (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    valid_from TIMESTAMPTZ NOT NULL DEFAULT now(),
    valid_to TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_price_lists_tenant_id ON price_lists(tenant_id);

ALTER TABLE price_lists ENABLE ROW LEVEL SECURITY;
ALTER TABLE price_lists FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'price_lists' AND policyname = 'price_lists_tenant_isolation'
    ) THEN
        CREATE POLICY price_lists_tenant_isolation ON price_lists
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS price_list_items (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    price_list_id UUID NOT NULL REFERENCES price_lists(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    price NUMERIC(14,4) NOT NULL,
    min_qty INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_price_list_items_price_list_id ON price_list_items(price_list_id);
CREATE INDEX IF NOT EXISTS idx_price_list_items_product_id ON price_list_items(product_id);
CREATE INDEX IF NOT EXISTS idx_price_list_items_tenant_id ON price_list_items(tenant_id);

ALTER TABLE price_list_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE price_list_items FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'price_list_items' AND policyname = 'price_list_items_tenant_isolation'
    ) THEN
        CREATE POLICY price_list_items_tenant_isolation ON price_list_items
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 4. Quotations & Quotation Items (Doc 14 §6)
CREATE TABLE IF NOT EXISTS quotations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    account_id UUID NOT NULL REFERENCES business_accounts(id) ON DELETE RESTRICT,
    quote_no TEXT NOT NULL,
    version INT NOT NULL DEFAULT 1,
    parent_quote_id UUID REFERENCES quotations(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'DRAFT',
    valid_until TIMESTAMPTZ NOT NULL,
    subtotal NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    discount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    tax_amount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    total NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    terms_text TEXT,
    prepared_by UUID NOT NULL,
    approved_by UUID,
    sent_at TIMESTAMPTZ,
    responded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_quotations_tenant_id ON quotations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_quotations_account_id ON quotations(account_id);
CREATE INDEX IF NOT EXISTS idx_quotations_quote_no ON quotations(quote_no);
CREATE INDEX IF NOT EXISTS idx_quotations_status ON quotations(status);

ALTER TABLE quotations ENABLE ROW LEVEL SECURITY;
ALTER TABLE quotations FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'quotations' AND policyname = 'quotations_tenant_isolation'
    ) THEN
        CREATE POLICY quotations_tenant_isolation ON quotations
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS quotation_items (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    quotation_id UUID NOT NULL REFERENCES quotations(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    qty INT NOT NULL,
    unit_price NUMERIC(14,4) NOT NULL,
    discount NUMERIC(14,4) NOT NULL DEFAULT 0.0000,
    line_total NUMERIC(14,4) NOT NULL,
    lead_time_days INT NOT NULL DEFAULT 0,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_quotation_items_quotation_id ON quotation_items(quotation_id);
CREATE INDEX IF NOT EXISTS idx_quotation_items_product_id ON quotation_items(product_id);
CREATE INDEX IF NOT EXISTS idx_quotation_items_tenant_id ON quotation_items(tenant_id);

ALTER TABLE quotation_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE quotation_items FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'quotation_items' AND policyname = 'quotation_items_tenant_isolation'
    ) THEN
        CREATE POLICY quotation_items_tenant_isolation ON quotation_items
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 5. Purchase Orders (Doc 14 §7)
CREATE TABLE IF NOT EXISTS purchase_orders (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    account_id UUID NOT NULL REFERENCES business_accounts(id) ON DELETE RESTRICT,
    quotation_id UUID REFERENCES quotations(id) ON DELETE RESTRICT,
    po_number TEXT NOT NULL,
    po_document_key TEXT,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    verified_by UUID,
    amount NUMERIC(14,4) NOT NULL,
    variance_detected BOOLEAN NOT NULL DEFAULT false,
    variance_notes TEXT,
    status TEXT NOT NULL DEFAULT 'PENDING_VERIFICATION',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_purchase_orders_tenant_id ON purchase_orders(tenant_id);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_account_id ON purchase_orders(account_id);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_quotation_id ON purchase_orders(quotation_id);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_status ON purchase_orders(status);

ALTER TABLE purchase_orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE purchase_orders FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'purchase_orders' AND policyname = 'purchase_orders_tenant_isolation'
    ) THEN
        CREATE POLICY purchase_orders_tenant_isolation ON purchase_orders
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 6. Consignment Locations & Stock (Doc 14 §10)
CREATE TABLE IF NOT EXISTS consignment_locations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    account_id UUID NOT NULL REFERENCES business_accounts(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    managed_by TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_consignment_locations_tenant_id ON consignment_locations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_consignment_locations_account_id ON consignment_locations(account_id);

ALTER TABLE consignment_locations ENABLE ROW LEVEL SECURITY;
ALTER TABLE consignment_locations FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'consignment_locations' AND policyname = 'consignment_locations_tenant_isolation'
    ) THEN
        CREATE POLICY consignment_locations_tenant_isolation ON consignment_locations
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS consignment_stock (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    location_id UUID NOT NULL REFERENCES consignment_locations(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    batch_id UUID,
    serial_no TEXT,
    qty INT NOT NULL DEFAULT 1,
    placed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    consumed_at TIMESTAMPTZ,
    invoiced_at TIMESTAMPTZ,
    discrepancy_flagged BOOLEAN NOT NULL DEFAULT false,
    discrepancy_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_consignment_stock_location_id ON consignment_stock(location_id);
CREATE INDEX IF NOT EXISTS idx_consignment_stock_product_id ON consignment_stock(product_id);
CREATE INDEX IF NOT EXISTS idx_consignment_stock_tenant_id ON consignment_stock(tenant_id);

ALTER TABLE consignment_stock ENABLE ROW LEVEL SECURITY;
ALTER TABLE consignment_stock FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'consignment_stock' AND policyname = 'consignment_stock_tenant_isolation'
    ) THEN
        CREATE POLICY consignment_stock_tenant_isolation ON consignment_stock
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 7. Device Traceability (Doc 14 §11)
CREATE TABLE IF NOT EXISTS device_units (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    batch_id UUID,
    serial_no TEXT NOT NULL,
    udi TEXT,
    status TEXT NOT NULL DEFAULT 'IN_STOCK',
    location_type TEXT NOT NULL DEFAULT 'WAREHOUSE',
    location_id UUID,
    implanted_at TIMESTAMPTZ,
    patient_ref TEXT,
    surgeon_name TEXT,
    order_id UUID REFERENCES orders(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_device_units_serial UNIQUE (tenant_id, serial_no)
);

CREATE INDEX IF NOT EXISTS idx_device_units_tenant_id ON device_units(tenant_id);
CREATE INDEX IF NOT EXISTS idx_device_units_product_id ON device_units(product_id);
CREATE INDEX IF NOT EXISTS idx_device_units_serial_no ON device_units(serial_no);
CREATE INDEX IF NOT EXISTS idx_device_units_status ON device_units(status);

ALTER TABLE device_units ENABLE ROW LEVEL SECURITY;
ALTER TABLE device_units FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'device_units' AND policyname = 'device_units_tenant_isolation'
    ) THEN
        CREATE POLICY device_units_tenant_isolation ON device_units
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;
