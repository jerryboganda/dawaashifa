# DOC 19 — BUILDER / REVIEWER PROTOCOL

**Supersedes the agent split in Doc 18 §1 and §3.** Execution order in Doc 18 §3 still applies; only the role assignment changes.

---

## 1. The new setup

| | Builder | Reviewer |
|---|---|---|
| Tool | Google Antigravity | GitHub Copilot, Grok 4.6 high-thinking |
| Model posture | Fast iteration, wide edits | Slow, deep, adversarial |
| Owns | All of `crates/`, `apps/`, `migrations/`, `sidecars/` | No ownership — reviews and fixes |
| Writes | Feature branches | Fix commits on the same branch |
| Config | `.agents/rules/*`, `.agents/workflows/*` | `.github/instructions/review.instructions.md` |

Both still obey `AGENTS.md`.

## 2. Why this ordering works

Antigravity iterates quickly and produces a lot of code. Grok at high thinking is expensive per token but catches what fast iteration misses. Putting the cheap agent first and the expensive one second is the correct economics — **provided you never spend Grok's reasoning on something a compiler could have caught for free.**

Hence the pipeline in §3: deterministic gates first, Grok last.

## 3. The pipeline

```
Antigravity builds
   ↓
[GATE 1 — machine, free, seconds]
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   pnpm check && pnpm lint && pnpm test
   contract drift check (openapi.json + generated client)
   ↓ FAIL → back to Antigravity, Grok never sees it
   ↓ PASS
[GATE 2 — Antigravity self-review]
   /prepare-review → produces REVIEW_BRIEF.md
   ↓
[GATE 3 — Grok 4.6, high thinking]
   reviews diff against spec + rubric
   emits findings with severities
   ↓
BLOCKER / HIGH → Grok fixes mechanically, or returns to Antigravity for design changes
MEDIUM → logged as issues, not fixed now
LOW → discarded
   ↓
[GATE 1 re-runs on the fix commit]
   ↓
Merge
```

**Gate 1 is non-negotiable and must run before Grok is invoked.** A PR that does not compile wastes a premium request and produces a review full of noise about symbols that do not exist.

## 4. What Grok is actually for

Tests catch functional bugs. Clippy catches sloppiness. Grok is for the class of defect that **passes every automated check and is still wrong.**

Concretely, in this codebase:

| Defect class | Why tests miss it |
|---|---|
| Repository method missing `AND tenant_id = $n` | Single-tenant tests pass fine |
| `tenant_id` read from request body | Works perfectly until someone forges it |
| An auto-approval path no test happens to exercise | Absence of a test is not a failing test |
| Test exists but asserts nothing meaningful | Green is green |
| A previously-passing test weakened to pass | Diff shows green, not the weakening |
| Read-then-write without a transaction or lock | Serial tests never race |
| Out-of-scope feature built | Nothing tests for absence |
| Migration edited rather than added | Applies cleanly on a fresh DB |
| `f64` sneaking into a money path | Rounds correctly on small numbers |
| Error swallowed with `let _ =` or `.ok()` | Happy path unaffected |
| N+1 query inside a loop | Fast on 50 seed rows |
| Schema deviating from the spec's Contracts section | Self-consistent code, wrong shape |

The rubric in `.github/instructions/review.instructions.md` enumerates these as an explicit checklist. Do not let the reviewer freelance — an unanchored reviewer produces style opinions, which cost time and change nothing.

## 5. The blind spot you must design around

**If Antigravity misreads a spec, Grok reading the same spec may misread it identically.** Two models agreeing is not verification; it can be correlated error.

The anchor is the **acceptance tests enumerated in each spec document.** They are concrete, countable, and independently checkable. Grok's first task on every review is mechanical:

1. List every acceptance test named in the spec
2. For each, locate the corresponding test in the diff
3. For each found, verify the assertions actually test the named behaviour
4. Report any missing, and any present-but-vacuous

This is a checklist comparison, not an interpretation. It is the one part of the review that is immune to shared misreading.

## 6. Severity rules — the loop killer

| Severity | Definition | Action |
|---|---|---|
| **BLOCKER** | Violates an invariant, security hole, data loss risk, missing acceptance test | Must fix before merge |
| **HIGH** | Correctness bug, race condition, spec contract deviation, out-of-scope feature | Must fix before merge |
| **MEDIUM** | Performance, maintainability, missing edge case | File an issue, do not fix now |
| **LOW** | Style, naming, preference | **Discard. Do not report.** |

Without this table, Grok will produce forty comments about naming and Antigravity will churn on them for an hour. State explicitly in the review prompt: **do not report LOW findings at all.**

## 7. Who fixes what

| Finding type | Fixer | Why |
|---|---|---|
| Missing tenant filter, `f64`, swallowed error, missing test, missing index | **Grok** | Mechanical, local, low risk |
| Migration edited | **Grok** | Revert plus new forward migration |
| Out-of-scope feature built | **Grok** | Deletion |
| Wrong architecture, wrong state machine, spec misread | **Antigravity** | Needs the builder's context; a reviewer rewriting design produces code nobody owns |

Grok's fixes go in a **separate commit** (`fix(review): ...`) so the diff between what was built and what was corrected stays visible. That diff is your data on where Antigravity is systematically weak — after ten PRs you will see the pattern and can add a rule to `.agents/rules/` to prevent it at the source.

## 8. Round limit

**Maximum two review rounds per PR.**

If a third round is needed, the problem is the specification, not the code. Stop, fix the spec, restart the branch. Three rounds means the two agents are negotiating with each other, which produces motion without progress.

## 9. Merge gating

Copilot's PR review posts comments only; it cannot block a merge. Two options:

**Manual (recommended for a single operator):** do not merge until the review comment reports zero BLOCKER and zero HIGH. Simple, no machinery.

**Automated:** have Grok emit a fenced `review-verdict` JSON block, and add a CI job that parses it and fails on any BLOCKER or HIGH. Make that job a required status check in branch protection.

```json
{ "spec": "10", "blocker": 0, "high": 2, "medium": 5,
  "acceptance_tests_expected": 19, "acceptance_tests_found": 17,
  "verdict": "CHANGES_REQUIRED" }
```

The `acceptance_tests_expected` vs `found` comparison is the single most valuable number in the whole pipeline. A gap there is the most common real defect.

## 10. PR size discipline

**Cap diffs at roughly 800 lines.** A reviewer given 4,000 lines skims, and a skimming reviewer is worse than no reviewer because it produces false confidence.

Docs 01, 12 and 16 exceed this. Split them:

| Spec | Sub-branches |
|---|---|
| 01 Domain model | `01a` workspace + core types · `01b` migrations tenancy/identity/catalog · `01c` migrations inventory/orders/payments · `01d` RLS + seed + tests |
| 12 Fulfilment | `12a` backend · `12b` rider PWA |
| 16 Console | `16a` shell + inbox · `16b` Rx + payment review · `16c` remaining screens |

## 11. Configuration changes from the original kit

1. `.agents/rules/00-core-invariants.md` — the "you are the frontend agent, do not edit `crates/**`" restriction is **removed**. Antigravity now owns everything.
2. `.agents/rules/20-backend.md` — **new**, glob-scoped to `crates/**` and `migrations/**`, carrying the Rust and SQL rules previously in the Copilot instruction files.
3. `.agents/workflows/prepare-review.md` — **new**, produces `REVIEW_BRIEF.md`.
4. `.github/instructions/review.instructions.md` — **new**, the review rubric.
5. `.github/workflows/copilot-setup-steps.yml` — **new**, gives the reviewer a Rust and Node toolchain so it can compile and run tests.
6. `.github/copilot-instructions.md` — rewritten from builder config to reviewer config.

The contract-drift gate from `AGENTS.md` §5 **still applies**, even though one agent now writes both sides. It is what stops a route change from silently breaking the generated client, regardless of who made the change.

## 12. The prompt pair

### Antigravity — build
```
/execute-spec {NN}

Build only what the spec covers. Consult the Out of scope section whenever you
are tempted to add something.

When implementation is complete and pnpm/cargo checks pass, run /prepare-review
to produce REVIEW_BRIEF.md, then open the PR.

Do not merge. A reviewer will pass over this before merge.
```

### Grok — review
```
Review this PR as an adversarial reviewer.

Read in this order:
1. .github/instructions/review.instructions.md — your rubric
2. docs/{NN}_{NAME}.md — the spec this implements
3. REVIEW_BRIEF.md — the builder's own account of what it did
4. The diff

Start with the acceptance-test audit in rubric section 1. List every acceptance
test the spec names, locate each in the diff, and verify the assertions actually
test the named behaviour. Report any missing, and any present but vacuous.

Then work the rubric checklist in order.

Report BLOCKER and HIGH findings only. Log MEDIUM as a list at the end.
Do not report LOW findings at all — no style, no naming, no preference.

For each finding give: severity, file and line, what is wrong, why it matters
here specifically, and the minimal fix.

End with the review-verdict JSON block.

Treat REVIEW_BRIEF.md as a claim to verify, not as evidence. If the brief says a
test exists, confirm it exists and confirm what it asserts.
```

That last line matters. The brief is the builder's self-report, and a self-report is exactly the thing a reviewer exists to check.

## 13. What to watch in the first ten PRs

Track these; they tell you whether the loop is working:

- **Findings per PR, by severity.** Should fall over time. If flat, your rules are not improving.
- **`acceptance_tests_found` vs `expected`.** Persistent gaps mean Antigravity is skipping tests — add a harder rule.
- **Second-round rate.** Above 50% means specs are ambiguous, not that the builder is weak.
- **Repeat finding types.** The same BLOCKER three times is a missing rule in `.agents/rules/`, not a stubborn agent.

The fix commits are your dataset. Read them.
