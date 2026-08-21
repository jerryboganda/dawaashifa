# GitHub Actions GHCR Zero-Build Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Offload all Docker compilation and build compute to GitHub Actions runners using GitHub Container Registry (GHCR), ensuring the production VPS and local machines perform 0% compilation.

**Architecture:** GitHub Actions runs Docker Buildx in parallel with GHA layer caching on push to `main`, pushes 6 production container images to `ghcr.io/jerryboganda/dawaashifa/*`, and SSHs to the VPS to pull images and execute zero-build updates (`docker compose pull && docker compose up -d --no-build`).

**Tech Stack:** GitHub Actions, Docker Buildx, GHCR (`ghcr.io`), Docker Compose, SvelteKit, Rust (Axum, SQLx).

---

### Task 1: Configure Production Docker Compose for GHCR Images

**Files:**
- Modify: `deploy/docker-compose.prod.yml`

**Interfaces:**
- Consumes: Image tags pointing to `ghcr.io/jerryboganda/dawaashifa/<service>:${IMAGE_TAG:-latest}`
- Produces: Clean container definitions for web, console, rider, api, worker, wa-unofficial with no runtime VPS build step.

- [ ] **Step 1: Update service image definitions in `deploy/docker-compose.prod.yml`**
  Set image paths for all 6 application services to `ghcr.io/jerryboganda/dawaashifa/<service>:${IMAGE_TAG:-latest}` while retaining local build configuration block for optional local testing.

- [ ] **Step 2: Validate compose file syntax**
  Run: `docker compose -f deploy/docker-compose.prod.yml config`
  Expected: Clean YAML output without errors.

---

### Task 2: Implement Multi-Image Build & Push GitHub Actions Pipeline

**Files:**
- Modify: `.github/workflows/deploy.yml`

**Interfaces:**
- Consumes: Push events on `main` or manual `workflow_dispatch`
- Produces: Pre-built OCI container images in `ghcr.io/jerryboganda/dawaashifa/*` tagged with `latest` and `sha-<SHA>`, and triggers VPS pull.

- [ ] **Step 1: Define matrix build job for all 6 containers**
  Configure `docker/setup-buildx-action@v3`, `docker/login-action@v3` with `ghcr.io`, and `docker/build-push-action@v6` with GHA caching.

- [ ] **Step 2: Define zero-build deployment job**
  Connect to VPS, pull updated images with `docker compose pull`, and restart with `docker compose up -d --no-build --remove-orphans`.

---

### Task 3: Update Deployment Script and Documentation

**Files:**
- Modify: `deploy/deploy.sh`
- Modify: `DEPLOYMENT_GUIDE.md`

**Interfaces:**
- Consumes: Zero-build workflow commands
- Produces: Clear, accurate one-command deployment documentation and scripts.

- [ ] **Step 1: Update `deploy/deploy.sh` to run `docker compose pull` then `docker compose up -d --no-build`**
- [ ] **Step 2: Update `DEPLOYMENT_GUIDE.md` with the new zero-build GHCR architecture overview**

---

### Task 4: Full Validation & Dry-Run

**Files:**
- Verify: `.github/workflows/deploy.yml`
- Verify: `deploy/docker-compose.prod.yml`

- [ ] **Step 1: Test docker-compose syntax parsing**
- [ ] **Step 2: Verify workflow YAML syntax validity**
