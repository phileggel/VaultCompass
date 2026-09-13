# ADR 020 — Drop the claude-kit and own the tooling

**Date**: 2026-09-13
**Status**: Accepted

## Context

For a year the project's agents, skills, scripts, hooks, convention docs and shared
`just` recipes were synced from an external repository, `claude-kit`, through
`scripts/sync-config.sh` and a manifest. The sync brought a constraint with it: every
synced file was read-only for project-specific content, because the next sync would
overwrite it. Convention docs could not carry project addenda, reviewer prompts could
not learn from triage outcomes, scripts could not gain project options, and CLAUDE.md
had to route around the kit's workflow definitions instead of stating the project's own.

Workflow C (`docs/workflow-c.md`) needs exactly the opposite: reviewer prompts edited
in the same PR that rejects a pattern-level false positive, a capture script with a
project output option, convention docs that state the project's rules, and one
justfile. Phases 1 to 5 of its build already edited synced scripts, skills and docs.
The kit's value was the starting inventory; its cost had become the rules it imposed
on files the project must now shape freely.

## Decision

The kit is dropped, not frozen. No sync ever again:

- The sync machinery is removed: `scripts/sync-config.sh`, `scripts/validate-sync.sh`,
  the `sync-kit` recipe, `.claude/kit-manifest.txt`, `.claude/kit-version.md`,
  `.claude/kit.config.json`, `.claude/kit-readme.md`, `.claude/kit-tools.md`, and the
  `/kit-discover` skill.
- `common.just` is folded into `justfile`.
- Every file the kit shipped — agents, skills, scripts, hooks, convention docs — is a
  project file, edited whenever a rule changes, in the same PR as the change. Rule
  numbers (B, F, E) stay stable and keep being cited; new rules are appended here.
- The skills Workflow C made dead are removed with the sync: `/start`, `/smart-commit`,
  `/create-pr`. `/whats-next` stays as the human's backlog overview; `/review-triage`
  stays as the grading axes; `/techdebt` stays as the entry format.

Alternatives considered:

- **Freeze**: keep the manifest and version file, never sync again. Rejected: a frozen
  kit still reads as an external authority ("do not edit, it is synced"), and a
  manifest with nothing to check is dead code.
- **Per-file opt-out**: keep syncing, mark the files the project edits as local.
  Rejected: phases 1 to 5 touched scripts, skills, docs and hooks alike; the opt-out
  list would grow to the whole manifest.
- **Sync scripts and hooks only**: rejected for the same reason; the capture script
  and the hooks are among the first files Workflow C needed to change.

## Consequences

**Pros**:

- Convention docs, reviewer prompts, scripts and recipes change in the PR that needs
  them to, with the harness as the only gate.
- One `justfile`, one place to read the rules (`CLAUDE.md` and `docs/workflow-c.md`),
  no reconciliation step after a sync.
- Reviewer prompts learn from triage: a pattern-level false positive is fixed at its
  source instead of being re-graded on every PR.

**Cons**:

- Upstream improvements to the kit no longer arrive. Anything worth having is ported
  by hand, as a normal change through the harness.
- Reviewer prompts can drift with the project. Every edit to them is a visible diff in
  a PR body's triage table; the monthly mutation sweep (plan phase 12) is the
  independent check that the review lane has not been softened.
- The lesson that was about the sync tool itself, L-004 in `docs/lessons.md`, is
  removed in the same change that adopts this decision; its number stays vacant, as
  the lessons file's numbering rule requires.
