-- Product Categories table
CREATE TABLE product_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    parent_id UUID REFERENCES product_categories(id) ON DELETE SET NULL,
    tax_category TEXT NOT NULL DEFAULT 'STANDARD',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_product_categories_tenant_name UNIQUE (tenant_id, name)
);

CREATE INDEX idx_product_categories_tenant_id ON product_categories(tenant_id);
CREATE INDEX idx_product_categories_parent_id ON product_categories(parent_id);

-- Generics master table
CREATE TABLE generics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    atc_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_generics_tenant_name UNIQUE (tenant_id, name)
);

CREATE INDEX idx_generics_tenant_id ON generics(tenant_id);
CREATE INDEX idx_generics_atc_code ON generics(atc_code);

-- Generic Equivalents table (for substitution engine)
CREATE TABLE generic_equivalents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    generic_id UUID NOT NULL REFERENCES generics(id) ON DELETE CASCADE,
    equivalent_generic_id UUID NOT NULL REFERENCES generics(id) ON DELETE CASCADE,
    equivalence_type TEXT NOT NULL DEFAULT 'BIOEQUIVALENT',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_generic_equivalents UNIQUE (tenant_id, generic_id, equivalent_generic_id)
);

CREATE INDEX idx_generic_equivalents_tenant_id ON generic_equivalents(tenant_id);
CREATE INDEX idx_generic_equivalents_generic_id ON generic_equivalents(generic_id);
CREATE INDEX idx_generic_equivalents_equiv_id ON generic_equivalents(equivalent_generic_id);

-- Product status enum
CREATE TYPE product_status AS ENUM ('ACTIVE', 'DISCONTINUED', 'OUT_OF_STOCK');

-- Products table (Drug master)
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    sku TEXT NOT NULL,
    name_en TEXT NOT NULL,
    name_ur TEXT,
    form TEXT NOT NULL,
    strength TEXT,
    pack_size TEXT NOT NULL,
    manufacturer TEXT NOT NULL,
    drap_registration_no TEXT NOT NULL,
    is_prescription_only BOOLEAN NOT NULL DEFAULT false,
    is_controlled BOOLEAN NOT NULL DEFAULT false,
    requires_cold_chain BOOLEAN NOT NULL DEFAULT false,
    mrp NUMERIC(14,4) NOT NULL,
    category_id UUID REFERENCES product_categories(id) ON DELETE SET NULL,
    hs_code TEXT,
    pct_code TEXT,
    status product_status NOT NULL DEFAULT 'ACTIVE',
    embedding vector(1024),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_products_tenant_sku UNIQUE (tenant_id, sku)
);

CREATE INDEX idx_products_tenant_id ON products(tenant_id);
CREATE INDEX idx_products_category_id ON products(category_id);
CREATE INDEX idx_products_name_en_trgm ON products USING GIN(name_en gin_trgm_ops);
CREATE INDEX idx_products_drap_reg ON products(drap_registration_no);

-- Product Generics mapping table
CREATE TABLE product_generics (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    generic_id UUID NOT NULL REFERENCES generics(id) ON DELETE CASCADE,
    strength_mg NUMERIC(10,3),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, product_id, generic_id)
);

CREATE INDEX idx_product_generics_tenant_id ON product_generics(tenant_id);
CREATE INDEX idx_product_generics_product_id ON product_generics(product_id);
CREATE INDEX idx_product_generics_generic_id ON product_generics(generic_id);

-- Product Aliases table (high leverage matching engine)
CREATE TABLE product_aliases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    alias_type TEXT NOT NULL DEFAULT 'TRANSLITERATION',
    script TEXT NOT NULL DEFAULT 'LATIN',
    weight NUMERIC(3,2) NOT NULL DEFAULT 1.00,
    source TEXT NOT NULL DEFAULT 'MANUAL',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_product_aliases UNIQUE (tenant_id, product_id, alias)
);

CREATE INDEX idx_product_aliases_tenant_id ON product_aliases(tenant_id);
CREATE INDEX idx_product_aliases_product_id ON product_aliases(product_id);
CREATE INDEX idx_product_aliases_trgm ON product_aliases USING GIN(alias gin_trgm_ops);
