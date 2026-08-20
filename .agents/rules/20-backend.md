---
activation: Glob
glob: "{crates,migrations,sidecars}/**/*.{rs,sql,ts,js,toml}"
description: Rust, SQL and sidecar conventions for the Dawaa backend. Applies when working in crates, migrations or sidecars.
---

# Backend conventions

## Hard rules
- No `unwrap()` / `expect()` outside tests and `main()`. Use `?` with `thiserror` domain errors.
- No `f64` for money. `rust_decimal::Decimal` in Rust, `NUMERIC(14,4)` in SQL, `String` over the wire.
- No raw SQL outside `*/repository/*.rs`.
- No `axum` imports in domain crates. Only `crates/api` knows HTTP exists.
- Every repository function takes `&TenantContext` first and filters by `tenant_id` **in the SQL**, not only via RLS.
- IDs are newtypes (`OrderId`, `TenantId`), never bare `Uuid`.

## Tenant context — the highest-risk area
`tenant_id` comes from JWT claims only. Never from a request body, path, query, or header.

```rust
async fn handler(ctx: TenantContext, Json(req): Json<CreateOrder>) -> ...
```
If `CreateOrder` contains a `tenant_id` field, that is a security defect. Delete it.

## Query pattern
```rust
pub async fn find_by_id(
    ctx: &TenantContext, pool: &PgPool, id: OrderId,
) -> Result<Option<Order>, OrderError> {
    let row = sqlx::query_as!(
        OrderRow,
        r#"SELECT id, tenant_id, branch_id, status AS "status: OrderStatus", total
           FROM orders WHERE id = $1 AND tenant_id = $2"#,
        id.0, ctx.tenant_id.0,
    ).fetch_optional(pool).await?;
    Ok(row.map(Into::into))
}
```
Run `cargo sqlx prepare --workspace` and commit `.sqlx/` whenever queries change. CI fails without it.

## State transitions
Exhaustive `match` returning `Result`. Illegal transitions return `Err(InvalidTransition)` — never panic, never silently no-op.

Every successful transition writes the status change, the domain event row, **and** `audit_log` in **one transaction**. If the audit write fails, the transition rolls back.

## Concurrency — think before writing
These are the patterns that pass tests and fail in production:
- Never generate a sequence number with `COUNT(*) + 1`. Use a Postgres sequence.
- Stock allocation, reservation, credit checks and cash totals are read-then-write. Wrap in a transaction with `SELECT ... FOR UPDATE`.
- Retry paths need idempotency keys: outbound sends, payment webhooks, rider submissions.
- Release and rollback operations must be idempotent — running twice must not double-apply.

## Migrations
- Filename `{unix_timestamp}_{snake_case}.sql`. **Forward-only — never edit a merged migration.**
- Every tenant-scoped table, in the same file: `tenant_id UUID NOT NULL`, `created_at`, `updated_at`, `ENABLE ROW LEVEL SECURITY`, the isolation policy, index on `tenant_id`, index on every FK.
- Money `NUMERIC(14,4)`. Never float.
- Regulatory data (customers, orders, prescriptions, invoices, audit) uses `deleted_at`, never hard delete.
- `CREATE INDEX CONCURRENTLY` on tables that will be large.

## External calls
Explicit timeout, jittered backoff, circuit breaker. On open circuit, enqueue for human handling — never drop the work, never block indefinitely.

## OpenAPI obligation
Annotate handlers with `utoipa`; derive `ToSchema` on DTOs. After any route, DTO or enum change:
```bash
cargo run -p api --bin emit-openapi
pnpm gen:api
```
Commit both `contracts/openapi.json` and the regenerated `apps/shared/src/api/` in the same PR. CI fails on drift in either direction.

Money fields: `#[schema(value_type = String, example = "1250.00")]`. Enums serialise `SCREAMING_SNAKE_CASE`.

## Testing
- `testcontainers` for Postgres. Repository tests hit a real database.
- State machines, money, tax and reconciliation: **test first**.
- Every endpoint: happy path, 401/403, and a cross-tenant test proving tenant A cannot read tenant B's row.
- Mock every external service. No test touches a real WhatsApp number, gateway, FBR endpoint or the GPU host.
- **Never weaken an existing test to make new code pass.** If an old test now fails, either the new code is wrong or the old test encoded a behaviour the spec changed — say which, in the PR.
