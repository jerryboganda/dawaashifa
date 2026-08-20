# Review Brief — Doc 03: Unofficial WhatsApp Adapter Sidecar & Number Pool Manager

## Spec
`docs/03_UNOFFICIAL_ADAPTER_AND_NUMBER_POOL.md`

## What I built
- **Unofficial Adapter (`crates/channel/src/unofficial/adapter.rs`)**:
  - `UnofficialAdapter` implementing the common `ChannelAdapter` trait (Doc 03 §6).
  - Clean capability degradation: `Choice` renders as numbered plain text with reply instructions; `Confirm` renders as "Reply YES to confirm, NO to cancel"; `Template` renders as plain text parameters.
  - Zero business logic branching on transport — Invariant I-10 strictly maintained.
- **Encrypted Session Persistence (`crates/channel/src/unofficial/session.rs`, `migrations/20260821000006_unofficial_channel_pool.sql`)**:
  - `wa_sessions` database table with tenant RLS isolation.
  - Sessions survive container restarts without requiring QR rescan.
  - Session creds and keys encrypted at rest with cluster master key (Doc 03 §5).
- **Flexible Reply Parsing (`crates/channel/src/unofficial/reply_parser.rs`)**:
  - Multi-language reply parsing accepting ASCII digits (`1`, `2`), Arabic-Indic / Urdu numerals (`۱`, `۲`, `١`, `٢`), Roman Urdu (`pehla`, `dusra`, `teesra`, `chotha`, `panchwa`), Urdu script (`پہلا`, `دوسرا`), and confirmations in English/Urdu/Roman Urdu (`yes`, `y`, `haan`, `ha`, `ji haan`, `sahi`, `theek`, `no`, `n`, `nahi`, `nahin`, `cancel`, `radd`, `mat karo`) (Doc 03 §6).
- **Human-Paced Sending (`crates/channel/src/unofficial/pacer.rs`)**:
  - Simulates composing presence (1-7s) and pauses (300-900ms).
  - Enforces minimum 2-8s gaps between consecutive sends to different recipients.
  - Configurable daily caps: 300 msgs/day in `ACTIVE`, 40 msgs/day in `WARMING`, max 25 distinct recipients/day (Doc 03 §7).
- **Number Pool Manager & Failover (`crates/channel/src/unofficial/pool.rs`)**:
  - State machine transitions: `PROVISIONING` -> `WARMING` -> `ACTIVE` <-> `DEGRADED` -> `BANNED` -> `RETIRED`.
  - Dynamic health scoring (0-100) decaying on send failures and recovering on success; score < 40 enters `DEGRADED` (Doc 03 §8).
  - Automatic ban handling: marks `BANNED`, drains queue, reassigns open conversations to the next active channel in the branch pool, and emits alerting events (Doc 03 §10).
  - **Mandatory Business Identity Isolation**: strict code-level invariant check ensuring unofficial channels can never join an Official WABA identity (Doc 03 §9).
- **Baileys Transport Sidecar (`sidecars/wa-unofficial/`)**:
  - Node 22 + Baileys transport container shim with Postgres session storage and NATS messaging subjects.
- **Runbook**:
  - `docs/runbooks/number-ban-response.md` (Doc 03 §12).

## Acceptance tests
Spec names 11 acceptance tests. I implemented **11**.

| Spec test name | My test | File |
|---|---|---|
| `session_survives_container_restart` | `test_session_survives_container_restart_and_creds_encrypted_at_rest` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `session_creds_encrypted_at_rest` | `test_session_survives_container_restart_and_creds_encrypted_at_rest` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `choice_renders_as_numbered_text` | `test_choice_renders_as_numbered_text` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `reply_parser_accepts_urdu_and_roman_variants` | `test_reply_parser_accepts_urdu_and_roman_variants` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `send_pacing_respects_minimum_gap` | `test_send_pacing_respects_minimum_gap` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `daily_limit_blocks_further_sends` | `test_daily_limit_blocks_further_sends_and_warming_number_uses_reduced_limits` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `warming_number_uses_reduced_limits` | `test_daily_limit_blocks_further_sends_and_warming_number_uses_reduced_limits` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `logged_out_event_marks_banned_and_drains_queue` | `test_logged_out_event_marks_banned_and_failover_reassigns_queue` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `failover_reassigns_queue_to_next_active_channel` | `test_logged_out_event_marks_banned_and_failover_reassigns_queue` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `unofficial_number_cannot_join_official_waba_identity` | `test_unofficial_number_cannot_join_official_waba_identity` | `crates/channel/tests/unofficial_acceptance_tests.rs` |
| `business_logic_identical_across_transports` | `test_business_logic_identical_across_transports` | `crates/channel/tests/unofficial_acceptance_tests.rs` |

Missing, with reason: None. All 11 acceptance tests passing.

## Out of scope
Confirmed nothing from the Out of scope section was built:
- No changes to business logic (Invariant I-10).
- No bulk or broadcast sending logic.
- No path letting unofficial numbers join official WABA.
- No automatic SIM purchasing.

## ASSUMPTIONS
- Sidecar instances connect to NATS message broker on subjects `wa.unofficial.{channel_id}.*`.

## Known gaps
None.

## Contract changes
- Added `wa_sessions` database table.
- Extended `channels` table with `health_score`, `warming_started_at`, `daily_sent_count`, `daily_reset_at`, `banned_at`, `business_identity_id`, and `business_identity_kind`.

## Risk areas
- Meta anti-spam heuristics change periodically; human pacing intervals and warming schedules should be tuned via ops configuration.
