#!/usr/bin/env bash
# =================================================================================================
# Dawaa Shifa Platform — Turnkey Production Startup Script
# Domain: dawaa.polytronx.com
# =================================================================================================
set -euo pipefail

echo "================================================================="
echo "  🚀 Starting Dawaa Shifa Platform on dawaa.polytronx.com"
echo "================================================================="

# 1. Ensure .env exists
if [ ! -f "../.env" ] && [ ! -f ".env" ]; then
  echo "⚠️  No .env file found. Creating .env from .env.example..."
  if [ -f "../.env.example" ]; then
    cp ../.env.example .env
  elif [ -f ".env.example" ]; then
    cp .env.example .env
  fi
fi

# 2. Build and launch all production containers
echo "==> Building and starting Docker services..."
docker compose -f docker-compose.prod.yml up -d --build

# 3. Wait for services to initialize
echo "==> Waiting for Caddy and API to become healthy..."
sleep 5

# 4. Print live health check status
echo "================================================================="
echo "  ✅ Dawaa Shifa Platform is LIVE!"
echo "================================================================="
echo "  • Public Pharmacy Portal:   https://dawaa.polytronx.com/"
echo "  • Operations & Rx Console:  https://dawaa.polytronx.com/ops"
echo "  • Rider Delivery PWA:       https://dawaa.polytronx.com/rider"
echo "  • REST API & Health Probes: https://dawaa.polytronx.com/api"
echo "  • Interactive API Docs:     https://dawaa.polytronx.com/swagger-ui"
echo "  • Observability Monitoring: https://dawaa.polytronx.com/monitoring"
echo "================================================================="
