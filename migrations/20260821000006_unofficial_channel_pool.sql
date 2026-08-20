-- Migration: 20260821000006_unofficial_channel_pool.sql
-- Tables for Unofficial Baileys Channel Adapter, Session Persistence, and Number Pool Manager (Doc 03)

-- 1. wa_sessions table for encrypted session persistence across container restarts (Doc 03 §5)
CREATE TABLE IF NOT EXISTS wa_sessions (
    channel_id UUID PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
    creds JSONB NOT NULL,
    keys JSONB NOT NULL,
    encrypted_secret TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_wa_sessions_tenant_id ON wa_sessions(tenant_id);

ALTER TABLE wa_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE wa_sessions FORCE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE tablename = 'wa_sessions' AND policyname = 'wa_sessions_tenant_isolation'
    ) THEN
        CREATE POLICY wa_sessions_tenant_isolation ON wa_sessions
            USING (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid)
            WITH CHECK (tenant_id = NULLIF(current_setting('app.current_tenant_id', true), '')::uuid);
    END IF;
END $$;

-- 2. Extend channels table with number pool metrics and business identity (Doc 03 §8, §9)
ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS health_score INT NOT NULL DEFAULT 100,
    ADD COLUMN IF NOT EXISTS warming_started_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS daily_sent_count INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS daily_reset_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS banned_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS business_identity_id UUID,
    ADD COLUMN IF NOT EXISTS business_identity_kind TEXT NOT NULL DEFAULT 'UnofficialIsolated';

CREATE INDEX IF NOT EXISTS idx_channels_health_score ON channels(health_score);
CREATE INDEX IF NOT EXISTS idx_channels_business_identity_id ON channels(business_identity_id);
