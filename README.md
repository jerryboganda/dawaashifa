# Shifa Platform (دوا)

> WhatsApp-first multi-branch pharmacy and healthcare commerce platform for Pakistan.

## Architecture & Modules

The platform is structured as a high-performance modular monolith in Rust with SvelteKit applications for the operations console and delivery rider PWA:

```
crates/
  core/           # Strongly-typed domain IDs, Money (exact Decimal), TenantContext, errors
  db/             # PostgreSQL connection pool, RLS session helper, Repository trait
  identity/       # Users, RBAC roles, branches, sessions
  catalog/        # Products, drug master, generics, aliases, product matching
  inventory/      # Batches, expiry, cold chain, append-only stock movement ledger
  channel/        # WhatsApp channel abstraction, Cloud API & unofficial adapters
  conversation/   # Threads, omni-channel inbox, human override
  ai/             # External AI gateway (LLM/VLM/STT), Urdu/Roman-Urdu language pipeline
  prescription/   # Rx intake, OCR results, mandatory pharmacist review workflow
  orders/         # Cart, order state machine, nearest-branch routing
  payments/       # Gateways, screenshot verification, TID ledger, COD
  fulfilment/     # Rider assignment, dispatch, POD, cash reconciliation
  b2b/            # B2B quotes, hospital accounts, credit limits, AR aging
  tax/            # FBR POS digital invoicing, fiscal numbers, QR
  admin/          # System settings, audit log explorer, reports
  api/            # Axum HTTP router, auth middleware, OpenAPI spec generator
  worker/         # NATS consumers, scheduled maintenance jobs

apps/
  console/        # SvelteKit ops console (Inbox, Rx review, order board, inventory)
  rider/          # SvelteKit offline-tolerant delivery rider PWA
  web/            # Astro marketing site
  shared/         # Generated API client and shared schemas

contracts/        # contracts/openapi.json and API specifications
```

---

## Non-Negotiable Invariants

- **I-1**: Every table has `tenant_id UUID NOT NULL`.
- **I-2**: Postgres Row-Level Security (RLS) is enabled on every tenant-scoped table.
- **I-3**: **No prescription order is ever auto-approved.** A licensed pharmacist approval is mandatory.
- **I-4**: **No payment is auto-approved from a screenshot.** Screenshots require human verification.
- **I-5**: Stock is an **append-only ledger** (`stock_movements`). Never `UPDATE quantity`.
- **I-6**: AI output never reaches a customer unmodified in Rx flows.
- **I-7**: No raw SQL outside `repository` modules.
- **I-8**: All money is `NUMERIC(14,4)` in DB, `rust_decimal::Decimal` in Rust, `string` over the wire. **Never `f64` or `f32`.**
- **I-9**: Every state transition writes an `audit_log` row.
- **I-10**: Business logic never branches on which WhatsApp transport is in use.

---

## Local Development Prerequisites

- **Rust**: 1.80+ (stable toolchain)
- **Node.js**: 22+
- **pnpm**: 10+
- **Docker & Docker Compose**

---

## Quickstart

### 1. Configure Environment
```bash
cp .env.example .env
```

### 2. Start Local Infrastructure
Start PostgreSQL 17 (with pgvector), Redis 7, NATS JetStream, and MinIO:
```bash
docker compose up -d
docker compose ps
```

### 3. Build and Test Rust Workspace
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

### 4. Install Frontend Dependencies
```bash
pnpm install
pnpm lint
pnpm test
```

---

## Branching & Review Protocol

- Build tasks are driven by specifications in `docs/`.
- Review is performed against `docs/19_BUILDER_REVIEWER_PROTOCOL.md` and `.github/instructions/review.instructions.md`.

