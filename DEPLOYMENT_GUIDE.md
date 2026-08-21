# 🚀 Shifa Platform Production Deployment Guide

## 1. Zero-Build & Cloud-Offloaded Architecture

All heavy compute workloads (Rust release compilation, SvelteKit bundling, Docker Buildx caching, test suites, invariant checking) are executed **100% on GitHub Actions runners**.

The production VPS performs **zero compilation** and **zero image builds**:
- Pre-built OCI container images are pushed directly to **GitHub Container Registry (`ghcr.io/jerryboganda/dawaashifa/*`)**.
- The VPS only pulls pre-built image layers and starts containers in ~5–10 seconds.
- Production CPU and RAM remain 100% available for PostgreSQL, Redis, NATS, and customer traffic.

### Single Domain Service Routing (`dawaa.polytronx.com`)

| Service | Container Image (`ghcr.io`) | Public URL | Description |
|---|---|---|---|
| **Public Pharmacy Portal** | `ghcr.io/jerryboganda/dawaashifa/web:latest` | `https://dawaa.polytronx.com/` | Customer catalog, prescription intake, order tracking |
| **Pharmacist / Ops Console** | `ghcr.io/jerryboganda/dawaashifa/console:latest` | `https://dawaa.polytronx.com/ops` | SvelteKit ops console for inbox, rx reviews, payments |
| **Rider Delivery PWA** | `ghcr.io/jerryboganda/dawaashifa/rider:latest` | `https://dawaa.polytronx.com/rider` | Rider dispatch and GPS delivery interface |
| **Backend REST API** | `ghcr.io/jerryboganda/dawaashifa/api:latest` | `https://dawaa.polytronx.com/api/v1/*` | High-performance Axum Rust API |
| **Background Worker** | `ghcr.io/jerryboganda/dawaashifa/worker:latest` | *Internal* | NATS stream consumers and scheduled jobs |
| **WhatsApp Sidecar** | `ghcr.io/jerryboganda/dawaashifa/wa-unofficial:latest` | `https://dawaa.polytronx.com/webhooks/*` | Inbound WhatsApp Baileys webhooks |
| **OpenAPI / Swagger** | *Embedded in API* | `https://dawaa.polytronx.com/swagger-ui` | Live interactive API documentation |
| **System Health Probe** | *Embedded in API* | `https://dawaa.polytronx.com/health` | Uptime & load balancer health checks |
| **Grafana Monitoring** | `grafana/grafana:10.3.3` | `https://dawaa.polytronx.com/monitoring` | Real-time Prometheus metrics, traces, and logs |

---

## 2. Automated Continuous Deployment (GitHub Actions)

When code is pushed to `main`:
1. **GitHub Actions** runs all verification, linting, and tests.
2. **GitHub Actions** builds all 6 containers using Docker Buildx and pushes them to `ghcr.io`.
3. **GitHub Actions** connects via SSH to the VPS and executes:
   ```bash
   docker compose -f docker-compose.prod.yml pull
   docker compose -f docker-compose.prod.yml up -d --no-build --remove-orphans
   ```
4. Deployment completes in **under 10 seconds with 0% CPU spike**.

---

## 3. Manual 1-Command VPS Deployment

If deploying or updating manually on the VPS:

```bash
# 1. Clone repository (first time only)
git clone https://github.com/jerryboganda/dawaashifa.git /opt/dawaa
cd /opt/dawaa

# 2. Copy and customize production environment secrets
cp .env.example deploy/.env

# 3. Execute zero-build deployment script
bash deploy/deploy.sh
```

---

## 4. Verification & Health Checks

```bash
# Check container status
docker compose -f deploy/docker-compose.prod.yml ps

# Check API health endpoint
curl -i https://dawaa.polytronx.com/health

# View live Caddy logs
docker logs -f shifa-caddy
```

---

## 5. Troubleshooting & Lessons Learned

For detailed analysis of past deployment issues (Vite CSS bundling, Caddy subpath routing, CI test timeouts, step-level GitHub Actions secrets, and fast-fail database pools), refer to the comprehensive guide:
- [`docs/runbooks/troubleshooting-and-lessons-learned.md`](docs/runbooks/troubleshooting-and-lessons-learned.md)

