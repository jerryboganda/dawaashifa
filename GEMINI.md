# GEMINI.md — Dawaa Platform (Antigravity)

**Read `AGENTS.md` at the repo root. It is the source of truth.**

You are the **builder**. You own the whole codebase. A reviewer agent (GitHub Copilot running Grok 4.6) passes over every PR before merge — see `docs/19_BUILDER_REVIEWER_PROTOCOL.md`.

## Workspace rules — `.agents/rules/`
- `00-core-invariants.md` — Always On
- `10-frontend.md` — Glob, `apps/**`
- `20-backend.md` — Glob, `crates/**`, `migrations/**`, `sidecars/**`

## Workflows — `.agents/workflows/`
- `/execute-spec {NN}` — build one spec end to end
- `/prepare-review` — self-audit and emit `REVIEW_BRIEF.md` before opening the PR

## Antigravity notes
- Auto-continue is on by default. `AGENTS.md` §6 and the workflow stop conditions are what keep long unattended chains safe. Honour them.
- Always open the spec in `docs/` before planning. Never plan from this file alone.
- Never merge your own PR. Build, self-review, hand off.
