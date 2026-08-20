# CLAUDE.md — Dawaa Platform

**Read `AGENTS.md` at the repo root. It is the source of truth.** This file adds only Claude Code specifics.

## Working style on this repo
- Work is defined by numbered spec docs in `docs/`. One spec, one branch, one PR.
- Before implementing, read the spec's **Depends on** and **Out of scope** sections. Building out-of-scope items is a defect.
- Use TodoWrite to track multi-step spec execution. Mark items complete as you go.
- Prefer targeted edits over rewriting whole files. Never rewrite a file wholesale unless explicitly asked.

## Agent split on this project
- Backend (`crates/**`, `migrations/**`, `sidecars/**`) is normally built in GitHub Copilot.
- Frontend (`apps/**`) is normally built in Google Antigravity.
- If you are asked to work on either side, respect the same boundaries and the same API contract protocol described in `AGENTS.md` §5.

## Before you finish
Run the full verification set from `AGENTS.md` §7 and confirm every box in §10.
