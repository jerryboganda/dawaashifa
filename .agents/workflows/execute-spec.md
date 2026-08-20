---
description: Execute one numbered spec document from docs/ end to end, producing a single reviewable PR.
---

# Workflow: Execute a spec document

Invoke with the spec number, e.g. `/execute-spec 16`.

## Step 1 — Load context
1. Read `@/AGENTS.md` in full.
2. Read `@/docs/00_MASTER_ARCHITECTURE_BLUEPRINT.md` sections 1–4.
3. Read the target spec `docs/{NN}_*.md` in full.
4. Read `contracts/openapi.json` if the spec touches API-backed screens.

## Step 2 — Verify dependencies
Check the spec's **Depends on** list. For each dependency, confirm the corresponding code exists on `main`.

If any dependency is missing, **stop here** and report which one. Do not stub it out and continue.

## Step 3 — Plan
Produce a written plan before touching any file:
- Every file you will create, with its path
- Every file you will modify
- Which acceptance test from the spec each change satisfies
- Anything in the spec you judge ambiguous

Present the plan. Do not begin implementation until it is coherent and complete.

## Step 4 — Regenerate the API client
```bash
pnpm gen:api
```
If this changes files under `apps/shared/src/api/`, the backend contract moved. Re-read the spec's Contracts section against the regenerated types before continuing.

## Step 5 — Branch
```bash
git checkout -b feat/{NN}-{slug}
```

## Step 6 — Implement
- Follow the spec's Contracts section exactly. Do not improve the shape.
- Build only what is in scope. Consult the **Out of scope** list whenever you are tempted to add something.
- Commit in logical increments with meaningful messages. Not one giant commit.

## Step 7 — Verify
```bash
pnpm check      # svelte-check — must be clean
pnpm lint       # must be clean
pnpm test       # must be green
pnpm -F console build
```
All four must pass. If a test fails, fix the code. **Never skip, ignore, or delete a test to get green.**

## Step 8 — Manual verification
For any customer-facing or pharmacist-facing screen:
- Render in `en`, `ur` (RTL), and `ur-Latn`
- Verify loading, empty, and error states each render correctly
- Verify keyboard navigation on any screen with a review or approval action
- On rider PWA work, verify behaviour with the network throttled to offline

## Step 9 — Self-review against the invariants
Re-read `.agents/rules/00-core-invariants.md`. Confirm explicitly:
- No prescription can be approved without an explicit pharmacist action
- No payment screenshot can be approved in bulk or automatically
- No money value was parsed into a JavaScript number
- No API type was hand-written
- Nothing under `crates/**` or `apps/shared/src/api/**` was edited

## Step 10 — Open the PR
The description must contain:
- **Spec:** which doc this implements
- **Built:** what was delivered
- **Skipped:** anything in scope you did not complete, and why
- **ASSUMPTIONS:** every ambiguity you resolved by judgement
- **Backend needed:** anything the backend agent must add for this to work fully

Do not merge your own PR.

## Stop conditions
Halt and report rather than continuing if:
- The same failure has resisted three attempts
- The spec contradicts `AGENTS.md` or the blueprint
- The work would require editing backend code
- A change would violate an invariant
