# Workflow C — the human sets expectations, the harness holds the line

The operating manual for how work moves from `docs/todo.md` to a release. Decided
on 2026-09-12 (`docs/plan/workflow-c-plan.md`); this document is the rule set,
the plan is the history.

## 1. Who owns what

| Who     | Owns                                                                                             |
| ------- | ------------------------------------------------------------------------------------------------ |
| Human   | `docs/todo.md`: what is worth doing, its user value, its done-when, and the **Next** queue.      |
| Human   | Validating a **design** before anything the user sees changes.                                   |
| Human   | Cutting a **release**. Several merged branches may wait for one.                                 |
| Agent   | `docs/techdebt.md`: every observation, smell and proposal. The human queues from it.             |
| Agent   | The task, end to end: tests, code, review, merge. **No pull request is validated by a human.**   |
| Agent   | Coding the right way: logic in Rust, dumb frontend, gold layouts, typed errors, stable ids.      |
| Harness | Proving it, mechanically, on every pull request. Nothing merges that the harness has not passed. |

Four human touchpoints. Everything else is either the agent's job or a machine gate.

## 2. The two files

### `docs/todo.md` — human-owned

- `## Next` at the top holds the queue: `#NNN` and `TD-NNN` references in the order to
  work them. The agent takes the first **ready** one and never edits the list.
- Every entry ends with `**User value:**`, `**Done when:**`, `**Design:**` and
  `**Open questions:**`.
- **Ready** means: queued, has a Done when, `Open questions: none`, and `Design` is
  `none` (no proposal needed yet) or `validated`.
- The agent writes to this file in three places only: it flips `Design` to
  `proposed (…)`, it adds open questions, and it marks an entry `merged, unreleased`
  then `Shipped in vX.Y.Z`. It never creates, removes or reorders entries.

### `docs/techdebt.md` — agent-owned

- Every entry carries a permanent `TD-NNN` reference (`## YYYY-MM-DD — TD-NNN — title`).
- The agent files here: reviewer findings it did not fix, smells met on the way,
  proposals for new work, surviving mutants, coverage holes, frozen architecture debt.
- Entries are observations, not commitments. The human promotes one by queuing its
  reference in Next.

## 3. The loop — one run, one entry

1. **Pick** the first ready entry in Next. Branch `c/NNN-slug` (or `c/td-NNN-slug`)
   off fresh `main`.
2. **Design gate** — if the entry changes what the user sees (a new screen, a moved or
   added control, a changed layout or wording pattern): produce the proposal (§ 4), set
   `Design: proposed (screenshots/design/NNN-*.png)`, merge that as a docs change, and
   **move on to the next ready entry**. The human validates by editing the line.
3. **Acceptance first** — turn Done when into failing tests: Rust for logic, Vitest for
   rendering, E2E for what a user does. Test names carry `#NNN` (or the `TRIGRAM-NNN`
   rule when the domain has a spec; the agent writes the rule from Done when in the
   same commit). Confirm red.
4. **Implement** to green, the right way (§ 6). Logic that lands in the frontend is a
   harness failure, not a style remark.
5. **Self-check** — `just harness`: architecture rules, lint, type-check, build, both
   suites with coverage, the coverage floors.
6. **Self-review** — run the reviewer agents that match the diff (§ 7); apply the
   triage policy; run them again until no 🔴 remains.
7. **Evidence** — `/visual-proof` for every changed component; the E2E run in CI
   captures the real app and links the screenshots from the pull request.
8. **PR** — opened for the record, not for approval. Body: the entry, each Done when
   clause with the test that proves it, the triage table, techdebt filed, screenshots.
   The commit title is the changelog line.
9. **Merge** when every required check is green: `just merge`. Never before.
10. **Closure** in the same PR: entry marked `merged, unreleased`; techdebt updated;
    `ARCHITECTURE.md` if a module appeared; the spec if a rule changed.
11. **Next** entry, or stop (§ 8).

**Release** — the human runs `just release -y` when they choose. It re-runs the full
harness on `main`, computes the version from the merged titles, writes the changelog,
tags and pushes; CI builds and leaves the draft; the human publishes it. Entries flip
from `merged, unreleased` to `Shipped in vX.Y.Z`.

## 4. Design proposal

Before any change to what the user sees, the agent commits rendered mocks of the
target state, built with the real components and tokens, under
`screenshots/design/NNN-{state}-{light|dark}.png`, plus `screenshots/design/NNN.md`
(five lines: what moves, what is added, what is removed, what stays). The
`/design-proposal` skill drives it: the same preview pipeline as `/visual-proof`,
written to the design folder instead of the proof folder.

The human validates by editing the entry's `Design` line to `validated`, or writes an
open question. Nothing else is needed. Once the work ships, the proposal images are
deleted in the closure commit; the visual proofs are the record.

## 5. The harness

`just harness` locally; the same set as required checks on every pull request:

| Check                    | Where                                 | Gate                                                                                      |
| ------------------------ | ------------------------------------- | ----------------------------------------------------------------------------------------- |
| Lint, format, type-check | `scripts/check.py`, Quality           | any error                                                                                 |
| Architecture rules A1–A8 | `scripts/arch-check.py`, Quality      | any violation; frozen debt may only shrink (`arch-allowlist.json`)                        |
| Unit and integration     | Vitest, cargo test, Quality           | any failure                                                                               |
| Coverage floors          | `scripts/coverage-gate.py`, Quality   | frontend features < 80 %, backend logic < floor (`coverage-gates.json`, ratchets to 90 %) |
| E2E on the real app      | `.github/workflows/e2e.yml`, every PR | any failure; screenshots linked                                                           |
| Commit hygiene           | Quality `pr-checks`                   | title > 72, wrong type, trailer                                                           |
| Security audit           | `security-audit.yml`                  | new advisory                                                                              |

Planned additions (build order in the plan): golden portfolio figures, reviewer
prompts as CI checks, visual regression, the merge guard, the mutation sweep.

## 6. The right way to code

- **Logic in Rust, dumb frontend.** The frontend renders, holds ephemeral UI state,
  and maps error codes to i18n keys. It never decides, validates business rules,
  aggregates or derives. If a step can skip the UI, it lives in Rust.
- **Gold layouts** for new code (`docs/backend-rules.md` B0/B37–B43,
  `docs/frontend-rules.md` F0/F26–F28); bit-by-bit for existing code (CLAUDE.md).
- **Typed errors** on the wire (`docs/error-model.md`); factories and aggregate-root
  methods on domain objects (CLAUDE.md § Critical Patterns).
- **Stable ids** on every interactive element; text from i18n; one event per action on
  the bus.
- **Ubiquitous language** in every identifier (`docs/ubiquitous-language.md`).
- **Surgical**: touch the file set the entry needs. Boyscout inside it, never beyond.

## 7. Reviewers and the triage policy

Reviewers run on the diff before the PR: `reviewer-backend` and `reviewer-arch` for
`.rs`, `reviewer-frontend` and `reviewer-arch` for `.ts`/`.tsx`, `reviewer-sql` for
migrations, `reviewer-infra` for scripts, hooks, config and workflows,
`reviewer-security` for commands, capabilities and secret handling, `reviewer-e2e` for
`e2e/**`, `spec-reviewer` / `contract-reviewer` / `adr-reviewer` when those documents
change, `spec-checker` before closing an entry that carries spec rules.

Every finding is graded and the outcome recorded in the PR body:

| Grade        | Action                                                            |
| ------------ | ----------------------------------------------------------------- |
| (a)          | Fix in the PR                                                     |
| (b)          | `TD-NNN` entry in `docs/techdebt.md`, linked from the PR body     |
| (c) one-off  | Inline comment `// <reviewer> FP: <reason> — see PR #NN`          |
| (c) pattern  | Edit the reviewer prompt in the same PR; note it in the PR body   |
| `[DECISION]` | Open question on the entry; the PR stays open; the agent moves on |

No halt for the human. A review comment from the human on a merged or open PR is
treated as a new open question on the entry.

## 8. Budget and stop rules

- One entry, one run, a wall-clock budget of three hours. Over budget → open question
  "larger than estimated: split?", move on.
- The same gate failing three times on one entry → open question, move on.
- E2E failed once and passed on re-run → the run continues; the flake is filed as
  `TD-NNN` with the failure screenshot.
- Never touch the live portfolio database, never force-push, never bypass a hook,
  never edit a released changelog line, never reorder Next, never cut a release.

## 9. Where the loop runs

The chat session is for writing entries together, design conversations and answering
open questions. The loop runs headless, one entry per run (`/next-todo`), from a
scheduled job; state lives in git and the two files, so there is no session to resume.
Until the scheduled runner exists, a human types `/next-todo` in a session and the
same rules apply.
