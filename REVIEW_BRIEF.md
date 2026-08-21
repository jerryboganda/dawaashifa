# Review Brief — Spec 17: Deployment, Observability, Backup & DR

**Branch:** `feat/17-deployment`  
**Spec Reference:** `docs/17_DEPLOYMENT_AND_OBSERVABILITY.md`  
**Builder:** Antigravity  
**Domain Configuration:** `dawaa.polytronx.com`  

---

## 1. Executive Summary
Spec 17 completes the production infrastructure, observability pipeline, disaster recovery, alert matrix, health endpoints, and zero-downtime deployment automation for the Shifa Platform. Production domain routing is configured for `dawaa.polytronx.com` across the Caddy reverse proxy, ops console, backend API, and background workers.

---

## 2. Invariant Compliance Checklist
- [x] **I-1 (Tenant Scoping):** All database interactions scoped by `tenant_id UUID NOT NULL`.
- [x] **I-2 (Row-Level Security):** RLS enforced on all tables.
- [x] **I-7 (Repository Isolation):** All SQL queries encapsulated in repository modules.
- [x] **I-8 (Money Invariant):** Zero floating point arithmetic, monetary values formatted in PKR string representation.
- [x] **I-10 (Transport Agnostic):** WhatsApp business logic decoupled from transport infrastructure.
- [x] **Health Check Invariant (Doc 17 §13):** `/health` and `/api/v1/health` report granular statuses of Postgres, Redis, NATS, MinIO, AI Host, and FBR gateway.
- [x] **Contract Drift Invariant (Doc 17 §6):** OpenAPI specification regenerated and committed synchronously with `@shifa/shared` types.

---

## 3. What Was Built
1. **Production Infrastructure (`deploy/`)**:
   - `deploy/docker-compose.prod.yml`: Postgres 17 (pgvector, pg_trgm), Redis 7 AOF, NATS JetStream, MinIO, API, Worker, Baileys sidecar, Caddy, OTel Collector, Prometheus, Grafana, Loki, Tempo.
   - `deploy/Caddyfile`: Reverse proxy, automated TLS, security headers, rate limiting, and domain routing for `dawaa.polytronx.com` (and `api.dawaa.polytronx.com`, `ops.dawaa.polytronx.com`, `monitoring.dawaa.polytronx.com`).
   - `deploy/Dockerfile.api` & `deploy/Dockerfile.worker`: Multi-stage non-root hardened container builds.
   - `deploy/prometheus/alerts.yml`: All 12 production alerts per Doc 17 §8 table.
   - `deploy/otel/otel-collector.yml` & `deploy/otel/tempo.yaml`: Distributed telemetry tracing and log shipping.
   - `deploy/backup/pgbackrest.conf` & `deploy/backup/restore_smoke_test.sh`: PITR configuration and automated monthly smoke test.

2. **Runbooks (`docs/runbooks/`)**:
   - `fbr-outage.md`: Provisional invoice emission and exponential retry queue recovery.
   - `ai-host-down.md`: Circuit breaker open state and deterministic pharmacist fallback.
   - `database-restore.md`: Point-in-time recovery procedure and verification test log.
   - `payment-gateway-outage.md`: Dynamic failover and manual screenshot verification.
   - `incident-template.md`: SEV1/SEV2 post-mortem template.
   - `deployment.md`: Zero-downtime rolling deployment and additive migration rules.
   - `number-ban-response.md`: WhatsApp cold reserve number migration.
   - `data-migration.md`: Legacy pharmacy system ingestion and verification.

3. **Backend Health Probes (`crates/api/src/routes/health.rs`)**:
   - Added `DependencyHealth` & `SystemHealthResponse` structs with `utoipa` schemas.
   - Registered `/health` and `/api/v1/health` in Axum router and OpenAPI registry.

4. **Acceptance Test Suite (`crates/api/tests/deployment_acceptance_tests.rs`)**:
   - 5 comprehensive automated tests covering health responses, security port shielding, PII prevention, runbook completeness, and alert rule coverage.

---

## 4. Verification Evidence
- `cargo fmt --all --check`: **PASS** (0 formatting differences)
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (0 warnings, 0 errors)
- `cargo test --workspace`: **PASS** (100% test pass across all 15 crates)
- `pnpm -r test`: **PASS** (all frontend suites in `apps/console` and `apps/rider` pass green)
- `pnpm -r check`: **PASS** (0 TypeScript errors)
- `pnpm -r lint`: **PASS** (0 lint errors)
- `contracts/openapi.json`: Emitted and synchronized with `apps/shared/src/api/schema.d.ts`.
