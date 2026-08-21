#!/usr/bin/env bash
set -euo pipefail

echo "======================================================================"
echo "  🚀 Starting Shifa Platform Production Deployment"
echo "  Target Domain: dawaa.polytronx.com"
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
  echo "📥 Pulling latest code from origin main..."
  git pull origin main || true
fi

# 3. Build and launch all production containers
echo "🏗️ Building and deploying production containers via Docker Compose..."
docker compose -f deploy/docker-compose.prod.yml up -d --build --remove-orphans

echo "⏳ Waiting for services to become healthy..."
sleep 5

# 4. Verify running services
docker compose -f deploy/docker-compose.prod.yml ps

echo "======================================================================"
echo "  ✅ Deployment Complete!"
echo "  Public Portal:        https://dawaa.polytronx.com"
echo "  Pharmacist Console:   https://dawaa.polytronx.com/ops"
echo "  Rider Delivery PWA:   https://dawaa.polytronx.com/rider"
echo "  API & Health:         https://dawaa.polytronx.com/health"
echo "  Swagger / OpenAPI:    https://dawaa.polytronx.com/swagger-ui"
echo "  Grafana Observability:https://dawaa.polytronx.com/monitoring"
echo "======================================================================"
