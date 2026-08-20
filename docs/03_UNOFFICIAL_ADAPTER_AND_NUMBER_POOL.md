# DOC 03 — UNOFFICIAL ADAPTER SIDECAR & NUMBER POOL MANAGER

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 02
**Produces:** `sidecars/wa-unofficial`, `crates/channel/src/unofficial/`
**Branch:** `feat/03-unofficial-adapter`

> **Build this late.** It is Phase P6 in the blueprint. Cloud API is the production transport; this is a fallback. Building it early tempts teams to depend on it.

---

## 1. Objective

A second `ChannelAdapter` implementation backed by Baileys, plus a number pool manager that detects bans and fails over. Business logic must remain unable to tell which transport is in use.

## 2. In scope

- Node 22 + Baileys sidecar, one container per phone number
- Auth state persisted to Postgres, not local disk
- NATS bridge between sidecar and Rust core
- `UnofficialAdapter` implementing `ChannelAdapter`
- Capability degradation for everything Cloud API can do that Baileys cannot
- Number pool: health scoring, ban detection, warming, failover
- Business identity isolation enforcement

## 3. Out of scope — do NOT build

- Any change to business logic to accommodate this transport
- Bulk or broadcast sending (guarantees a ban)
- Any path that lets an unofficial number join the official WABA
- Automatic number purchasing

## 4. Why a sidecar

Baileys and whatsapp-web.js are Node-only. There is no Rust implementation. The sidecar is a thin transport shim; **no business logic lives in it.**

```
┌────────────────┐   NATS    ┌──────────────────────┐   WS   ┌──────────┐
│  Rust core     │◄─────────►│ wa-unofficial:{msisdn}│◄──────►│ WhatsApp │
│ UnofficialAdapter│         │  Node 22 + Baileys    │        └──────────┘
└────────────────┘           └──────────────────────┘
```

NATS subjects:
- `wa.unofficial.{channel_id}.outbound` — core → sidecar
- `wa.unofficial.{channel_id}.inbound` — sidecar → core
- `wa.unofficial.{channel_id}.status` — connection state, ban events
- `wa.unofficial.{channel_id}.qr` — QR payload for pairing

## 5. Auth state persistence

Baileys' default `useMultiFileAuthState` writes to disk. **Do not use it.** A container restart would require a physical QR rescan on a phone.

```sql
wa_sessions(channel_id UUID PRIMARY KEY, tenant_id UUID NOT NULL,
            creds JSONB NOT NULL, keys JSONB NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL)
```

Implement a custom `AuthenticationState` backed by this table. Write on every `creds.update`. Encrypt `creds` and `keys` at rest with a key from env — session data is equivalent to a logged-in WhatsApp account.

## 6. Capability degradation

```rust
Capabilities {
    interactive_buttons: false,   // Baileys button support is unreliable
    list_messages: false,
    templates: false,             // no such concept
    outside_window: true,         // possible, but raises ban risk
    delivery_receipts: true,      // best-effort, do not depend on it
    max_send_rate_per_min: 12,    // deliberately low
    max_buttons: 0,
}
```

Rendering:
| Intent | Cloud API | Unofficial |
|---|---|---|
| `Choice` any size | buttons / list | numbered text, reply with a number |
| `Confirm` | two buttons | "Reply YES to confirm, NO to cancel" |
| `Template` | template send | plain text rendered from `body_text` |

Reply parsing must accept: `1`, `١` (Arabic-Indic), `option 1`, `pehla`, `haan`, `ha`, `yes`, `y`, `nahi`, `no`, `n`. Case and whitespace insensitive.

## 7. Human-paced sending

Machine-speed sending triggers bans faster than volume does.

```
for each outbound message:
  1. presence 'composing' for (body.length / 12) seconds, clamped 1–7s
  2. presence 'paused', wait 300–900ms jitter
  3. send
  4. wait 2–8s jitter before the next message to any recipient
```

Hard limits per number per day, configurable, defaulting to:
- 300 messages/day during `ACTIVE`
- 40 messages/day during `WARMING`
- No more than 25 distinct new recipients/day

Never send two messages to different recipients within 2 seconds. Never send anything resembling a broadcast.

## 8. Number pool

```sql
-- extends channels from Doc 01
channels(..., transport, status, business_identity_id, health_score,
         warming_started_at, daily_sent_count, daily_reset_at, banned_at)
```

State machine:
```
PROVISIONING → WARMING → ACTIVE ⇄ DEGRADED → BANNED → RETIRED
```

- `WARMING`: 7–14 days at reduced limits before promotion to `ACTIVE`
- `DEGRADED`: entered on repeated send failures or a `428`/`503` pattern; halves the rate limit and alerts ops
- `BANNED`: entered on `connection.update` with `DisconnectReason.loggedOut`, or `403`. Drain the queue, reassign open conversations, alert ops immediately.

Health score (0–100) decays on failures, recovers on successes. Below 40 → `DEGRADED`.

## 9. Business identity isolation — mandatory

**A ban on an unofficial number can cascade to a linked WABA and take out the paid channel.**

```rust
// Enforced in the pool manager, at insert and update
if channel.transport == Transport::Unofficial
   && identity.kind == IdentityKind::OfficialWaba {
    return Err(ChannelError::IdentityIsolationViolation);
}
```

Operational requirements documented in the runbook:
- Unofficial numbers registered under a separate Meta Business Manager, never the WABA's
- Separate egress IP range from the Cloud API integration
- Never add an unofficial number to the WABA
- Never use the same device fingerprint across both

## 10. Failover

On ban:
1. Mark `BANNED`, stop the container.
2. Move queued outbound messages to the next `ACTIVE` channel in the same branch's pool.
3. Reassign open conversations to the new channel.
4. Emit `channel.banned` for ops alerting.

**What failover cannot fix, and must be documented in the UI:** customers have the banned number saved in their phones. Messages they send to it are lost. Failover preserves outbound continuity only. The runbook must cover proactively publishing the new number through other channels.

## 11. Acceptance tests

- `session_survives_container_restart` — no QR rescan needed
- `session_creds_encrypted_at_rest`
- `choice_renders_as_numbered_text`
- `reply_parser_accepts_urdu_and_roman_variants` — table-driven, all listed forms
- `send_pacing_respects_minimum_gap`
- `daily_limit_blocks_further_sends`
- `warming_number_uses_reduced_limits`
- `logged_out_event_marks_banned_and_drains_queue`
- `failover_reassigns_queue_to_next_active_channel`
- `unofficial_number_cannot_join_official_waba_identity` — asserts the error
- `business_logic_identical_across_transports` — same `OutboundMessage` through both adapters produces equivalent `messages` rows differing only in rendering

## 12. Done checklist

- [ ] Sidecar containerised, one instance per number, restart-safe
- [ ] Auth state in Postgres, encrypted
- [ ] `UnofficialAdapter` implements `ChannelAdapter` in full
- [ ] All capability degradations implemented; no business-logic branching
- [ ] Human-paced sending with jitter and daily caps
- [ ] Pool state machine with warming, health scoring, ban detection, failover
- [ ] Identity isolation enforced in code, not just documentation
- [ ] Runbook committed at `docs/runbooks/number-ban-response.md`
- [ ] All 11 acceptance tests green
