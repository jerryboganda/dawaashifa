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
