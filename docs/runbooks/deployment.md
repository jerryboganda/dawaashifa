# Runbook: Zero-Downtime Deployment & Release Procedure

## Deployment Architecture
- Caddy reverse proxy balances traffic between old and new API containers.
- Database migrations execute **before** container rollout and adhere to the zero-downtime rules.

## Zero-Downtime Migration Invariants (Doc 17 §10)
1. **Additive Only:** New columns must be `NULL` or have defaults in the release they are introduced.
2. **Two-Release Column Deprecation:**
   - Release N: Stop writing to column.
   - Release N+1: Safely drop column.
3. **No Renames:** Add new column, backfill asynchronously in batches, switch reads, drop later.
4. **Concurrent Indexes:** All index creations on live tables must use `CREATE INDEX CONCURRENTLY`.

## Standard Release Procedure
1. **Prepare Offline SQL Cache:**
```bash
cargo sqlx prepare --workspace
```
2. **Apply Migrations:**
```bash
sqlx migrate run
```
3. **Build & Deploy New Containers (Rolling Update):**
```bash
docker compose -f deploy/docker-compose.prod.yml build api worker
docker compose -f deploy/docker-compose.prod.yml up -d --no-deps --scale api=2 --no-recreate api
# Wait for health probe, then terminate old container
docker compose -f deploy/docker-compose.prod.yml up -d --no-deps api worker
```
4. **Smoke Test Health Endpoint:**
```bash
curl -f http://localhost:8080/api/v1/health | jq .
```

## Emergency Rollback
If release fails smoke test:
```bash
docker compose -f deploy/docker-compose.prod.yml rollback
```
Since migrations are additive-only, previous code versions remain compatible with the database schema.
