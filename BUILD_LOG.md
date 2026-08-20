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
