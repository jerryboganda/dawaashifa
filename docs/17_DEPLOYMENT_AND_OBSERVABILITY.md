# DOC 17 — DEPLOYMENT, OBSERVABILITY, BACKUP & DR

**Agent:** Backend (Copilot)
**Depends on:** all prior specs
**Produces:** `deploy/`, `.github/workflows/`, `docs/runbooks/`
**Branch:** `feat/17-deployment`

---

## 1. Objective

Get the platform running reliably on your own infrastructure, with enough observability to diagnose problems and enough backup discipline to survive a disk failure.

## 2. In scope

- Docker Compose production stack
- CI/CD pipeline with the contract-drift gate
- OpenTelemetry tracing, metrics, structured logging
- Health checks and alerting
- Backup and restore, tested
- Zero-downtime migration procedure
- Runbooks
- Security hardening

## 3. Out of scope — do NOT build

- Kubernetes (unnecessary at this scale; revisit past ~50 branches)
- Multi-region replication
- Custom monitoring UI — Grafana is the UI

---

## 4. A direct warning about hosting

The stated plan is a **shared VPS**. That is not adequate for this system, for three reasons:

1. **Patient prescription data.** Shared infrastructure means neighbouring workloads on the same host. For DRAP-regulated health data, that is a defensibility problem before it is a technical one.
2. **Resource contention.** Postgres with partitioned tables, Redis, NATS, MinIO, the Rust binary, and Node sidecars will compete with whatever else runs there. Your slowest day will be caused by someone else's traffic.
3. **No isolation on failure.** A neighbour's runaway process takes down your order intake.

Minimum recommendation: a dedicated VPS or bare-metal box for the platform, and the GPU host stays separate as already planned. Costs are modest relative to a multi-branch pharmacy's revenue.

Build the deployment to work either way — but record this in the runbook so the decision is explicit rather than accidental.

---

## 5. Production stack

```yaml
services:
  postgres:   # 17 + pgvector + pg_trgm + pg_partman
  redis:      # 7, appendonly yes
  nats:       # JetStream, file storage
  minio:      # object storage, versioning on
  api:        # Rust binary, N replicas behind Caddy
  worker:     # NATS consumers, scheduled jobs
  wa-sidecar: # one per unofficial number, profile-gated
  caddy:      # reverse proxy, automatic TLS
  otel-collector / grafana / loki / tempo / prometheus
```

Resource floors: Postgres 4 vCPU / 8GB, api 2 vCPU / 2GB per replica, everything else 1 vCPU / 1GB. Below these, expect problems under load.

## 6. CI pipeline

```
lint      → cargo fmt --check, clippy -D warnings, pnpm lint
test      → cargo test --workspace (testcontainers), pnpm test
contract  → emit-openapi, git diff --exit-code contracts/openapi.json
            pnpm gen:api, git diff --exit-code apps/shared/src/api/
security  → cargo audit, pnpm audit, gitleaks
build     → docker build, push to registry
deploy    → staging on merge to main; production is manual approval
```

**The `contract` job is what keeps two independent agents in sync.** If Copilot changes a route without regenerating the spec, or Antigravity's client is stale, the build fails there. Do not make it optional and do not allow it to be skipped.

## 7. Observability

**Tracing** — OpenTelemetry, propagated from webhook receipt through AI call, database write and outbound send. A single WhatsApp message must be traceable end to end. Trace ID surfaces in error responses so support can quote it.

**Metrics** that matter here:
```
wa_messages_inbound_total{transport,channel}
wa_messages_outbound_total{transport,status}
wa_send_latency_seconds
wa_channel_health{channel}
rx_queue_depth / rx_review_duration_seconds / rx_ocr_confidence
payment_proofs_pending / payment_fraud_flags_total{flag}
orders_by_status{status,branch}
stock_allocation_failures_total{reason}
ai_invocations_total{task,model,outcome} / ai_latency_seconds / ai_tokens_total
fbr_queue_depth / fbr_submission_failures_total
rider_cash_variance_total
```

**Logs** — structured JSON, correlated by trace ID. **Never log:** prescription image contents, payment screenshot contents, full customer messages (log message IDs), passwords, tokens, or gateway secrets. Log volume, not content.

## 8. Alerts

| Alert | Threshold | Severity |
|---|---|---|
| WhatsApp channel banned | any | **critical, page** |
| Rx queue depth | >50 for 15 min | high |
| Oldest Rx waiting | >2 hours | high |
| Payment proofs pending | >30 | medium |
| FBR queue depth | >100 or any item >6h | medium |
| AI circuit breaker open | any task, >5 min | high |
| Order confirmation error rate | >2% over 10 min | high |
| Stock allocation failures | >10/hour | medium |
| Postgres connections | >80% of pool | high |
| Disk usage | >80% | high |
| Backup failed | any | **critical** |
| Rider cash variance | >PKR 5,000/day | medium |

## 9. Backup and restore

- **Postgres:** WAL archiving plus nightly base backup via `pgBackRest`. PITR to any point in the last 30 days.
- **MinIO:** versioning on; nightly sync to a second location. Prescription and POD buckets have object-lock retention.
- **Redis:** AOF; not authoritative, rebuildable.
- **NATS:** JetStream file storage on persistent volume; in-flight work survives restart.
- **Retention:** 30 days daily, 12 months monthly, 7 years for prescriptions, invoices and audit log.

**Restore is tested monthly, not assumed.** A scheduled job restores the latest backup into a scratch environment and runs a smoke test. An untested backup is not a backup. Record each test in the runbook.

## 10. Zero-downtime migrations

1. Migrations are additive-only in the same release as the code that uses them
2. Column removal is a **two-release** process: stop writing in release N, drop in release N+1
3. Never rename a column — add the new one, backfill, switch reads, drop later
4. Long backfills run as a background job in chunks, never inside the migration
5. Index creation uses `CONCURRENTLY`

## 11. Security hardening

- TLS everywhere; Caddy handles certificates
- Postgres, Redis, NATS and MinIO bound to the internal network only, **never exposed publicly**
- Secrets from environment or a secrets manager, never in the repo, never in the database
- SSH key-only, no password auth, non-standard port, fail2ban
- Firewall default-deny inbound except 80/443
- **Rotate the IP that has been shared publicly**, or firewall it to known sources
- Rate limit all public endpoints; webhook endpoints validate signatures before any parsing
- Quarterly dependency audit; `cargo audit` and `pnpm audit` in CI on every build

## 12. Runbooks

`docs/runbooks/`: `number-ban-response.md`, `fbr-outage.md`, `ai-host-down.md`, `database-restore.md`, `data-migration.md`, `payment-gateway-outage.md`, `incident-template.md`, `deployment.md`

Each states: symptoms, immediate action, diagnosis, resolution, prevention. Written for someone at 3am who did not build the system.

## 13. Acceptance tests

- `contract_drift_fails_ci` — mutate a route without regenerating, assert failure
- `stale_frontend_client_fails_ci`
- `health_endpoint_reports_all_dependencies`
- `trace_propagates_webhook_to_outbound`
- `no_pii_in_logs` — sweep log output for message bodies and image data
- `backup_restore_smoke_test` — restore into scratch, verify row counts
- `migration_applies_with_service_running`
- `secrets_absent_from_image` — scan built image
- `internal_services_not_publicly_bound`

## 14. Done checklist

- [ ] Production compose stack with documented resource floors
- [ ] CI with a mandatory, non-skippable contract-drift gate
- [ ] OpenTelemetry tracing end to end; all listed metrics exported
- [ ] Logging with PII exclusions verified by test
- [ ] All 12 alerts configured with routing
- [ ] pgBackRest with PITR; MinIO versioning and object lock
- [ ] Monthly automated restore test, results recorded
- [ ] Zero-downtime migration procedure documented and demonstrated
- [ ] Security hardening applied; shared IP rotated or firewalled
- [ ] All eight runbooks written
- [ ] All nine acceptance tests green
