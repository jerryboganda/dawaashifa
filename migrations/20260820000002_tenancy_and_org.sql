-- Enums for tenancy and branches
CREATE TYPE tenant_status AS ENUM ('ACTIVE', 'SUSPENDED', 'DEACTIVATED');
CREATE TYPE branch_status AS ENUM ('ACTIVE', 'CLOSED', 'REFURBISHING');

-- Tenants table (Root organization)
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL,
    legal_name TEXT NOT NULL,
    ntn TEXT,
    strn TEXT,
    status tenant_status NOT NULL DEFAULT 'ACTIVE',
    settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Branches table
CREATE TABLE branches (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    code TEXT NOT NULL,
    drap_licence_no TEXT NOT NULL,
    pharmacist_in_charge TEXT NOT NULL,
    address TEXT NOT NULL,
    city TEXT NOT NULL,
    geo GEOGRAPHY(POINT, 4326) NOT NULL,
    service_radius_km NUMERIC(6,2) NOT NULL DEFAULT 10.00,
    is_hub BOOLEAN NOT NULL DEFAULT false,
    cold_chain_capable BOOLEAN NOT NULL DEFAULT false,
    strn TEXT,
    fbr_pos_id TEXT,
    opening_hours JSONB NOT NULL DEFAULT '{}',
    status branch_status NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_branches_tenant_code UNIQUE (tenant_id, code)
);

CREATE INDEX idx_branches_tenant_id ON branches(tenant_id);
CREATE INDEX idx_branches_geo ON branches USING GIST(geo);

