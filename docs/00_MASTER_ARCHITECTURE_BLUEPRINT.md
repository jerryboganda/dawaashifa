# DAWAA PLATFORM — MASTER ARCHITECTURE BLUEPRINT
### Doc 00 of the Autonomous Development Kit
**WhatsApp-first multi-branch pharmacy & healthcare commerce platform — Pakistan**

> `Dawaa` (دوا) is a working codename. Rename before repo init; the name appears in ~40 places (crate names, DB name, container names, domains).

---

## 0. HOW TO USE THIS KIT

This document is **Doc 00**. It locks the decisions. Docs 01–18 are per-module build specs, each one executable by an agentic coding tool (Claude Code, Codex, Copilot Workspace, Antigravity) with **zero human clarification required**.

**Rules for every doc in this kit:**
- Contract-first: SQL migrations and OpenAPI spec are written **before** implementation code.
- Vertical slices, not horizontal layers. Each doc ships one working feature end-to-end.
- Every doc contains: in-scope, **explicitly out-of-scope**, exact file paths to create, acceptance tests, done-checklist.
- No doc may assume knowledge from another doc that isn't restated in its preamble. Agents lose context.

---

## 1. NON-NEGOTIABLE INVARIANTS

These go verbatim into `CLAUDE.md` / `AGENTS.md` at repo root. Any agent violating these has produced a defect regardless of whether tests pass.

| # | Invariant | Rationale |
|---|---|---|
| I-1 | Every table has `tenant_id UUID NOT NULL`. No exceptions, including lookup tables. | Single business today, multi-entity tomorrow. Retrofitting is a 6-month rewrite. |
| I-2 | Postgres Row-Level Security is enabled on every tenant-scoped table. | Defence in depth against a missing `WHERE` clause. |
| I-3 | **No prescription order is ever auto-approved.** A `pharmacist_approval` row must exist with a real `user_id` before state can advance past `RX_UNDER_REVIEW`. | DRAP requirement + patient safety. |
| I-4 | **No payment is ever auto-approved from a screenshot.** Gateway webhooks may auto-confirm; screenshots always require human approval. | Fraud control. |
| I-5 | Stock is an **append-only ledger**. Never `UPDATE quantity`. Insert a movement. | Batch/expiry audit, DRAP inspection, reconciliation. |
| I-6 | AI output never reaches a customer unmodified in Rx flows. It reaches a pharmacist first. | I-3 corollary. |
| I-7 | No raw SQL outside `repository` modules. All access through typed repositories. | Agent drift control. |
| I-8 | All money is `NUMERIC(14,4)` in PKR minor units logic — never `f64`. | Float money bugs are unrecoverable. |
| I-9 | Every state transition writes an `audit_log` row with actor, before, after, reason. | Regulatory + dispute resolution. |
| I-10 | Business logic must never branch on which WhatsApp transport is in use. | Channel abstraction integrity (§4). |

---

## 2. TECHNOLOGY STACK — LOCKED

| Layer | Choice | Notes |
|---|---|---|
| Core backend | **Rust + Axum** | Single binary, modular monolith |
| Async runtime | Tokio | |
| DB | **PostgreSQL 17** + `pgvector`, `pg_trgm`, `pg_partman` | |
| DB access | `sqlx` (compile-time checked queries) | Not an ORM. Agents write better raw-typed SQL than ORM chains. |
| Cache / locks | **Redis 7** | Sessions, rate limits, hot stock, idempotency keys |
| Message bus | **NATS JetStream** | Inbound queue, AI jobs, outbound send queue |
| Object storage | **MinIO** (self-hosted, S3 API) | Prescription images, voice notes, POD photos |
| Ops console | **SvelteKit** (SPA mode, TypeScript) | |
| Marketing site | **Astro** | Static, separate deploy |
| Rider app | **SvelteKit PWA** | Not native — see §11 |
| Unofficial WA sidecar | **Node 22 + Baileys** | Isolated container, see §4.3 |
| AI inference | **External — your GPU box, OpenAI-compatible HTTP** | Platform never loads a model in-process |
| Observability | OpenTelemetry → Grafana + Loki + Tempo | |
| Reverse proxy | Caddy | Auto-TLS |

### 2.1 Modular monolith — and why not microservices

You asked for limitless scale. Microservices would still be the wrong call *right now*:

- An agentic build across 11 repos fails on cross-service contract drift. One repo, one binary, one migration history is dramatically more buildable by AI agents.
- Rust + Axum on modest hardware handles tens of thousands of req/s. WhatsApp will rate-limit you long before Rust does.
- The module boundaries below are drawn so any module can be lifted into its own service later **without touching business logic** — each module talks to others only through its public `mod.rs` interface and NATS subjects.

Scale path when needed: read replicas → partition hot tables → extract `ai` and `conversation` into separate services → shard by `tenant_id`.

---

## 3. MODULE MAP

```
crates/
  core/           # domain types, errors, money, IDs, tenant context
  db/             # migrations, repositories, RLS setup
  identity/       # users, roles, RBAC, branches, sessions
  catalog/        # products, drug master, generics, aliases, MRP
  inventory/      # batches, expiry, cold chain, stock ledger, reservations
  channel/        # WhatsApp abstraction + adapters  ← §4
  conversation/   # threads, inbox, assignment, human override
  ai/             # LLM/VLM/STT gateway, language pipeline, confidence gating
  prescription/   # Rx intake, OCR result, pharmacist review & approval
  orders/         # cart, state machine, branch routing
  payments/       # gateways, screenshots, TID ledger, COD
  fulfilment/     # rider assignment, dispatch, POD, cash reconciliation
  b2b/            # quotes, hospital accounts, credit limits, AR aging
  tax/            # FBR POS invoicing, fiscal numbers, QR
  admin/          # settings, audit log, reports
  api/            # Axum router, auth middleware, OpenAPI
  worker/         # NATS consumers, schedulers
apps/
  console/        # SvelteKit ops console
  rider/          # SvelteKit PWA
  web/            # Astro marketing site
sidecars/
  wa-unofficial/  # Node + Baileys
```

---

## 4. CHANNEL ABSTRACTION — THE MOST IMPORTANT DESIGN DECISION

You want both the official Cloud API and unofficial automation (Baileys / whatsapp-web.js). That is architecturally fine **only** if the rest of the system cannot tell the difference.

### 4.1 The trait

```rust
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn id(&self) -> ChannelId;
    fn capabilities(&self) -> Capabilities;

    async fn send(&self, msg: OutboundMessage) -> Result<MessageReceipt>;
    async fn download_media(&self, ref_: MediaRef) -> Result<MediaBytes>;
    async fn health(&self) -> ChannelHealth;
}

pub struct Capabilities {
    pub interactive_buttons: bool,   // Cloud: true   Unofficial: degraded
    pub list_messages: bool,         // Cloud: true   Unofficial: false
    pub templates: bool,             // Cloud: true   Unofficial: false
    pub outside_24h_window: bool,    // Cloud: true   Unofficial: true (risky)
    pub delivery_receipts: bool,     // Cloud: true   Unofficial: unreliable
    pub max_send_rate_per_min: u32,  // Cloud: high   Unofficial: ~15, human-paced
}
```

### 4.2 Capability degradation (critical)

`OutboundMessage` is authored once, at the **richest** level. The adapter downgrades:

| Intent | Cloud API renders as | Unofficial renders as |
|---|---|---|
| Choose from 8 products | Interactive list message | Numbered text list, reply with a number |
| Confirm order | Reply buttons | "Reply YES to confirm" |
| Order dispatched (>24h later) | Utility template | Plain text |

The `conversation` module emits *intent*. Only the adapter knows how to render it. **Invariant I-10.**

### 4.3 Unofficial adapter — how it must be built

Baileys and whatsapp-web.js are Node-only. There is no Rust equivalent. Architecture:

- One **Docker container per phone number** running a thin Node service.
- Auth state persisted to **Postgres, not local disk** — container restarts must not require a QR rescan.
- Container ↔ Rust core over NATS only. Never HTTP-poll.
- Outbound send queue with **randomised human-like pacing** (typing indicator, 2–8s jitter, no bursts). Sending at machine speed is what triggers bans, more than volume.
- Ban detection: on `connection.update` with `loggedOut` / `403`, mark number `BANNED`, drain its queue, alert ops, promote the next number in the pool.

### 4.4 Number Pool Manager

```
numbers(id, tenant_id, msisdn, transport, status, session_ref, health_score,
        banned_at, last_seen_at, daily_sent_count, business_identity_id)
```

Status machine: `PROVISIONING → WARMING → ACTIVE → DEGRADED → BANNED → RETIRED`

New numbers go through a **warming period** — low volume for 7–14 days — before full traffic.

### 4.5 The risk you have not accounted for

You said you don't mind numbers being banned because you'll rotate. Three things that rotation does **not** solve:

1. **Your customers have the old number saved.** When it dies they message a dead number and get silence. You have no way to tell them the new one — that would itself require a working channel. Every ban silently amputates part of your customer base.
2. **Ban contagion.** If unofficial numbers are linked to the same Meta Business Manager / business identity as your official WABA, a ban can cascade and take out the paid channel too. **Mitigation is mandatory:** unofficial numbers live under a completely separate business identity, separate Business Manager, separate egress IPs, and are never added to the WABA. The `business_identity_id` column above exists to enforce this — the pool manager must refuse to place an unofficial number under the official identity.
3. **Rotation looks like spam.** Frequent number churn from one IP range accelerates future bans.

The cost avoided is roughly **PKR 2.79 per out-of-window utility message**. Build both adapters as specified — but route production traffic through Cloud API and keep the unofficial pool as a genuine fallback, not the default. That's a business call, not mine; the architecture supports either.

---

## 5. DATA MODEL — CORE TABLES

Full DDL is Doc 01. Shape here.

### 5.1 Tenancy & org
```
tenants(id, name, legal_name, ntn, strn, status)
branches(id, tenant_id, name, code, drap_licence_no, pharmacist_in_charge,
         address, geo POINT, service_radius_km, is_hub, cold_chain_capable)
users(id, tenant_id, phone, email, password_hash, status, locale)
roles(id, tenant_id, name, is_system)
permissions(id, key)              -- e.g. rx.approve, payment.approve
role_permissions / user_roles / user_branches
```

### 5.2 Catalog & drug master
```
products(id, tenant_id, sku, name_en, name_ur, form, strength, pack_size,
         manufacturer, drap_registration_no, is_prescription_only,
         is_controlled, mrp, requires_cold_chain, category_id, hs_code, pct_code)
generics(id, name, atc_code)
product_generics(product_id, generic_id, strength_mg)
generic_equivalents(generic_id, equivalent_generic_id, equivalence_type)
product_aliases(id, product_id, alias, alias_type, script, weight)
```

**`product_aliases` is the highest-leverage table in the system.** It maps every way a Pakistani customer might write a drug:
`Panadol` / `پیناڈول` / `pandol` / `panadal` / `panadole` / `paracetamol` → same SKU.
Seed it from your existing databases, then grow it automatically from every pharmacist correction (§8.4).

### 5.3 Inventory ledger
```
batches(id, tenant_id, product_id, branch_id, batch_no, expiry_date,
        cost_price, mrp_at_receipt, received_at, supplier_id, cold_chain_log_ref)
stock_movements(id, tenant_id, branch_id, product_id, batch_id, qty_delta,
                movement_type, ref_type, ref_id, occurred_at, actor_id)
   -- movement_type: RECEIPT | SALE | RETURN | TRANSFER_OUT | TRANSFER_IN
   --                ADJUSTMENT | EXPIRY_WRITEOFF | DAMAGE | RESERVATION | RELEASE
stock_current(tenant_id, branch_id, product_id, batch_id, qty)  -- maintained by trigger
```

Partition `stock_movements` monthly. Never delete.

### 5.4 Prescriptions
```
prescriptions(id, tenant_id, customer_id, image_object_key, source_channel,
              received_at, status, doctor_name, doctor_pmdc_no, issued_date)
rx_ocr_results(id, prescription_id, model_name, model_version, raw_output_json,
               confidence_overall, processed_at)
rx_lines(id, prescription_id, line_no, ocr_text, matched_product_id,
         match_confidence, match_method, qty, dosage_instructions,
         pharmacist_action, pharmacist_note)
   -- pharmacist_action: ACCEPTED | EDITED | REJECTED | ADDED_MANUALLY
pharmacist_approvals(id, prescription_id, user_id, decision, reason,
                     approved_at, ip, device)
```

The original image is **immutable**. You must be able to prove, years later, exactly what the pharmacist saw and what they changed. That is your legal defence.

### 5.5 Orders
```
orders(id, tenant_id, branch_id, customer_id, channel_id, prescription_id NULL,
       status, subtotal, discount, delivery_fee, tax_amount, total,
       payment_method, order_type)   -- order_type: RETAIL | B2B
order_items(id, order_id, product_id, batch_id, qty, unit_price, mrp_at_sale,
            line_total, is_prescription_only, substituted_from_product_id)
order_events(id, order_id, from_status, to_status, actor_id, reason, at)
```

---

## 6. ORDER STATE MACHINE

```
                          ┌── (no Rx items) ───────────────┐
DRAFT ──► CART_CONFIRMED ─┤                                ├─► AWAITING_PAYMENT
                          └─► AWAITING_RX ─► RX_UNDER_REVIEW ─┬─► RX_APPROVED ──┘
                                                              └─► RX_REJECTED ─► CANCELLED

AWAITING_PAYMENT ─┬─ (gateway webhook, verified) ──────────► CONFIRMED
                  ├─ (screenshot) ─► PAYMENT_UNDER_REVIEW ──► CONFIRMED | PAYMENT_REJECTED
                  └─ (COD selected) ─────────────────────────► CONFIRMED

CONFIRMED ─► PICKING ─► PACKED ─► DISPATCHED ─► OUT_FOR_DELIVERY
          ─► DELIVERED ─► CASH_RECONCILED (COD only) ─► CLOSED

Exits: CANCELLED | FAILED_DELIVERY ─► RETURNED ─► REFUNDED
```

Implement as an exhaustive Rust `enum` + `match`. Illegal transitions must not compile away silently — return `Err(InvalidTransition)`. Every transition writes `order_events` **and** `audit_log`.

---

## 7. BRANCH ROUTING (shared stock pool)

You chose a shared pool with nearest-branch routing. Algorithm:

1. Filter branches that hold **all** items in stock, with `expiry_date > today + safety_days`.
2. If cold-chain items present, filter to `cold_chain_capable = true`.
3. Rank by: full-fill possible → road distance to customer → current picking load → stock depth.
4. If no single branch can fill it: **split-fulfilment** (multiple branches, one customer-facing order) or **inter-branch transfer** if the delay is acceptable. Make this a configurable policy per tenant, not a hardcoded rule.
5. Reserve stock immediately on `CONFIRMED` via a `RESERVATION` movement with a TTL. Release on cancel/timeout.

Use PostGIS or `earthdistance` for the geo query. Cache branch stock summaries in Redis with 30s TTL.

---

## 8. AI LAYER

### 8.1 Gateway contract

Your models live on a separate GPU host. The platform speaks **OpenAI-compatible HTTP** and nothing else — so you can swap models, or fall back to a hosted provider, without touching code.

```
POST {AI_BASE_URL}/v1/chat/completions      # LLM
POST {AI_BASE_URL}/v1/audio/transcriptions  # STT (Whisper-compatible)
POST {AI_BASE_URL}/v1/chat/completions      # VLM (image content block)
POST {AI_BASE_URL}/v1/embeddings            # catalog vectors
```

Config-driven per task:
```toml
[ai.tasks.intent]      model = "qwen3-instruct"     timeout_ms = 4000
[ai.tasks.rx_ocr]      model = "qwen3-vl"           timeout_ms = 25000
[ai.tasks.stt]         model = "whisper-large-v3"   timeout_ms = 20000
[ai.tasks.embed]       model = "bge-m3"             timeout_ms = 3000
```

Circuit breaker per task. On open circuit → queue for human, never drop the customer.

### 8.2 Language pipeline (Urdu / Roman Urdu / English / code-mixed)

This is the hardest NLP problem in the build. Pipeline:

```
inbound text
  → script detect (Arabic block? Latin? mixed?)
  → if Latin: Roman-Urdu classifier (is this English or Roman Urdu?)
  → Roman-Urdu normaliser (rule-based: kh/x, ee/i, oo/u, aa/a, silent h,
     doubled consonants) → canonical form
  → intent + entity extraction (LLM, few-shot with Pakistani examples)
  → entity resolution against catalog (§8.3)
```

Roman Urdu has no standard orthography. `mujhe`/`mujay`/`mujhy`/`muje` are the same word. Do **not** rely on the LLM alone — the normaliser + alias table carry most of the weight.

### 8.3 Product matching — triple signal

Never single-signal. Score and combine:

| Signal | Implementation | Weight |
|---|---|---|
| Exact / alias hit | `product_aliases` lookup | 1.00 |
| Trigram similarity | `pg_trgm` on name + aliases | 0.40 |
| Phonetic | Double Metaphone tuned for Urdu-English transliteration | 0.35 |
| Vector | `pgvector` cosine on `bge-m3` embeddings | 0.25 |

Thresholds: `≥0.85` auto-suggest to customer (non-Rx only) · `0.55–0.85` show top 3 as a choice · `<0.55` escalate to human.

### 8.4 The learning loop — build this in from day one

Every pharmacist correction is labelled training data:

- Pharmacist edits `rx_line.ocr_text` "Augmantin 625" → product `Augmentin 625mg Tab`
- System writes a new row to `product_aliases` (alias `augmantin 625`, weight 0.9, source `PHARMACIST_CORRECTION`)
- Next time it's an exact hit

Within months this outperforms the base model on your specific handwriting population. **This is the single highest-ROI feature in the AI layer.** Do not defer it.

### 8.5 Substitution engine

Generic substitution is a **data lookup, not a generation task.** The LLM may only propose from `generic_equivalents`; it must never invent an equivalence. Every proposed substitution:
- carries the original product, the proposed one, and the equivalence type
- is flagged `requires_pharmacist_approval = true` always
- records whether the customer accepted

### 8.6 Confidence gating

Every AI output carries `confidence` and `escalation_reason`. Hard rules:
- Any Rx-related output → pharmacist queue, always, regardless of confidence.
- Non-Rx below threshold → branch manager queue.
- Circuit breaker open → human queue.
- **There is no path where low confidence results in silence to the customer.** Send an acknowledgement, queue the human.

---

## 9. PAYMENTS

### 9.1 Two paths, one ledger

**Path A — Gateway (trusted):** JazzCash, EasyPaisa, Safepay/PayFast aggregator, Raast.
- Signed server-side callback → verify HMAC → verify amount + order ref → auto-confirm.
- Never trust a client-side redirect as proof of payment.

**Path B — Screenshot (untrusted):** always human-approved (I-4).
```
payment_proofs(id, order_id, image_object_key, ocr_tid, ocr_amount,
               ocr_timestamp, ocr_sender, ocr_confidence,
               duplicate_of_proof_id, review_status, reviewed_by, reviewed_at)
transaction_id_ledger(tid, gateway, first_seen_order_id, tenant_id)  -- UNIQUE(tid)
```

Automated red flags surfaced to the reviewer (never auto-rejecting):
- TID already in `transaction_id_ledger` → **duplicate, highest severity**
- OCR amount ≠ order total
- Timestamp older than order creation, or > 48h old
- Image EXIF shows editing software
- Same sender account used across unrelated customer numbers

The reviewer sees the screenshot, the flags, and the order side by side. One click each way.

### 9.2 COD
- Rider collects cash → marks `DELIVERED` with amount collected.
- Daily per-rider reconciliation: expected vs collected vs deposited, variance flagged.
- `rider_cash_sessions(rider_id, opened_at, closed_at, expected, collected, deposited, variance)`

---

## 10. FBR / TAX

- Fiscal invoice generated at `CONFIRMED`, not at dispatch.
- Real-time POS reporting to FBR with retry queue — **an FBR outage must never block a sale.** Queue and reconcile.
- Invoice carries: fiscal invoice number, FBR QR code, branch STRN, HS/PCT codes per line.
- Medicines vs cosmetics vs devices carry different sales-tax treatment. Model tax rate **per product category**, never a global rate.
- Store the full FBR request/response payload against the invoice for audit.
- PDF invoice pushed to WhatsApp as a document message.

---

## 11. RIDER APP — PWA, not native

Deliberate choice for your ASAP timeline:
- No Play Store / App Store review loop. Ship in hours, not weeks.
- Works on cheap Android handsets riders actually carry.
- Offline-tolerant: IndexedDB queue, syncs when signal returns. Pakistani coverage demands this.
- Camera for POD photo, GPS for tracking, one-tap cash collection.

Reassess native only if you need background location beyond what a PWA allows.

---

## 12. OPS CONSOLE — KEY SCREENS

1. **Unified Inbox** — all branches, filter by branch/status/language, real-time via SSE. AI-drafted reply shown with an **Edit / Send / Override** control. Overrides are captured (§8.4).
2. **Rx Review Queue** — prescription image on the left, extracted lines on the right, per-line accept/edit/reject, one-tap approve. Optimise ruthlessly: this screen determines your throughput ceiling.
3. **Payment Review Queue** — screenshot, fraud flags, order, approve/reject.
4. **Order Board** — kanban by state, per branch.
5. **Inventory** — batch/expiry dashboard, expiring-in-90-days report, cold chain log.
6. **B2B Desk** — quotes, hospital accounts, credit limits, AR aging.
7. **Audit Explorer** — searchable, exportable. Your DRAP inspection answer.

---

## 13. BUILD SEQUENCE

Each phase ends in something that works in production.

| Phase | Contents | Outcome |
|---|---|---|
| **P0** | Docs 01–04. Repo, migrations, identity/RBAC, branches, Cloud API adapter, basic inbox. | Staff can chat with customers through the platform. |
| **P1** | Docs 05–06. Catalog, drug master, alias engine, inventory ledger, batch/expiry. | Real stock data in the system. |
| **P2** | Docs 07, 10. Conversation engine, cart, order state machine, branch routing, COD. | **Revenue loop closes — orders flow end to end, manually.** |
| **P3** | Docs 08–09. AI gateway, language pipeline, Rx OCR, pharmacist review. | AI assists; pharmacist approves. |
| **P4** | Doc 11. Gateways, screenshots, TID ledger. | Digital payments live. |
| **P5** | Docs 12–13. Rider PWA, dispatch, cash reconciliation, FBR POS. | Full fulfilment + compliance. |
| **P6** | Doc 03. Unofficial adapter + number pool. | Fallback channel ready. |
| **P7** | Doc 14. B2B implants module. | Hospital/surgeon channel. |
| **P8** | Docs 15–17. Data migration, console polish, observability, DR. | Hardened. |

Note P2: **the revenue loop closes before any AI is built.** If the AI layer slips, you are still trading.

---

## 14. THE DOCUMENT KIT

| Doc | Title |
|---|---|
| 00 | Master Architecture Blueprint *(this document)* |
| 01 | Domain Model, Full ERD & Migration Set |
| 02 | Channel Abstraction & Cloud API Adapter |
| 03 | Unofficial Adapter Sidecar & Number Pool Manager |
| 04 | Identity, RBAC, Branches & Session Management |
| 05 | Catalog, Drug Master & Product Matching Engine |
| 06 | Inventory Ledger, Batches, Expiry & Cold Chain |
| 07 | Conversation Engine, Inbox & Human Override |
| 08 | AI Orchestration — LLM/VLM/STT Gateway & Language Pipeline |
| 09 | Prescription Workflow & Pharmacist Approval |
| 10 | Orders, State Machine & Branch Routing |
| 11 | Payments — Gateways, Screenshots & COD |
| 12 | Fulfilment, Rider PWA & Cash Reconciliation |
| 13 | FBR POS Integration, Invoicing & Tax |
| 14 | B2B Module — Quotes, Credit & AR Aging |
| 15 | Data Migration Toolkit (SQL / Excel / POS imports) |
| 16 | Ops Console Specification & Design System |
| 17 | Deployment, Observability, Backup & DR |
| 18 | `CLAUDE.md` / `AGENTS.md` + Agent Execution Order & Prompt Pack |

---

## 15. AGENTIC DEVELOPMENT PROTOCOL

Rules that make this buildable by AI agents without supervision:

- **One doc, one branch, one PR.** Never let an agent work across two docs.
- **Migrations are append-only.** An agent may never edit a shipped migration.
- **Test-first for state machines and money.** Order transitions, tax, and reconciliation get tests written before implementation.
- **`cargo clippy -- -D warnings` in CI.** Agents produce warning-laden code; make it non-negotiable.
- **Golden-file tests for AI prompts.** Prompt changes must show their output diff.
- **Seed data is a first-class artifact.** A realistic seed (50 branches, 5,000 SKUs, 200 orders) so agents can test without your production data.
- **`AGENTS.md` restates §1 invariants at the top.** Agents read the first 200 lines most reliably.

---

## 16. OPEN ITEMS BEFORE DOC 01

1. Final product name (replaces `Dawaa`).
2. Branch count, SKU count, expected orders/day — sizes the partitioning strategy.
3. Sample export from each existing database format, so Doc 15 targets real schemas.
4. Confirm the GPU host's base URL and which model IDs are served.
5. Confirm your FBR integration tier and whether branches are already POS-integrated.
