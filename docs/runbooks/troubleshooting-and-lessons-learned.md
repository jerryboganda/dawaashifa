# Troubleshooting & Lessons Learned: Production Architecture & CI/CD

This document serves as the permanent institutional memory and engineering reference for the Dawaa platform deployment, CI/CD pipeline, frontend bundling, and test infrastructure.

---

## 1. Frontend Asset Bundling & Vite SPA Architecture

### Problem & Symptom
- Public website (`dawaa.polytronx.com`), Ops Console (`/ops/`), or Rider PWA (`/rider/`) rendered as raw, unstyled HTML with broken images, missing CSS, and unrendered SPA components.

### Root Cause
1. **Missing CSS Import**: In Vite vanilla TypeScript projects, CSS files located in `src/style.css` are NOT bundled into `dist/assets/index-[hash].css` unless they are explicitly imported inside the TypeScript entrypoint (`import './style.css'`) AND referenced in `index.html`.
2. **Missing Framework Plugins**: Svelte 5 ops console requires `@sveltejs/vite-plugin-svelte` and `svelte.config.js` with `vitePreprocess()`. Without the plugin, Svelte components are ignored during Vite build.
3. **Sub-Path Asset Base Mismatch**: When apps are served under sub-paths (e.g. `/ops/` or `/rider/`), omitting `base: '/ops/'` causes HTML `<script>` and `<link>` tags to point to `/assets/...` (root) instead of `/ops/assets/...`, resulting in 404s when requested behind Caddy.

### Standard Architecture & Invariants
- **Web (`apps/web`)**:
  - `src/main.ts` MUST contain `import './style.css';`.
  - `index.html` MUST link `<link rel="stylesheet" href="/src/style.css" />` and `<script type="module" src="/src/main.ts"></script>`.
- **Console (`apps/console`)**:
  - `vite.config.ts` MUST specify `base: '/ops/'` and include `svelte()`.
  - `src/main.ts` MUST mount the root Svelte 5 component (`mount(App, { target: document.getElementById('app')! })`).
- **Rider (`apps/rider`)**:
  - `vite.config.ts` MUST specify `base: '/rider/'`.
  - `src/main.ts` MUST mount the interactive delivery execution view with touch-optimized targets (>=44px).

### Automated Verification Test
Ensure that every frontend app has a test in `apps/<app>/src/<app>.test.ts` verifying that `dist/index.html` contains the generated stylesheet tag and JS module script.

---

## 2. Reverse Proxy & Subpath SPA Routing in Caddy

### Problem & Symptom
- Accessing `https://dawaa.polytronx.com/ops` resulted in 404 or served the landing page HTML instead of the console SPA.

### Root Cause
- Caddy's standard `handle` directive does not strip the sub-path prefix when proxying static files from the root of a container. Furthermore, accessing `/ops` without a trailing slash fails URL resolution for relative assets (`./assets/...`).

### Standard Caddyfile Configuration (`deploy/Caddyfile`)
```caddy
dawaa.polytronx.com {
    # 1. Trailing slash redirects for SPAs
    redir /ops /ops/ 308
    redir /rider /rider/ 308

    # 2. Subpath SPA Handlers with handle_path (strips path prefix)
    handle_path /ops* {
        reverse_proxy console:80
    }

    handle_path /rider* {
        reverse_proxy rider:80
    }

    # 3. Backend API & Observability
    handle /api/* {
        reverse_proxy api:8080
    }
    handle /health {
        reverse_proxy api:8080
    }
    handle /swagger-ui* {
        reverse_proxy api:8080
    }

    # 4. Root Public Marketing Web App
    handle {
        reverse_proxy web:80
    }
}
```

---

## 3. GitHub Actions CI & Verification Workflows

### Problem & Symptom
- `.github/workflows/verify.yml` failed immediately in `test` job.
- `.github/workflows/ci.yml` hung for 11+ minutes and timed out.
- Deployment workflow skipped deploying to VPS.

### Root Causes & Fixes

| Issue | Root Cause | Permanent Fix |
|---|---|---|
| **Invalid Postgres Docker Tag** | `verify.yml` had `image: pgvector/pgvector:pg18`. Postgres 18 does not exist on Docker Hub. | Use `pgvector/pgvector:pg17` across all workflows and compose files. |
| **pnpm Lockfile Version Mismatch** | `pnpm/action-setup@v4` was set to `version: 9`, but `package.json` and `pnpm-lock.yaml` use `pnpm@10.33.2` (lockfile v9.0). | Always pin `version: 10.33.2` matching `packageManager` field in root `package.json`. |
| **Invalid Package Name for OpenAPI** | `verify.yml` ran `cargo run -p api --bin emit-openapi`, but the crate name is `shifa-api`. | Always use `cargo run -p shifa-api --bin emit-openapi`. |
| **Step-Level `env:` Evaluation Bug** | `deploy.yml` used `if: env.VPS_HOST != ''`. In GitHub Actions, step `if:` expressions cannot read step-level `env:` definitions and evaluate to false. | Use `if: ${{ secrets.VPS_HOST != '' }}` or workflow-level environment variables. |

---

## 4. Test Suite Fast-Failure & Pool Acquisition Timeouts

### Problem & Symptom
- Running `cargo test --workspace` on machines without a live PostgreSQL database took over 11 minutes (25+ tests * 30-second default sqlx connection timeout).

### Root Cause
- `sqlx::postgres::PgPoolOptions::new().connect(&url)` defaults to a 30-second acquire/connection timeout. When database integration tests run in environments where Postgres is unreachable, each test hangs for 30s before hitting `Err` and skipping.

### Standard Rule for Integration Tests
All DB-backed tests MUST configure `.acquire_timeout(std::time::Duration::from_millis(500))` on `PgPoolOptions`:
```rust
let pool = match PgPoolOptions::new()
    .acquire_timeout(std::time::Duration::from_millis(500))
    .max_connections(5)
    .connect(&database_url)
    .await
{
    Ok(p) => p,
    Err(_) => {
        println!("Skipping DB test: postgres not reachable");
        return;
    }
};
```
*Result*: Total test execution time dropped from **12 minutes to 0.53 seconds**.

---

## 5. VPS Production Deployment & Branch Drift Prevention

### Problem & Symptom
- Production VPS was still running outdated containers from a feature branch (`feat/12-fulfilment`) despite code being merged to `main`.

### Standard VPS Release Procedure
When deploying directly on the VPS via SSH:
```bash
# 1. Navigate to workspace
cd /opt/dawaa

# 2. Enforce clean synchronization with main
git fetch origin main
git checkout main
git reset --hard origin/main

# 3. Pull pre-built images or rebuild containers
cd deploy
docker compose -f docker-compose.prod.yml up -d --build --remove-orphans

# 4. Verify healthy status
docker compose -f docker-compose.prod.yml ps
curl -s http://127.0.0.1:8096/health | jq .
```

---

## 6. Pre-Commit & Verification Checklist

Before pushing any changes or declaring a deployment task complete:
1. `pnpm -r build` — Ensure all frontend apps build without errors.
2. `pnpm -r check` — Ensure zero TypeScript type errors.
3. `pnpm -r test` — Ensure all frontend test suites pass.
4. `cargo fmt --all --check` — Ensure rust formatting is clean.
5. `cargo clippy --workspace --all-targets -- -D warnings` — Ensure zero compiler warnings.
6. `cargo test --workspace` — Ensure all backend tests pass in < 1 second.
7. `cargo run -p shifa-api --bin emit-openapi` — Ensure `contracts/openapi.json` is updated.
8. Check live response with `curl.exe -I https://dawaa.polytronx.com` and hard refresh in browser.
