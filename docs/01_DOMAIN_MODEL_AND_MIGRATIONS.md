# DOC 01 â€” DOMAIN MODEL, ERD & MIGRATION SET

**Agent:** Backend (Copilot)
**Depends on:** nothing â€” this is the foundation
**Produces:** `crates/core`, `crates/db`, complete `migrations/`
**Branch:** `feat/01-domain-model`

---

## 1. Objective

Establish the workspace, the core domain types, and the complete database schema with RLS. Every later spec builds on this. Get it right; migrations are forward-only.

## 2. In scope

- Cargo workspace skeleton with all crates stubbed
- `crates/core`: newtype IDs, `Money`, `TenantContext`, error primitives
- `crates/db`: connection pool, migration runner, RLS session setup, repository trait
- All migrations for every table in Â§5
- `docker-compose.yml`: Postgres 18 (+pgvector, pg_trgm, pg_partman), Redis 7, NATS JetStream, MinIO
- Seed data generator: 1 tenant, 8 branches, 5,000 products, 50 users

## 3. Out of scope â€” do NOT build

- Any HTTP route or Axum handler
- Any business logic
- Any repository method beyond a single proof-of-concept `find_by_id`
- Authentication (Doc 04)
- Frontend of any kind

## 4. Workspace skeleton

```
Cargo.toml                    # [workspace] members
crates/
  core/  db/  identity/  catalog/  inventory/  channel/  conversation/
  ai/  prescription/  orders/  payments/  fulfilment/  b2b/  tax/  admin/
  api/  worker/
migrations/
docker-compose.yml
.env.example
```

Every crate other than `core` and `db` is a stub with `lib.rs` containing only a module doc comment. They are filled by later specs.

## 5. Core types (`crates/core`)

```rust
// Newtype IDs â€” prevents argument-order bugs at compile time
macro_rules! id_type { ($n:ident) => {
    #[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,Serialize,Deserialize,sqlx::Type)]
    #[sqlx(transparent)]
    pub struct $n(pub uuid::Uuid);
};}
id_type!(TenantId);  id_type!(BranchId);  id_type!(UserId);
id_type!(ProductId); id_type!(BatchId);   id_type!(OrderId);
id_type!(CustomerId); id_type!(PrescriptionId); id_type!(ConversationId);
id_type!(MessageId); id_type!(RiderId);   id_type!(ChannelId);

// Money â€” NEVER f64
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(pub rust_decimal::Decimal);
// Serialises as a STRING over the wire. #[schema(value_type = String)]

// Tenant context â€” comes from the JWT, never from a request body
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub branch_ids: Vec<BranchId>,   // branches this user may act on
    pub permissions: HashSet<String>,
}
```

## 6. Schema

### 6.1 Tenancy & org
```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    name TEXT NOT NULL, legal_name TEXT NOT NULL,
    ntn TEXT, strn TEXT,
    status tenant_status NOT NULL DEFAULT 'ACTIVE',
    settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE branches (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    name TEXT NOT NULL, code TEXT NOT NULL,
    drap_licence_no TEXT NOT NULL,
    pharmacist_in_charge TEXT NOT NULL,
    address TEXT NOT NULL, city TEXT NOT NULL,
    geo GEOGRAPHY(POINT, 4326) NOT NULL,
    service_radius_km NUMERIC(6,2) NOT NULL DEFAULT 10,
    is_hub BOOLEAN NOT NULL DEFAULT false,
    cold_chain_capable BOOLEAN NOT NULL DEFAULT false,
    strn TEXT, fbr_pos_id TEXT,
    opening_hours JSONB NOT NULL DEFAULT '{}',
    status branch_status NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, code)
);
```

### 6.2 Identity
```sql
users(id, tenant_id, phone E.164 UNIQUE per tenant, email, full_name,
      password_hash, status, locale, last_login_at, created_at, updated_at)
roles(id, tenant_id, name, is_system, description)
permissions(id, key UNIQUE)              -- 'rx.approve', 'payment.approve', ...
role_permissions(role_id, permission_id)
user_roles(user_id, role_id)
user_branches(user_id, branch_id)        -- which branches a user may act on
sessions(id, user_id, tenant_id, token_hash, expires_at, ip, user_agent, revoked_at)
```

### 6.3 Customers
```sql
customers(id, tenant_id, msisdn E.164, display_name, preferred_locale,
          default_address, default_geo GEOGRAPHY(POINT,4326),
          is_blocked, notes, first_seen_at, last_seen_at, created_at, updated_at)
customer_addresses(id, tenant_id, customer_id, label, address_line,
                   city, geo, is_default, created_at)
UNIQUE (tenant_id, msisdn)
```

### 6.4 Catalog â€” detail in Doc 05
```sql
product_categories(id, tenant_id, name, parent_id, tax_category)
products(id, tenant_id, sku, name_en, name_ur, form, strength, pack_size,
         manufacturer, drap_registration_no, is_prescription_only,
         is_controlled, requires_cold_chain, mrp NUMERIC(14,4),
         category_id, hs_code, pct_code, status, embedding vector(1024),
         created_at, updated_at)
generics(id, name, atc_code)
product_generics(product_id, generic_id, strength_mg)
generic_equivalents(generic_id, equivalent_generic_id, equivalence_type)
product_aliases(id, tenant_id, product_id, alias, alias_type, script,
                weight NUMERIC(3,2), source, created_at)
```

### 6.5 Inventory â€” detail in Doc 06
```sql
suppliers(id, tenant_id, name, contact, ntn, created_at)
batches(id, tenant_id, product_id, branch_id, batch_no, expiry_date,
        cost_price, mrp_at_receipt, received_at, supplier_id, qty_received)
stock_movements(id, tenant_id, branch_id, product_id, batch_id,
                qty_delta INTEGER NOT NULL, movement_type,
                ref_type, ref_id, occurred_at, actor_id, note)
  PARTITION BY RANGE (occurred_at)
stock_current(tenant_id, branch_id, product_id, batch_id, qty,
              PRIMARY KEY (tenant_id, branch_id, product_id, batch_id))
cold_chain_logs(id, tenant_id, branch_id, batch_id, temperature_c,
                recorded_at, recorded_by, is_excursion)
```

### 6.6 Channel & conversation â€” Docs 02, 03, 07
```sql
channels(id, tenant_id, transport, msisdn, display_name, status,
         business_identity_id, waba_id, phone_number_id,
         session_ref, health_score, banned_at, last_seen_at,
         daily_sent_count, created_at, updated_at)
business_identities(id, tenant_id, name, kind, meta_business_id, notes)
conversations(id, tenant_id, customer_id, channel_id, branch_id,
              status, assigned_to, last_inbound_at, last_outbound_at,
              window_expires_at, locale_detected, created_at, updated_at)
messages(id, tenant_id, conversation_id, direction, transport_message_id,
         content_type, body, media_object_key, template_name,
         status, ai_generated, ai_confidence, overridden_by,
         sent_at, delivered_at, read_at, failed_reason, created_at)
  PARTITION BY RANGE (created_at)
```

### 6.7 Prescriptions â€” Doc 09
```sql
prescriptions(id, tenant_id, customer_id, conversation_id,
              image_object_key, source_channel, received_at, status,
              doctor_name, doctor_pmdc_no, issued_date, created_at, updated_at)
rx_ocr_results(id, tenant_id, prescription_id, model_name, model_version,
               raw_output JSONB, confidence_overall NUMERIC(4,3),
               processing_ms INTEGER, processed_at)
rx_lines(id, tenant_id, prescription_id, line_no, ocr_text,
         matched_product_id, match_confidence NUMERIC(4,3), match_method,
         qty, dosage_instructions, pharmacist_action, pharmacist_note)
pharmacist_approvals(id, tenant_id, prescription_id, user_id, decision,
                     reason, approved_at, ip, device)
```

### 6.8 Orders â€” Doc 10
```sql
orders(id, tenant_id, branch_id, customer_id, conversation_id,
       prescription_id NULL, order_no TEXT, status order_status,
       order_type, subtotal, discount, delivery_fee, tax_amount, total,
       payment_method, delivery_address, delivery_geo,
       placed_at, confirmed_at, delivered_at, closed_at,
       created_at, updated_at, UNIQUE (tenant_id, order_no))
order_items(id, tenant_id, order_id, product_id, batch_id, qty,
            unit_price, mrp_at_sale, discount, line_total, tax_rate,
            tax_amount, is_prescription_only, substituted_from_product_id)
order_events(id, tenant_id, order_id, from_status, to_status,
             actor_id, actor_type, reason, occurred_at)
```

### 6.9 Payments â€” Doc 11
```sql
payments(id, tenant_id, order_id, method, amount, status,
         gateway, gateway_ref, gateway_payload JSONB,
         confirmed_at, confirmed_by, created_at, updated_at)
payment_proofs(id, tenant_id, order_id, payment_id, image_object_key,
               ocr_tid, ocr_amount, ocr_timestamp, ocr_sender,
               ocr_confidence, duplicate_of_proof_id, fraud_flags JSONB,
               review_status, reviewed_by, reviewed_at, review_note)
transaction_id_ledger(tid TEXT, gateway TEXT, tenant_id UUID,
                      first_seen_order_id UUID, first_seen_at TIMESTAMPTZ,
                      PRIMARY KEY (tenant_id, gateway, tid))
```

### 6.10 Fulfilment â€” Doc 12
```sql
riders(id, tenant_id, branch_id, user_id, vehicle_type, cnic,
       licence_no, status, current_geo, created_at)
deliveries(id, tenant_id, order_id, rider_id, status, assigned_at,
           picked_up_at, delivered_at, failed_reason,
           pod_image_object_key, pod_signature_object_key,
           cash_collected, delivery_geo, distance_km)
rider_cash_sessions(id, tenant_id, rider_id, opened_at, closed_at,
                    expected_amount, collected_amount, deposited_amount,
                    variance, reconciled_by, note)
```

### 6.11 Tax & audit â€” Docs 13, 17
```sql
invoices(id, tenant_id, branch_id, order_id, invoice_no, fiscal_invoice_no,
         fbr_status, fbr_request JSONB, fbr_response JSONB, fbr_qr_payload,
         issued_at, pdf_object_key, retry_count, created_at, updated_at)
tax_categories(id, tenant_id, name, rate NUMERIC(5,2), fbr_code, is_exempt)
audit_log(id, tenant_id, actor_id, actor_type, entity_type, entity_id,
          action, before JSONB, after JSONB, reason, ip, occurred_at)
  PARTITION BY RANGE (occurred_at)
```

## 7. RLS pattern â€” apply to every tenant-scoped table

```sql
ALTER TABLE {t} ENABLE ROW LEVEL SECURITY;
CREATE POLICY {t}_tenant_isolation ON {t}
    USING (tenant_id = current_setting('app.tenant_id', true)::uuid);
```

`crates/db` sets `app.tenant_id` on every checkout from the pool:
```rust
sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
    .bind(ctx.tenant_id.0.to_string()).execute(&mut *conn).await?;
```
`true` scopes it to the transaction. Repositories **also** include `AND tenant_id = $n` explicitly. Two independent layers.

## 8. Acceptance tests

- `migrate_up_and_down_is_clean` â€” full migration run against a fresh container
- `every_tenant_table_has_rls` â€” queries `pg_policies`, asserts a policy exists for every table with a `tenant_id` column. **This test must fail if a later spec adds a table without RLS.**
- `every_fk_has_index` â€” introspects `pg_constraint` vs `pg_indexes`
- `rls_blocks_cross_tenant_read` â€” insert as tenant A, set `app.tenant_id` to B, assert zero rows
- `money_column_types` â€” asserts no money column is `float4`/`float8`
- `seed_generator_runs` â€” produces the documented volumes without error

## 9. Done checklist

- [ ] `cargo build --workspace` succeeds with all crates stubbed
- [ ] `docker compose up -d` brings up Postgres, Redis, NATS, MinIO
- [ ] `sqlx migrate run` applies cleanly to an empty database
- [ ] All six acceptance tests pass
- [ ] `cargo sqlx prepare --workspace` run, `.sqlx/` committed
- [ ] `.env.example` committed with every required variable, dummy values
- [ ] Seed generator produces 1 tenant / 8 branches / 5,000 products / 50 users
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

