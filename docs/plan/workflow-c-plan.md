# Plan — Workflow C: the human sets expectations, the harness holds the line

> Status: decided 2026-09-12. Nothing here is implemented yet; build order in § 10.

## 1. Division of labour

| Who     | Owns                                                                                             |
| ------- | ------------------------------------------------------------------------------------------------ |
| Human   | `docs/todo.md`: what is worth doing, its user value, its done-when. The **Next** queue.          |
| Human   | Validating a **design** before any change to what the user sees.                                 |
| Human   | Deciding **when a release is cut**. Several merged branches may wait for one release.            |
| AI      | `docs/techdebt.md`: every observation, smell and proposal. The human never edits it.             |
| AI      | Doing the task end to end: tests, code, review, merge. **No PR is validated by the human.**      |
| AI      | Coding the right way: logic in Rust, dumb frontend, gold layouts, typed errors.                  |
| Harness | Proving it, mechanically, on every pull request. Nothing merges that the harness has not passed. |

Four human touchpoints. Everything else is either the agent's job or a machine gate.

## 2. The two files

### `docs/todo.md` — human-owned

Keeps the current shape (`## #NNN`, User value, Done when) and gains:

```
## Next
#004, #006, #001, #005      ← the human's order; the agent only works this list

…per entry…
**Design:** none | proposed (screenshots/design/NNN-*.png) | validated
**Open questions:** none | - [ ] …
```

The agent writes to this file in three places only: it marks an entry `Shipped in
vX.Y.Z` (or `merged, unreleased`), it flips **Design** to `proposed`, and it adds open
questions. It never creates or reorders entries. An entry is **ready** when it is in
Next, has a Done when, has no open question, and its Design is `none` or `validated`.

### `docs/techdebt.md` — AI-owned

Every entry gets a stable `TD-NNN` reference so the human can put it in Next like any
todo (heading `## YYYY-MM-DD — TD-NNN — title`: the date stays first so the current
collector keeps parsing the file until phase 5 replaces it). The agent files there: reviewer findings it did not fix, smells met on the way,
proposals for new work, surviving mutants, coverage holes. The human reads it and
promotes what they want by queuing the reference.

## 3. Workflow C

One run = one entry from Next. Steps, all autonomous unless marked **human**:

1. **Pick** the first ready entry in Next. Branch `c/NNN-slug` off fresh `main`.
2. **Design gate** — if the entry changes what the user sees (new screen, moved or
   added control, changed layout or wording pattern): produce the design proposal
   (§ 4), set `Design: proposed`, commit that on `main` as docs, and **move on to the
   next ready entry**. **Human** looks at the images, edits the line to `validated`
   (or writes an open question). The entry becomes ready again.
3. **Acceptance first** — turn Done when into failing tests: Rust for logic, Vitest
   for rendering, E2E for what a user does. Test names carry `#NNN` (or the
   `TRIGRAM-NNN` rule when the domain has a spec; the agent writes the rule from Done
   when in the same commit). Confirm red.
4. **Implement** to green, the right way (§ 5). Logic that lands in the frontend is a
   harness failure, not a style remark.
5. **Self-check** — run the harness locally (`just harness`, § 6): lint, tests,
   coverage thresholds, architecture lints, golden fixture, visual proof.
6. **Self-review** — reviewer prompts run on the diff; the agent fixes every 🔴 and
   🟡 it can, files the rest as `TD-NNN`, comments false positives inline. Reviewers
   run again until no 🔴 remains. The reviewer prompts are project files: a recurring
   false positive is fixed in the prompt, in the same PR.
7. **PR** — opened for the record, not for approval. Body: entry ref, Done when with
   the proving test per clause, triage table, techdebt filed, screenshots. Commit title
   is the changelog line.
8. **Merge** when every required check is green. `just merge` refuses otherwise.
9. **Closure** in the same PR: entry marked `merged, unreleased`; techdebt updated;
   `ARCHITECTURE.md` if a module appeared.
10. **Next** entry. Stop when Next has no ready entry, or the budget rule (§ 7) fires.

**Release** — **human** runs `just release -y` on their machine when they choose.
It re-runs the full harness on `main`, computes the version from the merged titles,
writes the changelog, tags, pushes; CI builds and leaves the draft. The human
publishes. Entries flip to `Shipped in vX.Y.Z`.

## 4. Design proposal

Before any UI change the agent commits, under `screenshots/design/NNN-{state}-{light|dark}.png`,
rendered mocks of the target state built with the real components and tokens (the
visual-proof preview pipeline already renders components outside the app; the mock is
a preview of the proposed component tree with sample data). One image per changed
state, both themes, plus a five-line `screenshots/design/NNN.md`: what moves, what is
added, what is removed, what stays the same.

The human validates by editing the entry line. No chat needed. PNG mocks in the repo
are the medium: they work offline, diff in git, and sit next to the visual proofs.

## 5. The right way to code, as checks

Rules that today live in prose and reviewer judgement become mechanical where they can:

| Rule                                   | Mechanical check                                                                                                                                                                                                                                 |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Only `gateway.ts` calls `commands.*`   | `scripts/arch-check.py` A1: `commands.` outside a file named `*gateway.ts` → fail                                                                                                                                                                |
| No cross-feature imports               | A2: `features/a/**` importing `features/b/**` → fail; `features/shell/` is the composition root; the 18 known crossings are frozen in `arch-allowlist.json` and may only disappear                                                               |
| No cross-context imports in Rust       | A3: `crate::context::b` inside `context/a/**` outside `#[cfg(test)]` → fail                                                                                                                                                                      |
| Frontend is dumb                       | A7: `.reduce(` in feature code → fail; `Math.` uses frozen per file in the allowlist (display rounding, documented previews). Date construction and business conditionals stay reviewer judgement — no regex tells a default date from date math |
| Logic coverage                         | `scripts/coverage-gate.py` on the tarpaulin report: domain, application, service and use-case lines (no `api.rs`), floor in `coverage-gates.json` starting at the measured value and ratcheted up to the **90 %** target; fail under             |
| Frontend coverage                      | same script on the Vitest report: `src/features/**` minus gateways, **≥ 80 % lines**, fail under                                                                                                                                                 |
| Stable ids on interactive elements     | A5: raw `<button>`/`<input>`/`<a>` in feature code → fail (everything comes from `ui/`); A6: a `ui/` interactive component without `id=` → fail, today's 119 frozen per file in the allowlist                                                    |
| One notice per action on the event bus | Rust test helper asserting no two `send()` without an observed receive; reviewer prompt backs it                                                                                                                                                 |
| Typed errors on the wire               | A4: `Result<_, String>` in any `api.rs` → fail                                                                                                                                                                                                   |
| i18n: no literal user-facing strings   | A8: literal text in `aria-label`/`placeholder`/`title`/`label` in feature code → fail (the design-system dev page excepted); JSX text literals stay reviewer judgement — a regex cannot tell copy from code                                      |

What stays judgement (reviewer prompts): naming, UL terms, factory usage, test quality,
UX completeness. Those block on 🔴 only.

## 6. The harness

`just harness` locally and the same set as **required checks** on the PR. Nothing
merges without all of them.

| #   | Check                          | Exists today                     | Change                                                                                                                                                                                               |
| --- | ------------------------------ | -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Lint, format, type-check       | Quality                          | none                                                                                                                                                                                                 |
| 2   | Rust + Vitest unit/integration | Quality                          | none                                                                                                                                                                                                 |
| 3   | Coverage thresholds            | codecov comment, advisory        | `scripts/coverage-gate.py` after each coverage step (§ 5), codecov stays for the per-file view                                                                                                       |
| 4   | Architecture lints             | reviewer judgement               | `scripts/arch-check.py` A1–A8, own Quality job, `just arch-check`; frozen debt in `arch-allowlist.json` (ratchet: `--write-allowlist` only lowers)                                                   |
| 5   | E2E                            | main push only; local broken     | `pull_request` trigger with no path filter (a filtered-out run leaves a required check pending forever, so doc-only PRs run it too), required check                                                  |
| 6   | Real-app screenshots           | none                             | `e2e/helpers/screenshot.ts`; each spec captures its populated state in both themes; uploaded as an artifact and linked from a sticky PR comment (artifacts cannot be embedded inline)                |
| 7   | Visual regression              | screenshots committed, unchecked | pixel-diff previews against `main`; a diff fails unless the PR touches that component's image                                                                                                        |
| 8   | Golden portfolio               | none                             | committed synthetic portfolio; `src-tauri/tests/golden_portfolio.rs` compares valuation, performance, fees, price movement to `expected.json`; a moved number needs the entry's Done when to name it |
| 9   | Reviewer checks                | chat, on demand                  | `review.yml`: reviewer prompts run by the Claude GitHub action, 🔴 fails the check                                                                                                                   |
| 10  | Security audit                 | cargo audit weekly               | add to PR when `Cargo.lock`/`package-lock.json` change                                                                                                                                               |
| 11  | Commit hygiene                 | Quality pr-checks                | none                                                                                                                                                                                                 |
| 12  | Mutation sweep                 | none                             | monthly job; survivors filed as `TD-NNN` by the agent                                                                                                                                                |

Branch protection on `main`: required = 1–5, 7–9 (10 conditional). `scripts/merge.py`
gains a `gh pr checks` query and refuses when any required check is not green.

## 7. Budget and stop rules

- One entry, one run, a wall-clock budget (start at 3 h). Over budget → open question
  "larger than estimated: split?" on the entry, move on.
- Same gate failing three times on one entry → open question, move on.
- Never touch the live database, never force-push, never bypass a hook, never edit a
  released changelog line, never reorder Next.

## 8. Where it runs

The chat session stays for writing entries together and answering open questions.
The loop itself runs headless: `claude -p "/next-todo"` from a scheduled job. Cloud
routine preferred (E2E and screenshots come from CI, so the local L-011 breakage does
not matter); local cron works the same once L-011 is fixed. One run per entry; state
in git; no compaction, no resume checkpoints.

## 9. What is dropped

- Plan approval per task, `/smart-commit` and merge prompts, triage halts, the model
  switch, the A/B split, the spec → contract → plan → checklist ceremony for entries.
  Specs stay as the rule index; contracts stay as the wire record when a command
  changes; the agent writes both from Done when.
- The kit, for good. No sync ever again: manifest, version file, `sync-config.sh`,
  `sync-kit`, `/kit-discover` removed; `common.just` folded into `justfile`; the
  `kit-readme.md` and `kit-tools.md` references replaced by `docs/workflow-c.md`.
  Convention docs, agents, skills and scripts are project files, edited freely.
  Recorded as an ADR.
- Human PR review. Replaced by the harness plus the design gate before, and the
  release decision after.

## 10. Build order

Each phase is one PR under today's rules until the loop exists.

| #   | Phase                                                                                 | Files                                                                                                 |
| --- | ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 1   | E2E on PRs + screenshot helper + PR image comment                                     | `.github/workflows/e2e.yml`, `e2e/helpers/screenshot.ts`, one capture per critical spec               |
| 2   | Coverage thresholds as hard gates                                                     | `.github/workflows/quality.yml`, `vitest.config.ts`, `justfile` (`harness` recipe)                    |
| 3   | Architecture lints                                                                    | `scripts/arch-check.py`, wired in Quality and `just harness`                                          |
| 4   | Todo/techdebt file shapes: Next queue, Design and Open-questions lines, `TD-NNN` refs | `docs/todo.md`, `docs/techdebt.md`                                                                    |
| 5   | Workflow doc + CLAUDE.md rewrite + `/next-todo` skill + design-proposal recipe        | `docs/workflow-c.md`, `CLAUDE.md`, `.claude/skills/next-todo/SKILL.md`, `scripts/design-proposal.mjs` |
| 6   | Freeze the kit (ADR + deletions)                                                      | `.claude/kit-*`, `scripts/sync-config.sh`, `common.just`, `docs/adr/`                                 |
| 7   | Golden portfolio fixture                                                              | `src-tauri/tests/golden_portfolio.rs`, `src-tauri/tests/golden/`                                      |
| 8   | Reviewer checks in CI                                                                 | `.github/workflows/review.yml`                                                                        |
| 9   | Visual regression                                                                     | `scripts/visual-proof-compare.mjs`, Quality                                                           |
| 10  | Merge guard + branch protection                                                       | `scripts/merge.py` wrapper, repo settings                                                             |
| 11  | Scheduled runner                                                                      | cloud routine or `scripts/next-todo.sh` + cron                                                        |
| 12  | Mutation sweep                                                                        | `.github/workflows/mutants.yml`                                                                       |

First unattended run: after phase 5, on a copy-only entry (#004 or #006), the human
watching once. Phases 7–10 are what make merging money logic unattended acceptable;
until they land, the agent still merges alone but the human is told which entries
touched valuation, so they can read the golden numbers at release time.

## 11. Risks named plainly

- **Tests that encode a misunderstanding** pass the harness. Mitigations: the design
  gate for anything visible, the golden fixture for anything numeric, real-app
  screenshots on every PR for the human to glance at when they feel like it.
- **Prompt erosion**: editable reviewer prompts can be softened by a lazy false-positive
  edit. Every such edit is a visible diff; the mutation sweep is the independent check.
- **CI cost** roughly triples (E2E and reviewers per PR). Fine for a few PRs a day.
- **Large features** still deserve a design conversation in the chat before the entry
  is written. Workflow C removes ceremony from doing, not thinking.

## 12. Decisions (2026-09-12)

- Coverage: **90 % lines on Rust logic dirs** (`context/**/domain`,
  `context/**/application`, `use_cases/**`); 80 % lines on `src/features/**`.
- Design proposals are **committed PNG mocks** under `screenshots/design/`.
- Releases are cut **from the laptop** with `just release -y`; the human publishes
  the draft.
- The kit is **dropped for good**, not merely frozen. ADR in phase 6.

None — all questions have been resolved.
