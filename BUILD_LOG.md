# Dawaa / Shifa Platform Build Log

## Architecture Decision Record: PostgreSQL 18 & UUIDv7 Strategy
- **Date**: 2026-08-21
- **Status**: Accepted & Implemented
- **Context**: The platform generates UUIDv7 identifiers in Rust application code via `Uuid::now_v7()` to optimize B-tree indexing and guarantee chronological ordering. However, PostgreSQL 17 lacks native `uuidv7()` generation defaults and defaults to random v4 via `gen_random_uuid()`.
- **Decision**: Upgraded containerized infrastructure to PostgreSQL 18 (`pgvector/pgvector:pg18`) which provides native `uuidv7()` function support. All SQL migrations and table schemas standardize on `DEFAULT uuidv7()` for primary key columns.
- **Consequences**: Complete parity between application-generated and database-generated entity identifiers with time-ordered monotonic clustering for high-throughput write scalability.
