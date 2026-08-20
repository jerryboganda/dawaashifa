---
description: Self-review a completed spec branch and emit REVIEW_BRIEF.md for the reviewer agent.
---

# Workflow: Prepare for review

Run after implementation is complete and all local checks pass. Produces the handoff artifact the reviewer reads.

## Step 1 — Confirm the machine gates are green
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo sqlx prepare --workspace --check
cargo run -p api --bin emit-openapi && git diff --exit-code contracts/openapi.json
pnpm gen:api && git diff --exit-code apps/shared/src/api/
pnpm check && pnpm lint && pnpm test
```
**Do not continue until every one passes.** Sending a red branch to review wastes a premium request and produces noise.

## Step 2 — Self-audit against the spec
Open the spec's acceptance-tests section. For each named test:
- Locate your implementation of it
- Read your own assertions and ask whether they genuinely test the named behaviour
- If a test only asserts `is_ok()` where the spec named a specific behaviour, strengthen it now

Count them. You will report the count in the brief and the reviewer will verify it.

## Step 3 — Self-audit against Out of scope
Re-read the spec's Out of scope section. Search the diff for anything matching. Remove it.

## Step 4 — Invariant sweep
```bash
rg "UPDATE stock_current" --glob '!*trigger*'      # must be empty
rg "f64|f32" crates/ --glob '*.rs' | rg -i "price|total|amount|money"   # must be empty
rg "tenant_id" crates/*/src/**/dto.rs               # must not appear in request DTOs
rg "unwrap\(\)|expect\(" crates/ --glob '!*test*' --glob '!*main.rs'
git diff --name-only origin/main -- migrations/ | xargs -r git diff origin/main --  # no edits to existing files
```

## Step 5 — Write REVIEW_BRIEF.md
```markdown
# Review Brief — Doc {NN}

## Spec
docs/{NN}_{NAME}.md

## What I built
- bullet per feature, mapped to the spec section

## Acceptance tests
Spec names {N} tests. I implemented {M}.
| Spec test name | My test | File |
|---|---|---|
Missing, with reason: (none, or list)

## Out of scope
Confirmed nothing from the Out of scope section was built.
Specifically checked: (list the items)

## ASSUMPTIONS
Every ambiguity I resolved by judgement. Be exhaustive — this is where a wrong
guess is cheapest to catch.

## Known gaps
Anything incomplete, and why.

## Contract changes
Routes/DTOs changed: (list, or none)
openapi.json regenerated: yes/no
Generated client regenerated: yes/no

## Risk areas
Where I am least confident, and where I would look first if something broke.
```

Be honest in the brief. The reviewer verifies every claim in it, so an inflated brief only costs you a round.

## Step 6 — Open the PR
Title `feat(NN): {short description}`. Body links the spec and includes the brief inline.

**Do not merge.** Request review, then stop.
