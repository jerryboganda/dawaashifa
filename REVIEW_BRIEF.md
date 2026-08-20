# Review Brief — Doc 16: Ops Console Specification & Design System

## Spec
`docs/16_OPS_CONSOLE.md`

## What I built
- **Shared Design Tokens & Utilities (`apps/shared/src/`)**:
  - `tokens.ts`: Brand teal scale (50..900), surfaces, semantic status colours, severity levels, spacing scale, and radius tokens (Doc 16 §5).
  - `money.ts`: Strict PKR money formatting (`Rs 1,250.00`) and decimal validation. Prohibits JavaScript `Number` parsing on money (Invariant I-8).
  - `i18n.ts`: Multilingual catalogue with 3 locales (`en`, `ur` with RTL, and `ur-Latn`) covering all operations screens.
- **Ops Console Shell & High-Volume Screens (`apps/console/src/`)**:
  - **Unified WhatsApp Inbox (`src/routes/inbox/+page.svelte`, `state/inbox.ts`)**:
    - Three-pane layout: virtualised conversation list, message thread, customer context sidebar (Doc 16 §6).
    - Inline audio player with speech-to-text transcript rendering.
    - AI Drafts with confidence badges and 3 explicit actions: **Send · Edit · Discard** (preserving original draft for training loop).
    - SSE live reconnecting state and event replay on connection drop.
    - Invariant I-6: Rx-linked conversations excluded from bulk messaging actions.
    - Keyboard navigation: `j`/`k` navigation, `r` reply, `e` edit draft, `Enter` send.
  - **Prescription Review Desk (`src/routes/rx-review/+page.svelte`, `state/rx-review.ts`)**:
    - Split view: prescription image controls (zoom/rotate/contrast) and extracted medicine lines (Doc 16 §7).
    - Per-line actions: **Accept · Edit · Substitute · Reject** with top alternative candidates and confidence indicators.
    - Controlled substances warning banner.
    - Queue depth and oldest waiting prescription in header.
    - Invariant I-3 & Doc 16 §7: Zero bulk-approve control. Approve button is strictly disabled until all lines have an explicit decision.
    - Full keyboard navigation flow (`1..N` select line, `A` accept, `X` reject, `Ctrl+Enter` submit approval).
  - **Payment Proof Review (`src/routes/payments/review/+page.svelte`, `state/payments.ts`)**:
    - Three-pane review: screenshot proof, fraud & validation flags ranked by severity, matching order summary (Doc 16 §8).
    - `DUPLICATE_TID` full-width critical warning banner naming the earlier conflicting order.
    - Side-by-side comparison of order total money vs proof OCR extracted amount.
    - Invariant I-4: Zero bulk-approve control. Single-order explicit approve/reject decision.
  - **Order Board Kanban (`src/routes/orders/+page.svelte`, `state/orders.ts`)**:
    - Multi-column fulfillment Kanban (Confirmed, Allocated, Packed, Dispatched, Delivered).
    - State transition validation preventing illegal drag-and-drop operations (Doc 16 §9).
  - **Inventory & Cold Chain (`src/routes/inventory/+page.svelte`, `state/inventory.ts`)**:
    - Expiry risk dashboard (≤30, 31-60, 61-90 days) with value at risk totals.
    - Cold chain temperature log with excursion alerts.
  - **B2B Medical Device Desk (`src/routes/b2b/+page.svelte`, `state/b2b.ts`)**:
    - Hospital credit limits, current balances, and 90+ days overdue account locks.
    - Manufacturer device recall inquiry by batch/lot ID.
  - **Regulatory Audit Explorer (`src/routes/audit/+page.svelte`)**:
    - DRAP compliance immutable log with state diffs and CSV export.

## Acceptance tests
Spec names 14 acceptance tests. I implemented all **14** in `apps/console/src/console.test.ts` (100% green).

| Spec test name | My test | File |
|---|---|---|
| `no_hand_written_api_types` | `no_hand_written_api_types` | `apps/console/src/console.test.ts` |
| `no_money_arithmetic_in_browser` | `no_money_arithmetic_in_browser` | `apps/console/src/console.test.ts` |
| `rx_review_approve_disabled_until_all_lines_decided` | `rx_review_approve_disabled_until_all_lines_decided` | `apps/console/src/console.test.ts` |
| `no_bulk_approve_control_in_rx_review` | `no_bulk_approve_control_in_rx_review` | `apps/console/src/console.test.ts` |
| `no_bulk_approve_control_in_payment_review` | `no_bulk_approve_control_in_payment_review` | `apps/console/src/console.test.ts` |
| `rx_linked_conversation_excluded_from_bulk_send` | `rx_linked_conversation_excluded_from_bulk_send` | `apps/console/src/console.test.ts` |
| `duplicate_tid_renders_critical_banner` | `duplicate_tid_renders_critical_banner` | `apps/console/src/console.test.ts` |
| `every_screen_renders_in_urdu_rtl` | `every_screen_renders_in_urdu_rtl` | `apps/console/src/console.test.ts` |
| `status_colours_consistent_across_screens` | `status_colours_consistent_across_screens` | `apps/console/src/console.test.ts` |
| `sse_reconnects_and_replays_after_drop` | `sse_reconnects_and_replays_after_drop` | `apps/console/src/console.test.ts` |
| `virtualised_lists_render_10000_rows_smoothly` | `virtualised_lists_render_10000_rows_smoothly` | `apps/console/src/console.test.ts` |
| `keyboard_flow_completes_rx_review_without_mouse` | `keyboard_flow_completes_rx_review_without_mouse` | `apps/console/src/console.test.ts` |
| `order_board_rejects_illegal_transition_drop` | `order_board_rejects_illegal_transition_drop` | `apps/console/src/console.test.ts` |
| `all_screens_handle_loading_empty_error` | `all_screens_handle_loading_empty_error` | `apps/console/src/console.test.ts` |

Missing, with reason: None. All 14 tests passing.

## Out of scope
Confirmed nothing from Out of scope was built:
- No rider PWA code (in `apps/rider`).
- No Astro marketing site code (in `apps/web`).
- No backend code changes or API monkey-patching.

## ASSUMPTIONS
None.

## Known gaps
None.

## Contract changes
- Exported design tokens, money helpers, and i18n from `@shifa/shared`.

## Risk areas
- High prescription volume operations require pharmacist training on keyboard shortcuts (`1..N`, `A`, `X`, `Ctrl+Enter`) for optimal throughput.
