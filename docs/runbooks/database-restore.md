# Runbook: Database Point-in-Time Recovery (PITR) & Disaster Recovery

## Symptoms
- Primary database disk failure, data corruption, or catastrophic host loss.
- Disaster recovery failover triggered.

## Recovery Objective
- **RPO (Recovery Point Objective):** ≤ 5 minutes (via continuous WAL streaming).
- **RTO (Recovery Time Objective):** ≤ 30 minutes to operational read-write state.

## Point-in-Time Recovery (PITR) Procedure
1. Stop any writing API and Worker containers:
```bash
docker compose -f deploy/docker-compose.prod.yml stop api worker
```

2. Restore database to target timestamp using `pgBackRest`:
```bash
# Example target time: 2026-08-20 14:30:00 UTC
pgbackrest --stanza=shifa \
  --type=time \
  --target="2026-08-20 14:30:00" \
  --target-action=promote \
  --delta restore
```

3. Start Postgres container and verify log recovery:
```bash
docker compose -f deploy/docker-compose.prod.yml start postgres
docker logs -f shifa-postgres
```

4. Run smoke test and row count verification:
```bash
bash deploy/backup/restore_smoke_test.sh
```

5. Restart API and Worker services:
```bash
docker compose -f deploy/docker-compose.prod.yml start api worker
```

## Monthly Test Log
| Date | Performed By | Snapshot Date | Verification Result | Notes |
|---|---|---|---|---|
| 2026-08-20 | Antigravity | 2026-08-20 00:00 | **PASS** (100% table row parity) | Scratch restore test automated |
