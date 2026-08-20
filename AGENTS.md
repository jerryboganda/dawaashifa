# AGENTS.md — Dawaa Platform

Universal agent instructions. Read by Antigravity, GitHub Copilot, Claude Code, Codex, Cursor.
Tool-specific files (`CLAUDE.md`, `GEMINI.md`, `.github/copilot-instructions.md`) defer to this file. **This file wins on any conflict.**

**Roles:** Antigravity is the **builder** and owns the whole codebase. GitHub Copilot (Grok 4.6) is the **reviewer** and owns no code — it reviews diffs and commits mechanical fixes. Full protocol: `docs/19_BUILDER_REVIEWER_PROTOCOL.md`.

---

## 1. WHAT THIS IS

WhatsApp-first multi-branch pharmacy and healthcare commerce platform for Pakistan. Customers message on WhatsApp; AI assists; a licensed pharmacist approves; riders deliver; cash or gateway payment.

Full architecture: `docs/00_MASTER_ARCHITECTURE_BLUEPRINT.md`. Read it before your first task in this repo.

---

## 2. NON-NEGOTIABLE INVARIANTS

Violating any of these is a defect even if every test passes.

- **I-1** Every table has `tenant_id UUID NOT NULL`. No exceptions, including lookup tables.
- **I-2** Postgres Row-Level Security enabled on every tenant-scoped table.
- **I-3** No prescription order is ever auto-approved. A `pharmacist_approvals` row with a real `user_id` must exist before state advances past `RX_UNDER_REVIEW`.
- **I-4** No payment is auto-approved from a screenshot. Gateway webhooks may auto-confirm; screenshots always require human approval.
- **I-5** Stock is an append-only ledger. Never `UPDATE quantity`. Insert a `stock_movements` row.
- **I-6** AI output never reaches a customer unmodified in Rx flows. A pharmacist sees it first.
- **I-7** No raw SQL outside `repository` modules.
- **I-8** Money is `NUMERIC(14,4)`, `rust_decimal::Decimal` in Rust, `string` over the wire. **Never `f64`. Never JS `number`.**
- **I-9** Every state transition writes an `audit_log` row: actor, before, after, reason.
- **I-10** Business logic never branches on which WhatsApp transport is in use.

---

## 3. HOW WORK IS ASSIGNED

Work is defined by numbered spec documents in `docs/`. Each spec is one branch, one PR.

**Before writing any code:**
1. Open the spec doc you were asked to build (e.g. `docs/05_CATALOG_AND_MATCHING.md`).
2. Read its **Depends on** list. If a dependency spec is not merged, stop and say so.
3. Read its **Out of scope** section. Building out-of-scope items is a defect.
4. Follow its **Contracts** section exactly. Do not improve on the schema or API shape.

**Never work across two spec docs in one branch.**

If a spec is ambiguous, do not guess. Write your assumption at the top of the PR description under `## ASSUMPTIONS` and continue. Do not silently invent behaviour.

---

## 4. REPO LAYOUT

```
crates/          Rust backend (modular monolith)
  core/          domain types, money, IDs, tenant context, errors
  db/            migrations, repositories, RLS
  identity/ catalog/ inventory/ channel/ conversation/
  ai/ prescription/ orders/ payments/ fulfilment/ b2b/ tax/ admin/
  api/           Axum router, auth middleware, OpenAPI emit
  worker/        NATS consumers, schedulers
apps/
  console/       SvelteKit ops console
  rider/         SvelteKit PWA
  web/           Astro marketing site
  shared/        generated API client — DO NOT HAND-EDIT
sidecars/
  wa-unofficial/ Node 22 + Baileys
docs/            numbered specs 00-18
contracts/       openapi.json (generated), events.md
```

---

## 5. THE API CONTRACT — READ THIS TWICE

Backend and frontend are built by **different agents that do not share context.** The contract is therefore a generated artifact, never a described one.

**When changing routes or DTOs:**
- Annotate every Axum handler with `utoipa` derive macros.
- Run `cargo run -p api --bin emit-openapi` after any route or DTO change. This regenerates `contracts/openapi.json`.
- **Commit `contracts/openapi.json` in the same PR as the route change.** A route change without a regenerated spec is an incomplete PR.

**When consuming the API from `apps/`:**
- Run `pnpm gen:api` to regenerate `apps/shared/src/api/` from `contracts/openapi.json`.
- **Never hand-write a request type, response type, or endpoint URL.** Import from `@dawaa/shared`.
- If a field you need is missing, add it on the backend side and regenerate — never create a local type as a workaround.

CI fails the build if `contracts/openapi.json` is stale relative to the Rust source, or if `apps/shared/src/api/` is stale relative to `contracts/openapi.json`.

---

## 6. SAFETY — AUTO-CONTINUE GUARDRAILS

Agents here run long unattended chains. These require an explicit human stop:

- **Never** run `DROP`, `TRUNCATE`, or destructive `ALTER` against any database.
- **Never** edit a migration that is already merged to `main`. Migrations are append-only. Write a new one.
- **Never** commit secrets. Config comes from env vars. `.env` is gitignored; `.env.example` is committed with dummy values.
- **Never** call a real payment gateway, real FBR endpoint, or real WhatsApp number from tests. Use the mocks in `crates/*/tests/mocks/`.
- **Never** disable, skip, or `#[ignore]` a failing test to make CI pass. Fix it or report it.
- **Never** weaken an invariant in §2 to make a test pass.
- **Never** `git push --force` to `main`, and never merge your own PR.
- If you have retried the same failure three times, stop and report. Do not keep going.

---

## 7. COMMANDS

```bash
# backend
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
sqlx migrate run                      # apply migrations
cargo sqlx prepare --workspace        # refresh offline query cache — REQUIRED before commit
cargo run -p api --bin emit-openapi   # regenerate contracts/openapi.json

# frontend
pnpm install
pnpm gen:api                          # regenerate typed client from openapi.json
pnpm -F console dev
pnpm -F console build
pnpm -F console check                 # svelte-check, must be clean
pnpm lint
pnpm test

# full stack local
docker compose up -d                  # postgres, redis, nats, minio
```

---

## 8. CODE STANDARDS

**Rust**
- `cargo clippy -- -D warnings` must pass. Warnings are errors.
- No `unwrap()` or `expect()` outside tests and `main()`. Use `?` with `thiserror` domain errors.
- All DB queries via `sqlx::query_as!` (compile-time checked). Run `cargo sqlx prepare` before committing.
- Public functions on module boundaries get doc comments. Internal ones do not need them.
- Domain errors per crate with `thiserror`; `api` maps them to HTTP via a single `IntoResponse` impl.

**TypeScript / Svelte**
- `strict: true`. **No `any`.** Use `unknown` and narrow.
- Svelte 5 runes (`$state`, `$derived`, `$effect`). Not the old `export let` API.
- Tailwind for styling. No inline `style=` attributes, no CSS-in-JS.
- Every list rendering a server collection handles loading, empty, and error states. All three.
- Money arrives as a string. Format for display; never parse into a JS `number` for arithmetic.

**SQL**
- One migration per logical change, timestamp-prefixed, forward-only.
- Every foreign key gets an index. Every `tenant_id` column gets an index.
- `NOT NULL` by default; nullable requires a comment explaining why.

---

## 9. TESTING

- **State machines, money, tax, reconciliation: tests written before implementation.** No exceptions.
- Repository tests run against real Postgres via `testcontainers`, not mocks.
- Every API endpoint gets at least one happy-path and one auth-failure test.
- Every invariant in §2 that can be tested has a test asserting it is enforced.
- Frontend: component tests with Vitest, critical flows with Playwright.

---

## 10. DEFINITION OF DONE

A PR is done when all of these are true:

- [ ] Every item in the spec's own **Done checklist** is ticked
- [ ] Nothing from the spec's **Out of scope** was built
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` green
- [ ] `pnpm check` and `pnpm lint` clean (if frontend touched)
- [ ] `cargo sqlx prepare` run and `.sqlx/` committed (if queries changed)
- [ ] `contracts/openapi.json` regenerated and committed (if routes changed)
- [ ] New tables have `tenant_id`, RLS policy, and indexes
- [ ] Migrations are new files, not edits to existing ones
- [ ] PR description lists what was built, what was skipped, and any `## ASSUMPTIONS`
- [ ] `REVIEW_BRIEF.md` produced via `/prepare-review`
- [ ] Reviewer reports zero BLOCKER and zero HIGH; acceptance tests found == expected

---

## 11. LANGUAGE AND DOMAIN NOTES

- Customer-facing strings: English, Urdu (اردو), and Roman Urdu. All three. Never hardcode a user-facing string in a component — use the i18n catalogue.
- Urdu is right-to-left. Every customer-facing layout must work in RTL.
- Currency is PKR. Format `Rs 1,250.00`.
- Phone numbers are E.164 (`+923001234567`). Store canonical, display local.
- Dates display in `Asia/Karachi`. Store UTC. Always.
- "Rx" means prescription-only. "OTC" means over the counter. A product's `is_prescription_only` flag decides which order flow applies — never infer it from category.
