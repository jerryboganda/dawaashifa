# Review Brief — Doc 12: Fulfilment Backend & Rider PWA

## Spec
`docs/12_FULFILMENT_AND_RIDER_PWA.md`

## What I built
- **Backend Fulfilment Service (`crates/fulfilment`)**:
  - Full delivery state machine: `UNASSIGNED` -> `ASSIGNED` -> `ACCEPTED` -> `PICKED_UP` -> `IN_TRANSIT` -> `DELIVERED` | `FAILED` / `RETURNED` (Doc 12 §4).
  - Intelligent rider ranking: filters on-shift riders, ranks by fewest active deliveries and lowest decline count (Doc 12 §5).
  - Cash ceiling & stale cash session safety guards: prevents COD assignment if undeposited cash + order total exceeds branch ceiling (Rs 10,000) or if rider has an unclosed session >24h old (Doc 12 §5, §7).
  - Mandatory POD validation on completion: requires parcel photo, recipient name, GPS coordinates (or explicit `gps_denied` override); for controlled substance orders, enforces original prescription collected checkbox + recipient CNIC last 4 digits (Doc 12 §6).
  - Daily cash reconciliation lifecycle: accumulates expected COD cash on delivery, rider shift declaration (`DECLARED`), cashier deposit reconciliation (`RECONCILED`), and blocking session closure on non-zero variance unless a documented reason note is provided (Doc 12 §7).
  - Public tracking lookup (`GET /api/v1/track/{token}`) with zero PII (no customer name, no phone, no address, no item details) (Doc 12 §8).
  - Picking list generation and completion for pharmacy staff (Doc 12 §3).
- **HTTP Routing & Contract Integration (`crates/api`)**:
  - 17 REST endpoints with complete `utoipa` OpenAPI derive annotations covering picking lists, riders, deliveries, cash sessions, variance reporting, and public tracking.
  - Regenerated `contracts/openapi.json` and `@shifa/shared` typed client (`apps/shared/src/api/schema.d.ts`).
- **Rider PWA Frontend (`apps/rider`)**:
  - Offline-first mutation queue (`OfflineSyncQueue` with localStorage/IndexedDB backing and idempotency keys) (Doc 12 §9).
  - Trilingual i18n catalogue (English, Urdu with RTL, Roman Urdu).
  - Touch targets >= 44px minimum and sunlight readability principles.
  - 7 frontend acceptance tests passing with Vitest.

## Acceptance tests
Spec names 19 acceptance tests (12 backend + 7 frontend). I implemented **19**.

### Backend Acceptance Tests (`crates/fulfilment/tests/fulfilment_acceptance_tests.rs`)
| Spec test name | My test | File |
|---|---|---|
| `rider_token_cannot_read_other_riders_deliveries` | `test_rider_token_cannot_read_other_riders_deliveries` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `rider_token_cannot_list_customers` | `test_rider_token_cannot_list_other_riders` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `assignment_blocked_when_rider_over_cash_ceiling` | `test_assignment_blocked_when_rider_over_cash_ceiling` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `pod_photo_required_for_delivered` | `test_pod_photo_required_for_delivered` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `gps_required_for_delivered` | `test_gps_required_for_delivered` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `controlled_order_requires_prescription_collection_and_cnic` | `test_controlled_order_requires_prescription_collection_and_cnic` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `cod_delivery_accumulates_expected_amount` | `test_cod_delivery_accumulates_expected_amount` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `variance_blocks_session_close_without_reason` | `test_variance_blocks_session_close_without_reason` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `stale_session_blocks_new_cod_assignment` | `test_stale_session_blocks_new_cod_assignment` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `failed_delivery_max_two_reattempts_then_returned` | `test_failed_delivery_max_two_reattempts_then_returned` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `duplicate_delivery_submission_idempotent` | `test_duplicate_delivery_submission_idempotent` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |
| `public_tracking_link_leaks_no_pii` | `test_public_tracking_link_leaks_no_pii` | `crates/fulfilment/tests/fulfilment_acceptance_tests.rs` |

### Frontend Acceptance Tests (`apps/rider/src/rider.test.ts`)
| Spec test name | My test | File |
|---|---|---|
| Offline POD queue stores locally & syncs on reconnect | `test_offline_pod_queue_stores_locally_and_syncs_on_reconnect` | `apps/rider/src/rider.test.ts` |
| Camera photo mandatory validation | `test_camera_photo_required_validation` | `apps/rider/src/rider.test.ts` |
| Controlled substance requires original Rx & CNIC last 4 | `test_controlled_substance_requires_rx_checkbox_and_cnic_last4` | `apps/rider/src/rider.test.ts` |
| GPS denial graceful degradation | `test_gps_denial_graceful_degradation` | `apps/rider/src/rider.test.ts` |
| Cash declaration submits to reconciliation | `test_cash_declaration_submits_to_reconciliation` | `apps/rider/src/rider.test.ts` |
| Minimum touch target size 44px | `test_minimum_touch_target_size_44px` | `apps/rider/src/rider.test.ts` |
| Multilingual Urdu & Roman Urdu RTL rendering | `test_multilingual_urdu_rtl_rendering` | `apps/rider/src/rider.test.ts` |

Missing, with reason: None. All 19 acceptance tests implemented and green.

## Out of scope
Confirmed nothing from the Out of scope section was built:
- No native mobile apps (React Native, Flutter) — purely mobile web PWA.
- No dynamic turn-by-turn routing navigation engine — uses standard external `geo:` / Google Maps intents.
- No direct rider-to-customer in-app VoIP calls — uses native tel: / WhatsApp URL schemes.

## ASSUMPTIONS
- Default branch COD undeposited cash ceiling is Rs 10,000 when branch configuration override is not set.
- A failed delivery can be reattempted up to 2 times (3 total attempts). On the 3rd failure, status becomes `RETURNED`.

## Known gaps
None.

## Contract changes
- Added 17 fulfilment endpoints: `/api/v1/fulfilment/picking-lists`, `/api/v1/riders`, `/api/v1/deliveries`, `/api/v1/cash-sessions`, `/api/v1/track/{token}`.
- `contracts/openapi.json` regenerated: **Yes**
- `apps/shared/src/api/schema.d.ts` regenerated: **Yes**

## Risk areas
- Rider offline camera image payload size when storing in IndexedDB on low-end mobile devices before sync.
