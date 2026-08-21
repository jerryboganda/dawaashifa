#!/usr/bin/env bash
set -euo pipefail

echo "======================================================================"
echo "  🚀 Starting Shifa Platform Zero-Build Production Deployment"
echo "  Target Domain: dawaa.polytronx.com"
echo "  Registry:      ghcr.io/jerryboganda/dawaashifa/*"
echo "======================================================================"

# 1. Check for .env file
if [ ! -f .env ]; then
  if [ -f .env.example ]; then
    echo "⚠️ .env not found. Creating from .env.example..."
    cp .env.example .env
  else
    echo "❌ Error: Neither .env nor .env.example found."
    exit 1
  fi
fi

# 2. Pull latest git code if in repository
if [ -d .git ]; then
  echo "📥 Pulling latest compose configs from origin main..."
  git pull origin main || true
fi

# 3. Pull pre-compiled images from GHCR (Zero compilation on VPS!)
echo "📦 Pulling pre-built production container images from GHCR..."
docker compose -f deploy/docker-compose.prod.yml pull

# 4. Launch all production containers
echo "🚀 Starting production containers (zero-build instant startup)..."
docker compose -f deploy/docker-compose.prod.yml up -d --no-build --remove-orphans

echo "⏳ Waiting for services to become healthy..."
sleep 5

# 5. Run database migrations
echo "🗄️ Applying database migrations in production..."
for migration_file in $(ls -1 migrations/*.sql 2>/dev/null | sort); do
  echo "  Applying $(basename "$migration_file")..."
  docker compose -f deploy/docker-compose.prod.yml exec -T postgres psql -U "${POSTGRES_USER:-shifa}" -d "${POSTGRES_DB:-shifa}" -f - < "$migration_file" || true
done

# 6. Verify running services
docker compose -f deploy/docker-compose.prod.yml ps

echo "======================================================================"
echo "  ✅ Zero-Build Deployment Complete! (Zero VPS CPU pressure)"
echo "  Public Portal:        https://dawaa.polytronx.com"
echo "  Pharmacist Console:   https://dawaa.polytronx.com/ops"
echo "  Rider Delivery PWA:   https://dawaa.polytronx.com/rider"
echo "  API & Health:         https://dawaa.polytronx.com/health"
echo "  Swagger / OpenAPI:    https://dawaa.polytronx.com/swagger-ui"
echo "  Grafana Observability:https://dawaa.polytronx.com/monitoring"
echo "======================================================================"
