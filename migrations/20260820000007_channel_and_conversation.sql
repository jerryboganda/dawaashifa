-- Enums for communication channels and messages
CREATE TYPE channel_transport AS ENUM ('CLOUD_API', 'UNOFFICIAL_BAILEYS');
CREATE TYPE channel_status AS ENUM ('PROVISIONING', 'WARMING', 'ACTIVE', 'DEGRADED', 'BANNED', 'RETIRED');
CREATE TYPE conversation_status AS ENUM ('ACTIVE', 'WAITING_HUMAN', 'RESOLVED', 'EXPIRED');
CREATE TYPE message_direction AS ENUM ('INBOUND', 'OUTBOUND');
CREATE TYPE message_content_type AS ENUM ('TEXT', 'IMAGE', 'AUDIO', 'DOCUMENT', 'INTERACTIVE_LIST', 'INTERACTIVE_BUTTONS', 'TEMPLATE');
CREATE TYPE message_status AS ENUM ('PENDING', 'SENT', 'DELIVERED', 'READ', 'FAILED');

-- Business identities table
CREATE TABLE business_identities (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'OFFICIAL',
    meta_business_id TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_biz_identities_tenant_id ON business_identities(tenant_id);

-- Channels table
CREATE TABLE channels (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    transport channel_transport NOT NULL DEFAULT 'CLOUD_API',
    msisdn TEXT NOT NULL,
    display_name TEXT NOT NULL,
    status channel_status NOT NULL DEFAULT 'PROVISIONING',
    business_identity_id UUID NOT NULL REFERENCES business_identities(id) ON DELETE RESTRICT,
    waba_id TEXT,
    phone_number_id TEXT,
    session_ref TEXT,
    health_score NUMERIC(4,3) NOT NULL DEFAULT 1.000,
    banned_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    daily_sent_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_channels_tenant_msisdn UNIQUE (tenant_id, msisdn)
);

CREATE INDEX idx_channels_tenant_id ON channels(tenant_id);
CREATE INDEX idx_channels_biz_id ON channels(business_identity_id);

-- Conversations table
CREATE TABLE conversations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    customer_id UUID NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE RESTRICT,
    branch_id UUID REFERENCES branches(id) ON DELETE SET NULL,
    status conversation_status NOT NULL DEFAULT 'ACTIVE',
    assigned_to UUID REFERENCES users(id) ON DELETE SET NULL,
    last_inbound_at TIMESTAMPTZ,
    last_outbound_at TIMESTAMPTZ,
    window_expires_at TIMESTAMPTZ,
    locale_detected TEXT DEFAULT 'en',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_conversations_tenant_id ON conversations(tenant_id);
CREATE INDEX idx_conversations_customer_id ON conversations(customer_id);
CREATE INDEX idx_conversations_channel_id ON conversations(channel_id);
CREATE INDEX idx_conversations_branch_id ON conversations(branch_id);

-- Messages table (Partitioned by created_at)
CREATE TABLE messages (
    id UUID DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    direction message_direction NOT NULL,
    transport_message_id TEXT,
    content_type message_content_type NOT NULL DEFAULT 'TEXT',
    body TEXT,
    media_object_key TEXT,
    template_name TEXT,
    status message_status NOT NULL DEFAULT 'PENDING',
    ai_generated BOOLEAN NOT NULL DEFAULT false,
    ai_confidence NUMERIC(4,3),
    overridden_by UUID REFERENCES users(id) ON DELETE SET NULL,
    sent_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    failed_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE messages_default PARTITION OF messages DEFAULT;

CREATE INDEX idx_messages_tenant_id ON messages(tenant_id);
CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_messages_transport_id ON messages(transport_message_id);

