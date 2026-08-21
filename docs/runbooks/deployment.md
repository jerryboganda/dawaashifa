# Runbook: Zero-Downtime Zero-Build Deployment & Release Procedure

## 1. Deployment Architecture
- **Compute Offloading:** 100% of compilation and container building happens on GitHub Actions.
- **Image Registry:** GitHub Container Registry (`ghcr.io/jerryboganda/dawaashifa/*`).
- **Production VPS:** Runs zero builds; only pulls pre-built image layers and swaps containers in ~5–10 seconds.
- **Reverse Proxy:** Caddy reverse proxy balances traffic between services with automated TLS.
- **Database Migrations:** SQL migrations execute before container rollout and adhere to the zero-downtime rules.

---

## 2. Zero-Downtime Migration Invariants (Doc 17 §10)
1. **Additive Only:** New columns must be `NULL` or have defaults in the release they are introduced.
2. **Two-Release Column Deprecation:**
   - Release N: Stop writing to column.
   - Release N+1: Safely drop column.
3. **No Renames:** Add new column, backfill asynchronously in batches, switch reads, drop later.
4. **Concurrent Indexes:** All index creations on live tables must use `CREATE INDEX CONCURRENTLY`.

---

## 3. Standard Continuous Deployment Procedure (Automated via GitHub Actions)
When pushing to `main`:
1. GitHub Actions CI runs tests, lints, invariant checks, and OpenAPI contract drift verification.
2. GitHub Actions builds all 6 container images in parallel via Docker Buildx with layer caching and pushes to `ghcr.io`.
3. GitHub Actions SSHs into the VPS and executes:
   ```bash
   cd /opt/dawaa/deploy
   docker compose -f docker-compose.prod.yml pull
   docker compose -f docker-compose.prod.yml up -d --no-build --remove-orphans
   ```
4. Deployment completes in under 10 seconds with zero VPS CPU spikes.

---

## 4. Manual Zero-Build VPS Update Procedure
If executing manually on the VPS server:

```bash
cd /opt/dawaa
git pull origin main
cd deploy
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d --no-build --remove-orphans
```

---

## 5. Smoke Test & Health Check
```bash
# Check status of all containers
docker compose -f deploy/docker-compose.prod.yml ps

# Check API health endpoint
curl -f https://dawaa.polytronx.com/health | jq .
```

---

## 6. Emergency Rollback
If a release fails smoke testing, rollback instantly to a previous commit SHA tag without building:

```bash
cd /opt/dawaa/deploy
IMAGE_TAG=sha-<PREVIOUS_COMMIT_SHA> docker compose -f docker-compose.prod.yml pull
IMAGE_TAG=sha-<PREVIOUS_COMMIT_SHA> docker compose -f docker-compose.prod.yml up -d --no-build --remove-orphans
```
Since migrations are strictly additive-only, previous code versions remain compatible with the database schema.
