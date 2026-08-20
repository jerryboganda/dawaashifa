# Dawaa Platform — Autonomous Development Kit

WhatsApp-first multi-branch pharmacy and healthcare commerce platform for Pakistan.

`Dawaa` (دوا) is a working codename. Rename before `git init` — it appears in crate names, database names, container names and package scopes.

---

## Start here

1. **`docs/19_BUILDER_REVIEWER_PROTOCOL.md`** — how the builder/reviewer loop runs. Read this first.
2. **`docs/18_AGENT_EXECUTION_ORDER_AND_PROMPTS.md`** — execution order and prompts.
3. **`docs/00_MASTER_ARCHITECTURE_BLUEPRINT.md`** — every locked decision and why.
4. **`AGENTS.md`** — the contract both agents obey.

Then commit this kit as your first commit and run the Doc 01 prompt.

---

## Agent configuration

**Antigravity builds. Grok 4.6 in Copilot reviews.** See `docs/19_BUILDER_REVIEWER_PROTOCOL.md`.

| File | Tool | Purpose |
|---|---|---|
| `AGENTS.md` | all | Source of truth — wins on any conflict |
| `.agents/rules/00-core-invariants.md` | Antigravity | Always On |
| `.agents/rules/10-frontend.md` | Antigravity | Glob, `apps/**` |
| `.agents/rules/20-backend.md` | Antigravity | Glob, `crates/**`, `migrations/**` |
| `.agents/workflows/execute-spec.md` | Antigravity | `/execute-spec {NN}` |
| `.agents/workflows/prepare-review.md` | Antigravity | `/prepare-review` → `REVIEW_BRIEF.md` |
| `.github/copilot-instructions.md` | Copilot | **Reviewer** config |
| `.github/instructions/review.instructions.md` | Copilot | The review rubric |
| `.github/workflows/copilot-setup-steps.yml` | Copilot | Toolchain so the reviewer can run tests |
| `GEMINI.md` | Antigravity | Pointer + tool notes |
| `CLAUDE.md` | Claude Code | Pointer + tool notes |
| `.cursor/rules/00-core.mdc` | Cursor | Pointer |

Antigravity caps rules files at 12,000 characters. `AGENTS.md` sits at ~8.7k with room to grow. Specs in `docs/` have no cap — they are opened per task, not loaded as rules.

---

## The specs

| Doc | Title | Agent |
|---|---|---|
| 00 | Master Architecture Blueprint | — |
| 01 | Domain Model, ERD & Migrations | Backend |
| 02 | Channel Abstraction & Cloud API | Backend |
| 03 | Unofficial Adapter & Number Pool | Backend |
| 04 | Identity, RBAC & Branches | Backend |
| 05 | Catalog, Drug Master & Matching | Backend |
| 06 | Inventory Ledger & Cold Chain | Backend |
| 07 | Conversation Engine & Inbox | Backend |
| 08 | AI Orchestration & Language Pipeline | Backend |
| 09 | Prescription Workflow & Approval | Backend |
| 10 | Orders, State Machine & Routing | Backend |
| 11 | Payments, Gateways & COD | Backend |
| 12 | Fulfilment & Rider PWA | Both |
| 13 | FBR, Tax & Invoicing | Backend |
| 14 | B2B Quotes, Credit & AR | Backend |
| 15 | Data Migration Toolkit | Backend |
| 16 | Ops Console & Design System | Frontend |
| 17 | Deployment & Observability | Backend |
| 18 | Agent Execution Order & Prompts | — |
| 19 | Builder / Reviewer Protocol | — |

---

## The ten invariants

1. Every table has `tenant_id`
2. RLS on every tenant-scoped table
3. **No prescription auto-approval, ever**
4. **No payment screenshot auto-approval, ever**
5. Stock is an append-only ledger
6. AI output reaches a pharmacist before a customer in Rx flows
7. No raw SQL outside repositories
8. Money is `NUMERIC(14,4)` / `Decimal` / string — never a float
9. Every state transition writes `audit_log`
10. Business logic never branches on WhatsApp transport

Invariants 3, 4 and 6 are patient-safety and fraud controls, not preferences. An agent proposing to relax them for convenience should be refused.

---

## Build phases

| Phase | Specs | Outcome |
|---|---|---|
| P0 | 01, 04, 02 | Staff chat with customers through the platform |
| P1 | 05, 06 | Real catalogue and stock |
| P2 | 07, 10 | **Revenue loop closes — orders flow end to end** |
| P3 | 08, 09 | AI assists; pharmacist approves |
| P4 | 11 | Digital payments |
| P5 | 12, 13 | Delivery and fiscal compliance |
| P6 | 03 | Fallback channel |
| P7 | 14 | B2B implants |
| P8 | 15, 16, 17 | Migration, console polish, hardening |

**P2 matters most.** The business can trade with no AI at all. Everything after is improvement, not viability.

---

## Open items

Answer these before Doc 01:

1. Final product name
2. Branch count, SKU count, orders/day — sizes table partitioning
3. One sample export per legacy format — blocks Doc 15 only
4. GPU host base URL and served model IDs
5. FBR tier and current branch POS integration status
