# DOC 07 — CONVERSATION ENGINE, INBOX & HUMAN OVERRIDE

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 02, 04
**Produces:** `crates/conversation`
**Branch:** `feat/07-conversation-engine`

---

## 1. Objective

Turn a stream of WhatsApp messages into threaded conversations with routing, assignment, and human override. This closes the manual revenue loop — after this spec plus Doc 10, staff can serve customers end to end without any AI.

## 2. In scope

- Conversation threading and lifecycle
- Customer auto-creation on first inbound
- Branch routing of new conversations
- Assignment: manual, round-robin, and claim
- Real-time inbox feed via SSE
- Canned replies with variable substitution
- Human override of any pending outbound message
- Conversation notes, tags, and search
- SLA timers and escalation

## 3. Out of scope — do NOT build

- AI reply generation (Doc 08)
- Prescription handling (Doc 09)
- Cart or order creation (Doc 10)
- The inbox UI (Doc 16)

## 4. Lifecycle

```
NEW → BOT_HANDLING → AWAITING_HUMAN → ASSIGNED → RESOLVED → CLOSED
                                   ↘ ESCALATED ↗
```

- `NEW` — first inbound, not yet routed
- `BOT_HANDLING` — automation is handling it (no-op until Doc 08)
- `AWAITING_HUMAN` — in the unassigned queue
- `ASSIGNED` — a specific user owns it
- `ESCALATED` — raised to branch manager or operations head
- `RESOLVED` — staff marked it done; reopens automatically on new inbound
- `CLOSED` — 7 days after `RESOLVED` with no inbound

Any inbound message on a `RESOLVED` or `CLOSED` conversation reopens it as `AWAITING_HUMAN`, preserving history.

## 5. Customer resolution

On inbound from an unknown MSISDN:
1. Create a `customers` row with `msisdn`, `display_name` from the WhatsApp profile
2. Set `preferred_locale` to `UNKNOWN` — do not guess yet
3. Create the conversation

`UNIQUE (tenant_id, msisdn)` prevents duplicates under concurrent inbound. Handle the conflict by fetching, not by erroring.

If `customers.is_blocked` is true: persist the message, do not route, do not notify. Blocked customers get silence, not an error message.

## 6. Branch routing

New conversations route by, in order:
1. Explicit branch if the customer messaged a branch-specific number
2. Customer's last-ordered branch, if within 60 days
3. Nearest branch to the customer's default geo, if known
4. Tenant default branch

Routing is a suggestion, not a lock. Staff can transfer a conversation to another branch; the transfer is audited.

## 7. Assignment

```rust
pub enum AssignmentStrategy { Manual, RoundRobin, LeastBusy }
```

Per-branch setting. `LeastBusy` counts open `ASSIGNED` conversations per online user. Only users with `inbox.view` on that branch are eligible.

- Claim: `POST /conversations/:id/claim` — atomic, first writer wins, second gets 409
- Reassign requires `inbox.assign`
- Unassigned conversations older than the SLA threshold escalate automatically

## 8. Human override — the control that matters

```sql
messages(..., status, ai_generated, ai_confidence, overridden_by,
         original_body, approved_by, approved_at)
```

Outbound messages have a status machine:
```
DRAFT → PENDING_APPROVAL → APPROVED → QUEUED → SENT → DELIVERED → READ
      ↘ DISCARDED                            ↘ FAILED
```

- A message in `PENDING_APPROVAL` can be edited by any user with `inbox.override`
- Editing preserves `original_body` and sets `overridden_by`
- **Every override is training data.** On override, publish `conversation.reply_overridden` to NATS with original, corrected, and context. Doc 08 consumes this.
- Bulk approval of drafts is permitted for non-Rx conversations only. Rx-linked conversations require individual review (invariant I-6).

## 9. Canned replies

```sql
canned_replies(id, tenant_id, branch_id NULL, shortcode, title,
               body_en, body_ur, body_ur_latn, variables JSONB, usage_count)
```

Variables: `{{customer_name}}`, `{{order_no}}`, `{{branch_name}}`, `{{branch_phone}}`, `{{total}}`, `{{rider_name}}`. Unresolved variables block sending with a clear error rather than shipping `{{order_no}}` to a customer.

## 10. Real-time feed

`GET /api/v1/inbox/stream` — SSE, authenticated, scoped to the user's branches.

Events: `message.inbound`, `message.status_changed`, `conversation.assigned`, `conversation.escalated`, `conversation.status_changed`.

Heartbeat every 25s. Client reconnects with `Last-Event-ID`; server replays from the durable NATS consumer. Cap replay at 100 events, then instruct a full refresh.

## 11. SLA

Per-branch config, defaults:
- First response: 15 minutes during opening hours
- Resolution: 4 hours
- Breach escalates to `BRANCH_MANAGER`, then `OPERATIONS_HEAD` after a further 30 minutes

Timers pause outside branch opening hours. A conversation opened at 11pm does not breach by morning.

## 12. Endpoints

```
GET    /api/v1/conversations          ?status&branch&assigned_to&q&unread&page
GET    /api/v1/conversations/:id
GET    /api/v1/conversations/:id/messages   ?before&limit  (cursor paginated)
POST   /api/v1/conversations/:id/messages   send — respects window rules
POST   /api/v1/conversations/:id/claim
POST   /api/v1/conversations/:id/assign     [inbox.assign]
POST   /api/v1/conversations/:id/transfer   {branch_id}  [inbox.assign]
POST   /api/v1/conversations/:id/resolve
POST   /api/v1/conversations/:id/escalate
POST   /api/v1/conversations/:id/notes
POST   /api/v1/conversations/:id/tags
PATCH  /api/v1/messages/:id                 edit a PENDING_APPROVAL draft [inbox.override]
POST   /api/v1/messages/:id/approve         [inbox.reply]
POST   /api/v1/messages/:id/discard
GET    /api/v1/inbox/stream                 SSE
GET    /api/v1/canned-replies               ?q&branch_id
POST   /api/v1/canned-replies               [inbox.reply]
```

## 13. Acceptance tests

- `first_inbound_creates_customer_and_conversation`
- `concurrent_first_inbound_creates_one_customer` — 20 parallel, assert one row
- `inbound_on_closed_conversation_reopens_it`
- `blocked_customer_message_stored_but_not_routed`
- `routing_prefers_last_ordered_branch_within_60_days`
- `claim_is_atomic` — two users, one wins, other gets 409
- `least_busy_assignment_picks_lowest_open_count`
- `override_preserves_original_body_and_sets_overridden_by`
- `override_publishes_training_event`
- `bulk_approve_rejected_for_rx_linked_conversation`
- `canned_reply_unresolved_variable_blocks_send`
- `sse_scoped_to_user_branches_only`
- `sse_replay_from_last_event_id`
- `sla_timer_pauses_outside_opening_hours`
- `sla_breach_escalates_then_re_escalates`
- `send_outside_window_requires_template`

## 14. Done checklist

- [ ] Conversation lifecycle with reopen-on-inbound
- [ ] Customer auto-creation, race-safe
- [ ] Branch routing with the four-step precedence
- [ ] Three assignment strategies; atomic claim
- [ ] Override with original preserved and training event published
- [ ] Rx-linked conversations excluded from bulk approval
- [ ] Canned replies with strict variable resolution
- [ ] SSE with branch scoping, heartbeat, bounded replay
- [ ] SLA timers respecting opening hours, two-stage escalation
- [ ] All 16 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
