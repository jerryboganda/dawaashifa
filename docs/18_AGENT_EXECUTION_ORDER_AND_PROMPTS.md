# DOC 18 — AGENT EXECUTION ORDER & PROMPT PACK

**Read this before starting any build.** It is the operating manual for the two-agent setup.

---

## 1. The setup

| | Backend | Frontend |
|---|---|---|
| Tool | GitHub Copilot (Grok 4.6) | Google Antigravity |
| Owns | `crates/**`, `migrations/**`, `sidecars/**` | `apps/**` |
| Config | `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md` | `.agents/rules/*.md`, `.agents/workflows/*.md` |
| Shared | `AGENTS.md` — wins on any conflict | same |

Both read `AGENTS.md`. Neither edits the other's directories.

## 2. The failure mode this design prevents

Two agents that cannot see each other's work will drift on the API contract. Copilot renames a field; Antigravity's client still expects the old one; nothing surfaces until integration, by which point both have built on the mistake.

The prevention has three parts, all mandatory:

1. **`contracts/openapi.json` is generated**, not written. Copilot regenerates it in the same PR as any route change.
2. **`apps/shared/src/api/` is generated** from that file. Antigravity never hand-writes an API type.
3. **CI fails on drift** in either direction. This gate is not optional and must never be skipped to unblock a build.

When Antigravity needs a field that does not exist, the correct action is to **stop and report**, not to create a local type. A local workaround type is the exact failure this design exists to prevent.

## 3. Execution order

Sequential unless marked parallel. Do not start a spec whose dependencies are unmerged.

| Order | Spec | Agent | Notes |
|---|---|---|---|
| 1 | 01 Domain model | Backend | Everything blocks on this |
| 2 | 04 Identity & RBAC | Backend | |
| 3 | 02 Channel & Cloud API | Backend | |
| 4 | 05 Catalog & matching | Backend | ∥ with 06 |
| 5 | 06 Inventory ledger | Backend | ∥ with 05 |
| 6 | 07 Conversation engine | Backend | |
| 7 | 10 Orders & routing | Backend | **Revenue loop closes here** |
| 8 | 16 Console — shell, inbox, orders | Frontend | Starts once 07 and 10 merge |
| 9 | 08 AI orchestration | Backend | ∥ with 8 |
| 10 | 09 Prescription workflow | Backend | |
| 11 | 16 Console — Rx review, payment review | Frontend | |
| 12 | 11 Payments | Backend | |
| 13 | 12 Fulfilment backend | Backend | |
| 14 | 12 Rider PWA | Frontend | After 13 merges |
| 15 | 13 FBR & tax | Backend | |
| 16 | 15 Data migration | Backend | Needs real sample exports |
| 17 | 03 Unofficial adapter | Backend | Deliberately late |
| 18 | 14 B2B module | Backend | |
| 19 | 16 Console — B2B, reports, audit | Frontend | |
| 20 | 17 Deployment & observability | Backend | Continuous, finalised here |

**Doc 16 is split across three frontend passes.** Do not attempt the whole console in one branch.

## 4. Prompt pack

### 4.1 Copilot — backend kickoff

```
Read AGENTS.md and .github/copilot-instructions.md in full, then read
docs/01_DOMAIN_MODEL_AND_MIGRATIONS.md.

Build exactly what Doc 01 specifies. Before writing code, produce a plan listing
every file you will create and which acceptance test each satisfies. Wait for my
confirmation of the plan.

Constraints you must not violate:
- Every table gets tenant_id and an RLS policy
- Money is NUMERIC(14,4) and rust_decimal::Decimal, never a float
- Migrations are forward-only
- Build nothing from the "Out of scope" section

When done, run: cargo clippy --workspace --all-targets -- -D warnings,
cargo test --workspace, cargo sqlx prepare --workspace. All must be clean.
```

### 4.2 Copilot — subsequent specs

```
Read AGENTS.md, then docs/{NN}_{NAME}.md.

Confirm its "Depends on" specs are merged on main. If any is missing, stop and
tell me rather than stubbing it.

Plan first, listing files and mapping each to an acceptance test. Then implement.

Do not build anything in "Out of scope". Follow the Contracts section exactly —
do not improve the schema or API shape.

If you change any route or DTO, run cargo run -p api --bin emit-openapi and
commit the regenerated contracts/openapi.json in the same PR.

Finish with the full verification set and confirm every item in the spec's
Done checklist.
```

### 4.3 Antigravity — frontend kickoff

```
/execute-spec 16

Build only the shell, navigation, and the unified inbox in this pass. Leave
prescription review, payment review, B2B, and reports for later passes.

Before starting, run pnpm gen:api. Do not hand-write any API type — everything
comes from @dawaa/shared.

The inbox must handle loading, empty, and error states, virtualise the
conversation list, and render correctly in Urdu RTL. Verify all three locales
before you consider it done.
```

### 4.4 Antigravity — the two critical screens

```
/execute-spec 16

This pass builds the prescription review and payment review screens only.

Read docs/16_OPS_CONSOLE.md sections 7 and 8 carefully, and read
docs/09_PRESCRIPTION_WORKFLOW.md section 8 for the approval rules.

Hard requirements:
- No bulk-approve control on either screen. Not one.
- Approve stays disabled until every prescription line has an explicit decision
- Duplicate transaction ID renders as an unmissable critical banner
- The full prescription review flow must be completable by keyboard alone

These two screens set the throughput ceiling for the whole business. Every extra
click costs real money. Optimise accordingly.
```

### 4.5 Cross-agent handoff

When backend finishes a spec the frontend needs:
```
Doc {NN} is merged. contracts/openapi.json is updated.
Run pnpm gen:api and confirm the new endpoints appear in @dawaa/shared
before starting frontend work.
```

When frontend hits a missing field:
```
Doc {NN} needs field {X} on {endpoint}, which is not in the generated client.
Do not add a local type. Report it and I will task the backend agent.
```

## 5. Review protocol

Neither agent merges its own PR. Before merging, check:

- [ ] Nothing from the spec's Out of scope was built
- [ ] The spec's Done checklist is fully ticked
- [ ] `contracts/openapi.json` regenerated if routes changed
- [ ] No new table without `tenant_id`, RLS and indexes
- [ ] No migration edited, only added
- [ ] No test skipped, ignored or deleted to get green
- [ ] No invariant weakened
- [ ] `## ASSUMPTIONS` in the PR description reviewed and accepted

The assumptions section matters most. It is where an agent tells you what it guessed — and where a wrong guess is cheapest to catch.

## 6. When an agent goes wrong

Common failure patterns and the fix:

| Pattern | Fix |
|---|---|
| Builds out-of-scope features | Re-read the Out of scope section aloud in the prompt |
| Invents an API shape | Point at the Contracts section; make it quote the shape back |
| Skips a failing test | Reject the PR; restate the never-skip rule |
| Edits a merged migration | Reject; require a new forward migration |
| Hand-writes an API type | Reject; run `pnpm gen:api` and re-prompt |
| Loops on the same error | Enforce the three-attempt stop rule |
| Adds bulk approve to Rx | Reject immediately; this is a safety invariant, not a preference |

## 7. What to do first

1. Rename `Dawaa` throughout to your final product name
2. Fill the open items in `docs/00` §16
3. `git init`, commit this kit as the first commit — the config files must exist before any agent runs
4. Open `docs/01` and run the prompt in §4.1

The agents read these files on first load. Committing them first is what makes the whole thing work.
