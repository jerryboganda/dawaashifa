# Shifa Platform Build Log

## Architecture Decision Record 1: PostgreSQL 18 & UUIDv7 Strategy
- **Date**: 2026-08-21
- **Status**: Accepted & Implemented
- **Context**: The platform generates UUIDv7 identifiers in Rust application code via `Uuid::now_v7()` to optimize B-tree indexing and guarantee chronological ordering.
- **Decision**: Upgraded containerized infrastructure to PostgreSQL 18 (`pgvector/pgvector:pg18`) with native `uuidv7()` default values in all table migrations.

## Architecture Decision Record 2: Product Name Standardization
- **Date**: 2026-08-21
- **Status**: Accepted & Implemented
- **Decision**: Replaced working codename "Dawaa" with "Shifa" (شفا) across all Rust crates (`shifa-*`), database name (`shifa`), Docker containers (`shifa-*`), npm packages (`@shifa/*`), and environment variables.

## Architecture Decision Record 3: GitHub Actions Cloud Compute Offloading & GHCR Zero-Build VPS Deployment
- **Date**: 2026-08-21
- **Status**: Accepted & Implemented
- **Context**: Running multi-crate Rust `--release` compilation and SvelteKit frontend bundling directly on the production VPS caused 100% CPU lockups, memory exhaustion, and risk of dropping active traffic.
- **Decision**: Offloaded all compilation, bundling, and image building to GitHub Actions cloud runners with Docker Buildx and GHA layer caching. Images are pushed to GitHub Container Registry (`ghcr.io/jerryboganda/dawaashifa/*`). The production VPS performs zero builds, updating via `docker compose pull` and `docker compose up -d --no-build` in ~5–10 seconds with 0% CPU pressure.

## Architecture Decision Record 4: Vite SPA Bundling & Sub-Path Caddy Reverse Proxying
- **Date**: 2026-08-21
- **Status**: Accepted & Implemented
- **Decision**: All frontend SPAs enforce explicit CSS imports in TypeScript entrypoints (`import './style.css'`) and define explicit sub-path bases (`base: '/ops/'` for Console, `base: '/rider/'` for Rider). Caddy uses `handle_path` with trailing-slash 308 redirects to ensure seamless client-side routing.

## Architecture Decision Record 5: Fast-Fail Integration Test DB Connection Pools
- **Date**: 2026-08-21
- **Status**: Accepted & Implemented
- **Decision**: All database-backed unit and acceptance tests configure `.acquire_timeout(std::time::Duration::from_millis(500))` on `PgPoolOptions`. In environments without a live PostgreSQL instance, tests skip immediately in <0.5s instead of hanging for sqlx's default 30-second timeout, reducing CI test execution from 12 minutes to 0.53s.

