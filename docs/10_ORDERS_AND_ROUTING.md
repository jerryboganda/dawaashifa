# DOC 10 — ORDERS, STATE MACHINE & BRANCH ROUTING

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 04, 05, 06, 07
**Produces:** `crates/orders`
**Branch:** `feat/10-orders-routing`

> **This spec closes the revenue loop.** After Docs 01–07 and 10, the business can trade with no AI, no gateway payments, and no rider app. Everything after this is improvement, not viability.

---

## 1. Objective

Cart, order lifecycle, branch routing against shared stock, and COD. The state machine is the spine of the platform.

## 2. In scope

- Cart building from conversation or console
- Order state machine with exhaustive transitions
- Branch routing across the shared stock pool
- Split fulfilment and inter-branch transfer fallback
- Stock reservation on confirmation
- COD as the primary payment method
- Order numbering, cancellation, returns
- Delivery fee and tax calculation hooks

## 3. Out of scope — do NOT build

- Payment gateways or screenshots (Doc 11)
- Rider assignment or delivery tracking (Doc 12)
- FBR invoice submission (Doc 13 — call the hook, stub the implementation)
- B2B quotes and credit (Doc 14)

## 4. State machine — exhaustive

```rust
pub enum OrderStatus {
    Draft, CartConfirmed, AwaitingRx, RxUnderReview, RxApproved, RxRejected,
    AwaitingPayment, PaymentUnderReview, PaymentRejected, Confirmed,
    Picking, Packed, Dispatched, OutForDelivery, Delivered,
    CashReconciled, Closed, Cancelled, FailedDelivery, Returned, Refunded,
}

pub fn can_transition(from: OrderStatus, to: OrderStatus) -> bool {
    use OrderStatus::*;
    matches!((from, to),
        (Draft, CartConfirmed) | (Draft, Cancelled)
      | (CartConfirmed, AwaitingRx) | (CartConfirmed, AwaitingPayment)
      | (CartConfirmed, Cancelled)
      | (AwaitingRx, RxUnderReview) | (AwaitingRx, Cancelled)
      | (RxUnderReview, RxApproved) | (RxUnderReview, RxRejected)
      | (RxApproved, AwaitingPayment) | (RxRejected, Cancelled)
      | (AwaitingPayment, PaymentUnderReview) | (AwaitingPayment, Confirmed)
      | (AwaitingPayment, Cancelled)
      | (PaymentUnderReview, Confirmed) | (PaymentUnderReview, PaymentRejected)
      | (PaymentRejected, AwaitingPayment) | (PaymentRejected, Cancelled)
      | (Confirmed, Picking) | (Confirmed, Cancelled)
      | (Picking, Packed) | (Picking, Cancelled)
      | (Packed, Dispatched) | (Packed, Cancelled)
      | (Dispatched, OutForDelivery)
      | (OutForDelivery, Delivered) | (OutForDelivery, FailedDelivery)
      | (Delivered, CashReconciled) | (Delivered, Closed) | (Delivered, Returned)
      | (CashReconciled, Closed)
      | (FailedDelivery, OutForDelivery) | (FailedDelivery, Returned)
      | (Returned, Refunded) | (Returned, Closed)
      | (Refunded, Closed)
    )
}
```

Every transition, in one database transaction:
1. Validate with `can_transition`, else `Err(InvalidTransition)`
2. Update `orders.status`
3. Insert `order_events`
4. Insert `audit_log`
5. Run side effects (reserve stock, notify customer, etc.)

**If the audit write fails, the whole transaction rolls back.** Invariant I-9.

### 4.1 Rx branching
An order containing any item with `is_prescription_only = true` **must** pass through `AwaitingRx → RxUnderReview → RxApproved`. There is no code path from `CartConfirmed` directly to `AwaitingPayment` when an Rx item is present. Enforced in `confirm_cart`, tested explicitly.

## 5. Order numbering

Format: `{BRANCH_CODE}-{YYMMDD}-{SEQ4}` e.g. `LHR01-260820-0042`

Sequence per branch per day from a Postgres sequence, gap-tolerant. **Never derive it from a count** — concurrent inserts would collide. `UNIQUE (tenant_id, order_no)`.

## 6. Branch routing

```rust
pub struct RoutingRequest {
    pub items: Vec<(ProductId, i32)>,
    pub customer_geo: Option<Point>,
    pub delivery_address: String,
    pub requires_cold_chain: bool,
}

pub enum RoutingResult {
    Single { branch: BranchId, allocations: Vec<BatchAllocation> },
    Split  { parts: Vec<(BranchId, Vec<BatchAllocation>)> },
    RequiresTransfer { fulfilling: BranchId, transfers: Vec<TransferPlan>, eta_hours: i32 },
    Unfulfillable { missing: Vec<(ProductId, i32)> },
}
```

Algorithm:
1. Candidate branches: `ACTIVE`, within `service_radius_km` of the customer, cold-chain-capable if needed
2. Filter to those with all items available via FEFO (Doc 06)
3. Rank: full-fill possible → road distance → current picking load → stock depth
4. If none can fill completely, apply the tenant's `split_fulfilment_policy`:
   - `ALLOW_SPLIT` → split across branches, one customer-facing order
   - `PREFER_TRANSFER` → single branch plus inter-branch transfer, if ETA is acceptable
   - `SINGLE_BRANCH_ONLY` → `Unfulfillable`
5. On `Unfulfillable`, return which items are missing so staff can offer substitutes

Routing is recomputed if the order sits in `AwaitingPayment` for over 30 minutes — stock moves.

## 7. Reservations

On `Confirmed`, reserve via Doc 06 with TTL:
- COD: 2 hours
- Awaiting gateway payment: 30 minutes

On `Cancelled`, `RxRejected`, `PaymentRejected`, or TTL expiry, release. Release is idempotent.

**Reservation failure blocks the transition to `Confirmed`.** Never confirm an order you cannot fill.

## 8. Pricing

```
line_total = qty × unit_price − line_discount
subtotal   = Σ line_total
total      = subtotal − order_discount + delivery_fee + tax_amount
```

- `unit_price` ≤ `product.mrp`, enforced by Doc 05. Hard block.
- `mrp_at_sale` snapshotted onto the line — MRP changes must not alter historical orders
- Delivery fee: per-branch base plus distance bands, free above a threshold
- Tax via Doc 13's `calculate_tax` hook; stub returns zero until Doc 13 lands
- **All money arithmetic in `rust_decimal::Decimal`.** Never `f64`.

## 9. Cancellation and returns

Cancellable up to `Dispatched` by staff with `order.cancel`; by the customer only before `Picking`.

Returns:
- Medicines are non-returnable once dispatched, except for a dispensing error or a quality defect. Enforce as a policy flag per category, not a global rule.
- A return creates `RETURN` stock movements **only** for items a pharmacist certifies as safe to restock. Anything that left the cold chain, or any opened pack, is written off, not restocked.
- `Refunded` requires `payment.refund`.

## 10. Endpoints

```
POST   /api/v1/orders                       create draft from conversation
GET    /api/v1/orders                       ?status&branch&customer&from&to&page
GET    /api/v1/orders/:id
POST   /api/v1/orders/:id/items             add line
PATCH  /api/v1/orders/:id/items/:item_id    change qty or price [order.edit]
DELETE /api/v1/orders/:id/items/:item_id
POST   /api/v1/orders/:id/confirm-cart      → AwaitingRx or AwaitingPayment
POST   /api/v1/orders/:id/route             recompute routing
POST   /api/v1/orders/:id/transition        {to, reason} generic guarded transition
POST   /api/v1/orders/:id/cancel            [order.cancel]
POST   /api/v1/orders/:id/return            [order.edit]
GET    /api/v1/orders/:id/events            timeline
GET    /api/v1/orders/routing/preview       dry run before creating an order
```

## 11. Acceptance tests

- `every_illegal_transition_rejected` — exhaustive matrix over all 21 states
- `transition_writes_event_and_audit_atomically`
- `audit_failure_rolls_back_status_change`
- `rx_item_forces_rx_branch` — cannot skip to AwaitingPayment
- `non_rx_order_skips_rx_branch`
- `order_number_unique_under_concurrency` — 100 parallel, zero collisions
- `routing_prefers_single_branch_over_split`
- `routing_respects_cold_chain_capability`
- `routing_split_policy_respected` — all three policies
- `routing_unfulfillable_lists_missing_items`
- `routing_recomputed_after_30_minutes_in_awaiting_payment`
- `confirm_fails_when_reservation_fails`
- `cancel_releases_reservation`
- `reservation_release_idempotent`
- `price_above_mrp_rejected_on_line_add`
- `mrp_snapshot_immutable_after_mrp_change`
- `money_arithmetic_uses_decimal` — no float in the crate
- `return_restock_requires_pharmacist_certification`
- `cold_chain_item_never_restocked_on_return`

## 12. Done checklist

- [ ] Exhaustive state machine; illegal transitions return errors
- [ ] Transition, event, and audit written atomically
- [ ] Rx branch unavoidable when Rx items present
- [ ] Sequence-based order numbering, collision-free under load
- [ ] Routing with all four outcomes and three split policies
- [ ] Reservation on confirm; idempotent release; confirm blocked on failure
- [ ] MRP enforced and snapshotted; all money in `Decimal`
- [ ] Return restocking gated on pharmacist certification
- [ ] All 19 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
