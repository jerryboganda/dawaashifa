# DOC 06 — INVENTORY LEDGER, BATCHES, EXPIRY & COLD CHAIN

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 04, 05
**Produces:** `crates/inventory`
**Branch:** `feat/06-inventory-ledger`

---

## 1. Objective

Batch-level stock tracking as an append-only ledger, with expiry control, cold chain logging, reservations, and inter-branch transfers. This is what a DRAP inspection looks at.

## 2. In scope

- Append-only `stock_movements` ledger (invariant I-5)
- `stock_current` maintained by trigger
- Batch receipt with expiry and supplier
- FEFO allocation (first expired, first out)
- Reservations with TTL
- Inter-branch transfers with in-transit state
- Expiry reporting and automatic write-off
- Cold chain temperature logs and excursion flagging
- Stock take / adjustment with reason codes

## 3. Out of scope — do NOT build

- Purchase orders or supplier ordering workflows
- Branch routing decisions (Doc 10 consumes availability from here)
- Demand forecasting
- Barcode hardware integration

## 4. The ledger — invariant I-5

**Never `UPDATE stock_current.qty` directly. Ever.** Insert a movement; a trigger maintains the projection.

```sql
CREATE TYPE movement_type AS ENUM (
  'RECEIPT','SALE','RETURN','TRANSFER_OUT','TRANSFER_IN',
  'ADJUSTMENT','EXPIRY_WRITEOFF','DAMAGE','RESERVATION','RELEASE'
);

CREATE OR REPLACE FUNCTION apply_stock_movement() RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO stock_current (tenant_id, branch_id, product_id, batch_id, qty)
  VALUES (NEW.tenant_id, NEW.branch_id, NEW.product_id, NEW.batch_id, NEW.qty_delta)
  ON CONFLICT (tenant_id, branch_id, product_id, batch_id)
  DO UPDATE SET qty = stock_current.qty + NEW.qty_delta;

  IF (SELECT qty FROM stock_current WHERE tenant_id = NEW.tenant_id
        AND branch_id = NEW.branch_id AND product_id = NEW.product_id
        AND batch_id = NEW.batch_id) < 0 THEN
    RAISE EXCEPTION 'negative stock for batch %', NEW.batch_id;
  END IF;
  RETURN NEW;
END $$ LANGUAGE plpgsql;
```

Negative stock raises, rolling back the transaction. There is no configuration that permits it.

`stock_movements` is partitioned monthly and never deleted.

## 5. FEFO allocation

```rust
pub async fn allocate_fefo(
    ctx: &TenantContext, pool: &PgPool,
    branch: BranchId, product: ProductId, qty: i32,
) -> Result<Vec<BatchAllocation>, InventoryError>;
```

Order by `expiry_date ASC`, excluding batches expiring within `min_shelf_life_days` (tenant setting, default 90). Split across batches when one is insufficient. Return `Err(InsufficientStock { available })` when short — never partially allocate silently.

**Never dispense a batch expiring sooner than the patient's course length.** A 30-day course cannot be filled from a batch expiring in 14 days.

## 6. Reservations

On order confirmation, insert `RESERVATION` movements (negative delta) with a TTL:

```sql
stock_reservations(id, tenant_id, order_id, branch_id, product_id, batch_id,
                   qty, expires_at, released_at, created_at)
```

Default TTL 2 hours for COD, 30 minutes for awaiting-payment. A scheduled worker releases expired reservations by inserting compensating `RELEASE` movements. Reservation release is idempotent — running it twice must not double-release.

## 7. Transfers

```
DRAFT → DISPATCHED → IN_TRANSIT → RECEIVED
                   ↘ CANCELLED    ↘ DISCREPANCY
```

`TRANSFER_OUT` at dispatch, `TRANSFER_IN` at receipt. Stock in transit belongs to neither branch's available pool. Quantity mismatch at receipt sets `DISCREPANCY` and requires manual reconciliation with a reason code — it does not auto-adjust.

## 8. Expiry management

- Nightly job flags batches expiring within 90 / 60 / 30 days
- Batches past expiry are automatically excluded from allocation, immediately, without waiting for the job
- `EXPIRY_WRITEOFF` movements require `inventory.adjust` and a reason
- Report: expiring stock by branch, by value, by product

## 9. Cold chain

```sql
cold_chain_logs(id, tenant_id, branch_id, batch_id, temperature_c NUMERIC(4,1),
                recorded_at, recorded_by, source, is_excursion, note)
```

- Products with `requires_cold_chain` may only be held at branches with `cold_chain_capable = true`
- Acceptable range configurable per product, default 2–8°C
- Readings outside range set `is_excursion = true` and alert immediately
- A batch with an unresolved excursion is **quarantined** — excluded from allocation until a pharmacist clears it with a documented decision

## 10. Endpoints

```
GET    /api/v1/inventory/stock            ?branch_id&product_id&low_stock&expiring_days
GET    /api/v1/inventory/availability     ?product_ids[]&qty[]&geo  → branches that can fill
POST   /api/v1/inventory/receipts         [inventory.receive]
POST   /api/v1/inventory/adjustments      [inventory.adjust]  {reason required}
POST   /api/v1/inventory/transfers        [inventory.transfer]
POST   /api/v1/inventory/transfers/:id/dispatch
POST   /api/v1/inventory/transfers/:id/receive
GET    /api/v1/inventory/movements        ?branch&product&batch&from&to  (audit trail)
GET    /api/v1/inventory/expiring         ?days=90
POST   /api/v1/inventory/writeoff         [inventory.adjust]
POST   /api/v1/inventory/cold-chain/logs  [inventory.view]
POST   /api/v1/inventory/cold-chain/:batch_id/clear-excursion  [rx.approve]
```

## 11. Acceptance tests

- `movement_updates_current_stock`
- `negative_stock_raises_and_rolls_back`
- `no_code_path_updates_stock_current_directly` — grep-based test asserting no `UPDATE stock_current` outside the trigger
- `fefo_allocates_earliest_expiry_first`
- `fefo_excludes_batches_below_min_shelf_life`
- `fefo_splits_across_batches`
- `fefo_insufficient_returns_error_not_partial`
- `reservation_reduces_available_stock`
- `expired_reservation_released_by_worker`
- `reservation_release_is_idempotent`
- `transfer_stock_invisible_at_both_branches_while_in_transit`
- `transfer_quantity_mismatch_sets_discrepancy`
- `expired_batch_excluded_from_allocation_immediately`
- `cold_chain_product_rejected_at_non_capable_branch`
- `excursion_quarantines_batch_from_allocation`
- `excursion_clear_requires_rx_approve_permission`
- `concurrent_allocation_does_not_oversell` — 50 parallel allocations of 1 unit against stock of 10 yields exactly 10 successes

The concurrency test is not optional. Overselling is the most likely production failure in this module.

## 12. Done checklist

- [ ] Ledger with trigger-maintained projection; negative stock impossible
- [ ] Monthly partitioning on `stock_movements`
- [ ] FEFO with shelf-life floor and multi-batch splitting
- [ ] Reservations with TTL and idempotent release worker
- [ ] Transfers with in-transit isolation and discrepancy handling
- [ ] Expiry exclusion immediate, write-off audited
- [ ] Cold chain logging, excursion quarantine, pharmacist clearance
- [ ] All 17 acceptance tests green, including the concurrency test
- [ ] `contracts/openapi.json` regenerated
