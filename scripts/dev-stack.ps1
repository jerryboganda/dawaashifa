<#
.SYNOPSIS
  Docker-free local development stack for Windows.

.DESCRIPTION
  Starts Postgres (already installed via Scoop), NATS, MinIO and Redis as
  native Windows processes. No container runtime required.

  Postgres  - existing Scoop install, port 5432
  NATS      - nats-server.exe, ports 4222 / 8222
  MinIO     - minio.exe, ports 9000 / 9001
  Redis     - Memurai (Windows-native Redis), port 6379

.EXAMPLE
  .\scripts\dev-stack.ps1 up
  .\scripts\dev-stack.ps1 status
  .\scripts\dev-stack.ps1 down
  .\scripts\dev-stack.ps1 install     # one-time: fetch NATS + MinIO via Scoop
  .\scripts\dev-stack.ps1 reset-db    # drop and recreate the test database
#>

param(
    [Parameter(Position = 0)]
    [ValidateSet('up', 'down', 'status', 'install', 'reset-db', 'check')]
    [string]$Command = 'status'
)

$ErrorActionPreference = 'Stop'

$Root      = Split-Path -Parent $PSScriptRoot
$DataDir   = Join-Path $Root '.devstack'
$NatsData  = Join-Path $DataDir 'nats'
$MinioData = Join-Path $DataDir 'minio'
$LogDir    = Join-Path $DataDir 'logs'

$TestDb    = 'shifa_test'
$DevDb     = 'shifa_dev'
$PgUser    = 'postgres'

function Write-Step($msg)  { Write-Host "  -> $msg" -ForegroundColor Cyan }
function Write-Ok($msg)    { Write-Host "  OK  $msg" -ForegroundColor Green }
function Write-Warn2($msg) { Write-Host "  !   $msg" -ForegroundColor Yellow }
function Write-Err($msg)   { Write-Host "  X   $msg" -ForegroundColor Red }

function Test-Port($port) {
    $c = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
    return $null -ne $c
}

function Test-Command($name) {
    return $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

# ---------------------------------------------------------------- install ---
function Invoke-Install {
    Write-Host "`nOne-time dependency install`n" -ForegroundColor White

    if (-not (Test-Command 'scoop')) {
        Write-Err "Scoop not found. Install from https://scoop.sh then re-run."
        exit 1
    }

    Write-Step "Adding Scoop 'main' bucket (idempotent)"
    scoop bucket add main 2>$null | Out-Null

    foreach ($pkg in @('nats-server', 'minio')) {
        if (Test-Command $pkg) {
            Write-Ok "$pkg already present"
        } else {
            Write-Step "Installing $pkg"
            scoop install $pkg
        }
    }

    if (Test-Port 6379) {
        Write-Ok "Redis already listening on 6379"
    } else {
        Write-Warn2 "Redis not detected."
        Write-Host "      Install Memurai Developer (free, Windows-native Redis):"
        Write-Host "      https://www.memurai.com/get-memurai"
        Write-Host "      Redis is not needed until Doc 07 - safe to defer."
    }

    Write-Host "`nDone. Run: .\scripts\dev-stack.ps1 up`n"
}

# ----------------------------------------------------------------- checks ---
function Invoke-Check {
    Write-Host "`nEnvironment check`n" -ForegroundColor White
    $fail = $false

    # Postgres
    if (Test-Command 'psql') {
        $v = (psql --version) -replace '[^\d\.]', ''
        if ($v -match '^18') { Write-Ok "Postgres $v" }
        else { Write-Warn2 "Postgres $v - project targets 18.x"; $fail = $true }
    } else {
        Write-Err "psql not on PATH"; $fail = $true
    }

    # Required extensions must be AVAILABLE (not necessarily installed)
    if (Test-Port 5432) {
        $required = @('pgcrypto', 'pg_trgm', 'postgis')
        foreach ($ext in $required) {
            $q = "SELECT 1 FROM pg_available_extensions WHERE name='$ext';"
            $r = psql -U $PgUser -d postgres -tAc $q 2>$null
            if ($r -eq '1') { Write-Ok "extension available: $ext" }
            else { Write-Err "extension MISSING: $ext"; $fail = $true }
        }

        # pgvector is deliberately NOT required locally - Doc 05 only
        $q = "SELECT 1 FROM pg_available_extensions WHERE name='vector';"
        $r = psql -U $PgUser -d postgres -tAc $q 2>$null
        if ($r -eq '1') { Write-Ok "extension available: vector (bonus)" }
        else { Write-Warn2 "vector not available - expected on Windows. Doc 05 migrations are gated; CI covers them." }
    } else {
        Write-Err "Postgres not listening on 5432"; $fail = $true
    }

    foreach ($t in @(@('nats-server', 4222), @('minio', 9000))) {
        if (Test-Command $t[0]) { Write-Ok "$($t[0]) installed" }
        else { Write-Warn2 "$($t[0]) missing - run: .\scripts\dev-stack.ps1 install" }
    }

    if (Test-Command 'cargo')      { Write-Ok "cargo" }      else { Write-Err "cargo missing"; $fail = $true }
    if (Test-Command 'sqlx')       { Write-Ok "sqlx-cli" }   else { Write-Warn2 "sqlx-cli missing: cargo install sqlx-cli --no-default-features --features postgres" }
    if (Test-Command 'pnpm')       { Write-Ok "pnpm" }       else { Write-Warn2 "pnpm missing" }

    Write-Host ""
    if ($fail) { Write-Err "Environment incomplete."; exit 1 }
    else { Write-Ok "Environment ready.`n" }
}

# --------------------------------------------------------------------- up ---
function Invoke-Up {
    Write-Host "`nStarting local stack`n" -ForegroundColor White
    New-Item -ItemType Directory -Force -Path $NatsData, $MinioData, $LogDir | Out-Null

    # Postgres
    if (Test-Port 5432) {
        Write-Ok "Postgres already running on 5432"
    } else {
        Write-Step "Starting Postgres"
        try { Start-Service postgresql* -ErrorAction Stop; Write-Ok "Postgres started" }
        catch { Write-Err "Could not start Postgres. Start it manually, then re-run." ; exit 1 }
    }

    # Databases
    foreach ($db in @($DevDb, $TestDb)) {
        $exists = psql -U $PgUser -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='$db';" 2>$null
        if ($exists -ne '1') {
            Write-Step "Creating database $db"
            psql -U $PgUser -d postgres -c "CREATE DATABASE $db;" | Out-Null
        }
        foreach ($ext in @('pgcrypto', 'pg_trgm', 'postgis')) {
            psql -U $PgUser -d $db -c "CREATE EXTENSION IF NOT EXISTS $ext;" 2>$null | Out-Null
        }
        Write-Ok "database ready: $db"
    }

    # NATS
    if (Test-Port 4222) {
        Write-Ok "NATS already running on 4222"
    } elseif (Test-Command 'nats-server') {
        Write-Step "Starting NATS with JetStream"
        Start-Process nats-server `
            -ArgumentList "--jetstream","--store_dir","$NatsData","--http_port","8222" `
            -WindowStyle Hidden `
            -RedirectStandardOutput (Join-Path $LogDir 'nats.log') `
            -RedirectStandardError  (Join-Path $LogDir 'nats.err.log')
        Start-Sleep -Seconds 2
        if (Test-Port 4222) { Write-Ok "NATS started (monitor: http://localhost:8222)" }
        else { Write-Err "NATS failed - see $LogDir\nats.err.log" }
    } else {
        Write-Warn2 "nats-server not installed - skipping (needed from Doc 07)"
    }

    # MinIO
    if (Test-Port 9000) {
        Write-Ok "MinIO already running on 9000"
    } elseif (Test-Command 'minio') {
        Write-Step "Starting MinIO"
        $env:MINIO_ROOT_USER     = 'minioadmin'
        $env:MINIO_ROOT_PASSWORD = 'minioadmin'
        Start-Process minio `
            -ArgumentList "server","$MinioData","--console-address",":9001" `
            -WindowStyle Hidden `
            -RedirectStandardOutput (Join-Path $LogDir 'minio.log') `
            -RedirectStandardError  (Join-Path $LogDir 'minio.err.log')
        Start-Sleep -Seconds 2
        if (Test-Port 9000) { Write-Ok "MinIO started (console: http://localhost:9001)" }
        else { Write-Err "MinIO failed - see $LogDir\minio.err.log" }
    } else {
        Write-Warn2 "minio not installed - skipping (needed from Doc 09)"
    }

    if (Test-Port 6379) { Write-Ok "Redis running on 6379" }
    else { Write-Warn2 "Redis not running (needed from Doc 07)" }

    Write-Host "`nDATABASE_URL for tests:" -ForegroundColor White
    Write-Host "  postgres://$PgUser@127.0.0.1:5432/$TestDb`n"
}

# ------------------------------------------------------------------- down ---
function Invoke-Down {
    Write-Host "`nStopping local stack`n" -ForegroundColor White
    foreach ($p in @('nats-server', 'minio')) {
        $proc = Get-Process $p -ErrorAction SilentlyContinue
        if ($proc) { $proc | Stop-Process -Force; Write-Ok "stopped $p" }
        else { Write-Ok "$p not running" }
    }
    Write-Warn2 "Postgres and Redis left running (services). Stop manually if needed.`n"
}

# ----------------------------------------------------------------- status ---
function Invoke-Status {
    Write-Host "`nStack status`n" -ForegroundColor White
    $svcs = @(
        @{ n = 'Postgres'; p = 5432 },
        @{ n = 'Redis   '; p = 6379 },
        @{ n = 'NATS    '; p = 4222 },
        @{ n = 'MinIO   '; p = 9000 }
    )
    foreach ($s in $svcs) {
        if (Test-Port $s.p) { Write-Host "  UP    $($s.n)  :$($s.p)" -ForegroundColor Green }
        else                { Write-Host "  DOWN  $($s.n)  :$($s.p)" -ForegroundColor DarkGray }
    }
    Write-Host ""
}

# --------------------------------------------------------------- reset-db ---
function Invoke-ResetDb {
    Write-Host "`nResetting $TestDb`n" -ForegroundColor White
    psql -U $PgUser -d postgres -c "DROP DATABASE IF EXISTS $TestDb WITH (FORCE);" | Out-Null
    psql -U $PgUser -d postgres -c "CREATE DATABASE $TestDb;" | Out-Null
    foreach ($ext in @('pgcrypto', 'pg_trgm', 'postgis')) {
        psql -U $PgUser -d $TestDb -c "CREATE EXTENSION IF NOT EXISTS $ext;" 2>$null | Out-Null
    }
    Write-Ok "recreated $TestDb"
    $env:DATABASE_URL = "postgres://$PgUser@127.0.0.1:5432/$TestDb"
    Write-Step "Running migrations"
    sqlx migrate run
    Write-Ok "migrations applied`n"
}

switch ($Command) {
    'install'  { Invoke-Install }
    'check'    { Invoke-Check }
    'up'       { Invoke-Up }
    'down'     { Invoke-Down }
    'status'   { Invoke-Status }
    'reset-db' { Invoke-ResetDb }
}
