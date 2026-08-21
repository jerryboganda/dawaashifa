@echo off
rem =================================================================================================
rem Dawaa Shifa Platform — Turnkey Windows Startup Script
rem Domain: dawaa.polytronx.com
rem =================================================================================================

echo =================================================================
echo   Starting Dawaa Shifa Platform on dawaa.polytronx.com
echo =================================================================

if not exist .env (
    if exist ..\.env.example (
        echo Creating .env from ..\.env.example...
        copy ..\.env.example .env
    )
)

echo Building and starting Docker services...
docker compose -f docker-compose.prod.yml up -d --build

echo =================================================================
echo   Dawaa Shifa Platform is LIVE!
echo =================================================================
echo   - Public Pharmacy Portal:   https://dawaa.polytronx.com/
echo   - Operations ^& Rx Console:  https://dawaa.polytronx.com/ops
echo   - Rider Delivery PWA:       https://dawaa.polytronx.com/rider
echo   - REST API ^& Health Probes: https://dawaa.polytronx.com/api
echo   - Interactive API Docs:     https://dawaa.polytronx.com/swagger-ui
echo   - Observability Monitoring: https://dawaa.polytronx.com/monitoring
echo =================================================================
pause
