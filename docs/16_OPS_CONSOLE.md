# DOC 16 — OPS CONSOLE SPECIFICATION & DESIGN SYSTEM

**Agent:** Frontend (Antigravity)
**Depends on:** Docs 02, 04, 05, 06, 07, 09, 10, 11 merged and `contracts/openapi.json` current
**Produces:** `apps/console`, `apps/shared`
**Branch:** `feat/16-ops-console` — split into sub-branches per screen group

> Run `pnpm gen:api` before starting and after every backend merge. Never hand-write an API type.

---

## 1. Objective

The console every branch manager, pharmacist, and operations lead lives in all day. **Two screens determine the throughput of the entire business**: the prescription review queue and the unified inbox. Optimise those ruthlessly; everything else can be conventional.

## 2. In scope

Shell and navigation · unified inbox · prescription review · payment review · order board · inventory · products · B2B desk · reports · settings · audit explorer · design system in `apps/shared`

## 3. Out of scope — do NOT build

- The rider PWA (Doc 12)
- The marketing site (Astro, separate)
- Any backend change — if data is missing, report it, do not work around it
- Custom charting; use a library

## 4. Design principles

**This is a tool, not a landing page.** Density over whitespace. Information over decoration.

- Staff use it for eight hours. Optimise for the hundredth use, not the first.
- Keyboard-first on high-volume screens. A pharmacist reviewing 200 prescriptions a day should never need the mouse.
- Never hide destructive or approval actions behind hover — touch devices exist, and hovering costs time.
- Show real data density: a table showing 8 rows on a 1080p screen is a wasted screen.
- Every screen states what it is doing when it is loading, empty, or broken.

## 5. Design tokens

```ts
// apps/shared/src/tokens.ts — mirrored in tailwind.config.ts
colors: {
  brand:   { 50…900 },        // teal family, matching existing brand
  surface: { base, raised, sunken, overlay },
  text:    { primary, secondary, muted, inverse },
  status:  { pending, review, approved, rejected, dispatched, delivered, failed },
  severity:{ critical, high, medium, low },
}
spacing: 4px base scale
radius:  { sm: 4, md: 6, lg: 10 }
```

Status colours are **semantic and shared**. An order in `PENDING_APPROVAL` uses the same colour on the order board, the inbox, and the payment queue. Inconsistent status colour is a defect.

Fonts: Inter for Latin, Noto Nastaliq Urdu for Urdu script. Load Urdu subset only where needed — it is a heavy font.

## 6. Screen: Unified Inbox — high priority

Three-pane: conversation list · message thread · context sidebar.

- Real-time via SSE (Doc 07), with a visible connection indicator; on drop, show reconnecting, then replay
- Filters: branch, status, assigned, unread, has-prescription, language
- Virtualised list — branches will have thousands of conversations
- Thread renders text, images, voice notes (inline player **with transcript shown**), documents, and location
- AI drafts render visually distinct with a confidence badge. Three actions: **Send · Edit · Discard**
- Editing a draft preserves the original for the training loop — do not silently replace it
- Context sidebar: customer detail, open orders, order history, last prescription, notes, tags
- Keyboard: `j`/`k` navigate, `r` reply, `e` edit draft, `a` assign, `Enter` send, `/` search

**Never build a bulk-send control that includes Rx-linked conversations.** Invariant I-6.

## 7. Screen: Prescription Review — highest priority

**This screen sets the ceiling on how many orders the business can process per day.** Treat every extra click as a real cost.

Split view: prescription image left, extracted lines right.

Image pane: zoom, pan, rotate, contrast toggle, fit-to-width. Handwriting is hard to read — these controls are functional necessities, not niceties.

Lines pane, per line:
- The raw OCR text as extracted
- Matched product with a confidence badge
- Top 3 alternative candidates, one click to switch
- Quantity and dosage fields, editable
- Actions: **Accept · Edit · Substitute · Reject**
- Low-confidence lines visually flagged and sorted to the top

Requirements:
- **No bulk-approve-all control.** Every line needs an explicit decision. The approve button stays disabled until all lines are decided, with a clear count of how many remain.
- Keyboard: `1`–`9` jump to line, `a` accept, `e` edit, `s` substitute, `x` reject, `Ctrl+Enter` submit
- Product search on edit uses the matching endpoint with debounced typeahead, showing stock status per branch
- Substitution requires a reason and shows a mandatory customer-notification prompt
- Controlled substances render with a prominent warning banner
- Show queue depth and the oldest waiting prescription in the header — pharmacists need to see the backlog

## 8. Screen: Payment Review

Three-pane: proof image · fraud flags · matching order.

- Flags render by severity with a plain-language explanation. `DUPLICATE_TID` is unmissable — full-width critical banner naming the earlier order.
- Order total and OCR amount shown side by side with any difference highlighted
- Approve and reject are one click each; reject requires a reason
- Queue sorted by severity, then age
- **No bulk approve.** Invariant I-4.

## 9. Remaining screens

**Order Board** — kanban by status, drag to transition (only legal transitions droppable), filters by branch/date/method, click for detail with a full event timeline.

**Inventory** — stock by branch with batch and expiry columns; expiry dashboard at 90/60/30 days with value at risk; receipt, adjustment and transfer forms; cold chain log with excursion alerts prominent.

**Products** — searchable table, edit form with MRP validation shown inline before save, alias manager with source and hit count, bulk import with dry-run report display.

**B2B Desk** — accounts, quote builder with live totals, AR aging with drill-through, consignment stock, device recall lookup.

**Reports** — sales by branch/product/period, AI usage and cost, override rate, SLA compliance, cash variance, tax report. All exportable to CSV and XLSX.

**Settings** — branches, users and roles, canned replies, templates, tax categories, AI thresholds, SLA config, split-fulfilment policy.

**Audit Explorer** — filter by actor, entity, action, date. Before/after diff view. Exportable. This is the screen you open during a DRAP inspection; make it fast and complete.

## 10. Technical requirements

- SvelteKit 2, Svelte 5 runes, TypeScript strict, Tailwind
- All API types from `@dawaa/shared` — generated, never hand-written
- Money is a string; format only, never arithmetic (invariant I-8)
- Three locales: `en`, `ur` (RTL), `ur-Latn`. Every string in the catalogue.
- Logical CSS properties throughout — `ms-*`, `text-start`, `border-e`
- Optimistic updates with rollback on failure for status transitions
- Virtualise any list that can exceed 100 rows
- Route-level code splitting; the inbox must not pull in B2B code

## 11. Acceptance tests

- `no_hand_written_api_types` — lint rule failing on local interfaces mirroring API shapes
- `no_money_arithmetic_in_browser` — source sweep for `Number(` on money fields
- `rx_review_approve_disabled_until_all_lines_decided`
- `no_bulk_approve_control_in_rx_review`
- `no_bulk_approve_control_in_payment_review`
- `rx_linked_conversation_excluded_from_bulk_send`
- `duplicate_tid_renders_critical_banner`
- `every_screen_renders_in_urdu_rtl`
- `status_colours_consistent_across_screens`
- `sse_reconnects_and_replays_after_drop`
- `virtualised_lists_render_10000_rows_smoothly`
- `keyboard_flow_completes_rx_review_without_mouse` — Playwright
- `order_board_rejects_illegal_transition_drop`
- `all_screens_handle_loading_empty_error`

## 12. Done checklist

- [ ] Design tokens shared between Tailwind config and `apps/shared`
- [ ] Inbox: SSE, virtualised, AI drafts with three actions, keyboard shortcuts
- [ ] Rx review: split view, per-line decisions, no bulk approve, keyboard-complete
- [ ] Payment review: severity-ranked flags, duplicate TID unmissable, no bulk approve
- [ ] All remaining screens per §9
- [ ] Three locales, RTL verified on every screen
- [ ] No hand-written API types, no browser money arithmetic
- [ ] All 14 acceptance tests green
- [ ] `pnpm check` and `pnpm lint` clean
