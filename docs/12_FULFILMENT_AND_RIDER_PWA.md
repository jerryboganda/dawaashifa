# DOC 12 — FULFILMENT, RIDER PWA & CASH RECONCILIATION

**Agents:** Backend (Copilot) for `crates/fulfilment` · Frontend (Antigravity) for `apps/rider`
**Depends on:** Docs 01, 04, 06, 10, 11
**Produces:** `crates/fulfilment`, `apps/rider`
**Branch:** `feat/12-fulfilment` (backend), `feat/12-rider-pwa` (frontend)

> This spec spans both agents. Build the backend first and regenerate `contracts/openapi.json` before starting the PWA.

---

## 1. Objective

Picking, packing, rider assignment, delivery tracking, proof of delivery, and daily cash reconciliation. Riders carry cash; the reconciliation is a financial control, not a convenience feature.

## 2. In scope

**Backend:** picking lists, rider management, assignment, delivery lifecycle, POD storage, cash sessions, variance reporting, customer tracking link
**Frontend:** rider PWA — task list, navigation handoff, POD capture, cash collection, offline queue

## 3. Out of scope — do NOT build

- Third-party logistics integration
- Real-time map tracking for customers (a status link is enough)
- Native mobile apps
- Route optimisation across multiple orders

## 4. Delivery lifecycle

```
UNASSIGNED → ASSIGNED → ACCEPTED → PICKED_UP → IN_TRANSIT
           → DELIVERED | FAILED
FAILED → REASSIGNED → ASSIGNED   (max 2 reattempts, then RETURNED)
```

A rider may decline an assignment with a reason; it returns to `UNASSIGNED` and the rider's decline count is recorded.

## 5. Assignment

Per-branch strategy: `MANUAL` or `AUTO_NEAREST`.

`AUTO_NEAREST` ranks available riders by: currently on shift → fewest active deliveries → nearest to the branch → lowest recent decline rate.

Constraints: a rider carrying more than the branch's COD ceiling in undeposited cash cannot be assigned further COD orders. This is the control that stops cash accumulating on the street.

## 6. Proof of delivery

Required on `DELIVERED`:
- Photo (package at the door, or with the recipient) — mandatory
- GPS coordinates at the moment of delivery — mandatory
- Recipient name — mandatory
- Signature — optional, per-branch setting
- Cash collected amount — mandatory for COD

**Controlled substance orders additionally require:** original physical prescription collected (checkbox plus photo) and recipient CNIC last four digits. Flagged from Doc 09.

POD images go to MinIO at `{tenant_id}/pod/{yyyy}/{mm}/{delivery_id}.jpg`. Retained per the tenant's retention policy, minimum two years.

## 7. Cash reconciliation

```sql
rider_cash_sessions(id, tenant_id, rider_id, branch_id,
                    opened_at, closed_at,
                    expected_amount, collected_amount, deposited_amount,
                    variance, reconciled_by, status, note)
```

- A session opens on the rider's first COD delivery of the day
- `expected_amount` accumulates from each COD `DELIVERED`
- At shift end the rider declares `collected_amount`; a cashier records `deposited_amount`
- `variance = deposited − expected`. Non-zero variance requires a documented reason and blocks session closure.
- A rider with an open session over 24 hours old is flagged and blocked from new COD assignments

Variance report by rider, by branch, by week. Persistent shortfalls are a personnel matter the system must make visible, not silently absorb.

## 8. Backend endpoints

```
GET    /api/v1/fulfilment/picking-lists       ?branch&status
POST   /api/v1/fulfilment/picking-lists/:id/complete
GET    /api/v1/riders                         ?branch&status&on_shift
POST   /api/v1/riders                         [user.create]
POST   /api/v1/riders/:id/shift/start
POST   /api/v1/riders/:id/shift/end
GET    /api/v1/deliveries                     ?branch&rider&status&date
POST   /api/v1/deliveries/:id/assign          [order.edit]
POST   /api/v1/deliveries/:id/accept          rider token
POST   /api/v1/deliveries/:id/decline         rider token, {reason}
POST   /api/v1/deliveries/:id/pickup          rider token
POST   /api/v1/deliveries/:id/deliver         rider token, multipart POD
POST   /api/v1/deliveries/:id/fail            rider token, {reason}
GET    /api/v1/cash-sessions                  ?rider&branch&status
POST   /api/v1/cash-sessions/:id/declare      rider token
POST   /api/v1/cash-sessions/:id/reconcile    [payment.approve]
GET    /api/v1/cash-sessions/variance-report  [report.view]
GET    /api/v1/track/:token                   public, no auth, customer status
```

Rider tokens are scoped: a rider can only read and act on their own assigned deliveries. A rider token must not be able to list other riders' deliveries or read customer records beyond the current delivery.

## 9. Rider PWA

**Design constraints, not preferences:** used one-handed, outdoors, in sunlight, on a cheap Android phone, on a patchy connection, by someone in a hurry.

- Minimum 44×44px touch targets; primary actions at the bottom of the screen within thumb reach
- High contrast; readable in direct sunlight
- Urdu and Roman Urdu interface — many riders do not read English comfortably
- Large text, minimal chrome, one primary action per screen

Screens:
1. **Today** — assigned deliveries, ordered by route, each showing address, amount to collect, distance
2. **Delivery detail** — customer name, phone (tap to call), address (tap to open Google Maps), items, amount
3. **Deliver** — camera for POD, recipient name, cash collected, confirm
4. **Failed** — reason picker, optional photo, confirm
5. **Cash** — today's expected total, declare collected, session status

### 9.1 Offline behaviour — mandatory

Assume the network drops mid-delivery.

- Queue all writes in IndexedDB with an idempotency key
- Sync on reconnect, in order, retrying with backoff
- **Never block the UI on a network call.** Optimistic update, reconcile after.
- Persistent sync indicator: how many actions are pending
- POD photos queue as blobs and upload when signal returns; compress to under 500KB before queueing
- A delivery marked complete offline shows as complete to the rider and syncs later. Duplicate submission on retry is prevented by the idempotency key server-side.

### 9.2 Permissions
Camera and GPS may be denied. Degrade gracefully: if GPS is denied, allow delivery with a recorded flag rather than blocking. If the camera is denied, block delivery — POD is mandatory — and show clear instructions to enable it.

## 10. Acceptance tests

**Backend**
- `rider_token_cannot_read_other_riders_deliveries`
- `rider_token_cannot_list_customers`
- `assignment_blocked_when_rider_over_cash_ceiling`
- `pod_photo_required_for_delivered`
- `gps_required_for_delivered`
- `controlled_order_requires_prescription_collection_and_cnic`
- `cod_delivery_accumulates_expected_amount`
- `variance_blocks_session_close_without_reason`
- `stale_session_blocks_new_cod_assignment`
- `failed_delivery_max_two_reattempts_then_returned`
- `duplicate_delivery_submission_idempotent`
- `public_tracking_link_leaks_no_pii` — no phone, no address, no items

**Frontend**
- `offline_delivery_queues_and_syncs_on_reconnect`
- `queued_action_not_duplicated_on_retry`
- `pod_photo_compressed_under_500kb`
- `ui_renders_correctly_in_urdu_rtl`
- `all_touch_targets_meet_44px`
- `gps_denied_allows_delivery_with_flag`
- `camera_denied_blocks_delivery_with_instructions`

## 11. Done checklist

- [ ] Delivery lifecycle with decline, reattempt cap, and return
- [ ] Assignment with cash-ceiling constraint
- [ ] POD: photo, GPS, recipient mandatory; controlled-substance extras
- [ ] Cash sessions with variance blocking closure
- [ ] Scoped rider tokens verified against privilege-escalation tests
- [ ] Public tracking link carrying no PII
- [ ] PWA installable, offline-tolerant, idempotent sync
- [ ] Urdu RTL verified on every rider screen
- [ ] All 19 acceptance tests green
- [ ] `contracts/openapi.json` regenerated before PWA work began
