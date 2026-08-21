# 🚀 Shifa Platform Production Deployment Guide

## 1. Single Domain Architecture Overview

The entire platform runs under the single domain: **`dawaa.polytronx.com`**.

| Service | Public URL | Description |
|---|---|---|
| **Public Pharmacy Portal** | `https://dawaa.polytronx.com/` | Customer catalog, prescription intake, order tracking |
| **Pharmacist / Ops Console** | `https://dawaa.polytronx.com/ops` | SvelteKit ops console for inbox, rx reviews, payments |
| **Rider Delivery PWA** | `https://dawaa.polytronx.com/rider` | Rider dispatch and GPS delivery interface |
| **Backend REST API** | `https://dawaa.polytronx.com/api/v1/*` | High-performance Axum Rust API |
| **WhatsApp Webhooks** | `https://dawaa.polytronx.com/webhooks/*` | Inbound WhatsApp Meta Cloud API & Baileys webhooks |
| **OpenAPI / Swagger** | `https://dawaa.polytronx.com/swagger-ui` | Live interactive API documentation |
| **System Health Probe** | `https://dawaa.polytronx.com/health` | Uptime & load balancer health checks |
| **Grafana Monitoring** | `https://dawaa.polytronx.com/monitoring` | Real-time Prometheus metrics, traces, and logs |

---

## 2. DNS Setup (Cloudflare / Registrar)

Only **1 DNS record** is required:

- **Type**: `A`
- **Name**: `dawaa`
- **IPv4 Address**: `<YOUR_VPS_PUBLIC_IP>`
- **Proxy status**: `DNS only` (or Proxied with Full/Strict SSL)

---

## 3. One-Command VPS Deployment

SSH into your VPS server:

```bash
# 1. Clone repository
git clone https://github.com/jerryboganda/dawaashifa.git
cd dawaashifa

# 2. Copy and customize production environment secrets
cp .env.example .env

# 3. Execute 1-command automated deployment
bash deploy/deploy.sh
```

---

## 4. Verification & Health Check

After running `deploy.sh`:

```bash
# Check container status
docker compose -f deploy/docker-compose.prod.yml ps

# Check API health endpoint
curl -i https://dawaa.polytronx.com/health

# View live Caddy logs
docker logs -f shifa-caddy
```
