# Kit Tools Reference

Thematic index of all agents, skills, scripts, git hooks, and justfile recipes
provided by **tauri-claude-kit**. Use this file to discover what is available
without reading each agent definition individually.

Each item lists its **trigger** (when to invoke it) and a one-line description.

---

## Code Review Agents

| Agent               | Trigger                                  | Description                                                                                                                                                                                           |
| ------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `reviewer`          | Any `.rs`, `.ts`, or `.tsx` modified     | DDD architecture: bounded context isolation, gateway pattern, factory methods, data flow direction, dead code, English-only                                                                           |
| `reviewer-backend`  | Any `.rs` modified                       | Rust quality: anyhow error handling, no `unwrap()` in production, Clippy, trait-based repositories, async correctness, inline tests                                                                   |
| `reviewer-frontend` | Any `.ts` / `.tsx` modified              | React/TS quality + UX/M3: gateway encapsulation, hook colocation, presenter layer, `useCallback`/`useMemo` correctness, M3 design tokens, UX completeness (empty/loading/error states), accessibility |
| `reviewer-sql`      | Any `migrations/` file modified or added | SQL migrations: atomicity, idempotency, destructive DDL guards, FK indexes, SQLite type affinity, primary key convention, NOT NULL                                                                    |

---

## Spec & Planning Agents

| Agent                  | Trigger                                      | Description                                                                                                                                    |
| ---------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `spec-reviewer`        | After spec-writer, before /contract          | Quality gate on a spec doc: rule atomicity, scope, DDD alignment, UX completeness, contractability, conflicts                                  |
| `contract-reviewer`    | After /contract, before feature-planner      | Quality gate on a domain contract: coverage vs spec, traceability, error exhaustiveness, type correctness                                      |
| `retro-spec`           | Onboarding an existing feature to the kit    | Infers TRIGRAM-NNN rules from existing code and writes a first-pass `docs/spec/{domain}.md` with `retro-inferred` annotations for human review |
| `feature-planner`      | After contract-reviewer approves             | Translates spec into `docs/plan/{feature}-plan.md` with DDD layer breakdown, rule-to-task mapping, Workflow TaskList                           |
| `test-writer-backend`  | After contract-reviewer, before backend impl | Writes all failing Rust test stubs from the domain contract; confirms red via cargo test                                                       |
| `test-writer-frontend` | After backend commit, before frontend impl   | Writes all failing Vitest stubs from the domain contract; reads fresh bindings.ts; confirms red via vitest                                     |
| `spec-checker`         | After implementation, before final commit    | Verifies every TRIGRAM-NNN rule is implemented and tested; checks all contract commands are covered in backend, frontend, and tests            |

> **Resuming after interruption or compaction:** The plan is always saved to `docs/plan/{feature}-plan.md`.
> After any interruption, ground the agent explicitly:
>
> ```
> Read docs/plan/{feature}-plan.md, then execute step 4.2 only. Stop after.
> ```
>
> Never say "continue" alone — the agent will re-plan from scratch instead of resuming.

---

## Quality & Process Agents

| Agent                | Trigger                                                                   | Description                                                                                                      |
| -------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `i18n-checker`       | Any `.ts` / `.tsx` or translation JSON modified                           | Hardcoded strings, missing/unused translation keys, cross-locale mismatches                                      |
| `workflow-validator` | Before committing a feature implementation                                | Reads `docs/plan/*-plan.md` Workflow TaskList; produces ✅/❌ table; blocks commit if required steps are missing |
| `script-reviewer`    | Any `.sh`, `.py` (in `scripts/`) or `.githooks/` file modified            | Script quality: `set -euo pipefail`, shebang, quoting, portability, security                                     |
| `maintainer`         | Any workflow, config, or `capabilities/*.json` modified; before a release | CI/config/capability correctness, security, consistency; delegates dependency audit to `/dep-audit`              |

---

## Skills (slash commands)

| Skill          | Command          | Description                                                                                                                                              |
| -------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `start`        | `/start [scope]` | Select workflow A (full) or B (simple) for the current task; outputs actionable checklist. Optional scope: `fix`, `chore`, `test`, `feature`, `refactor` |
| `smart-commit` | `/smart-commit`  | Conventional commit with sensitive-file check, linter run, suggested title with char count, and user confirmation                                        |
| `dep-audit`    | `/dep-audit`     | Audit npm + Cargo dependencies for outdated versions and CVEs; run before every release                                                                  |
| `adr-manager`  | `/adr-manager`   | Create, update (supersede), or index Architecture Decision Records in `docs/adr/`                                                                        |
| `spec-writer`  | `/spec-writer`   | Interactive spec writer: interviews user, reads domain, produces `docs/spec/{feature}.md` with TRIGRAM-NNN rules                                         |
| `contract`     | `/contract`      | Derives or updates `docs/contracts/{domain}-contract.md` from a validated spec; upsert-aware, human-approved                                             |

---

## Git Hooks (`.githooks/`)

| Hook         | Runs on      | Behaviour                                                                                           |
| ------------ | ------------ | --------------------------------------------------------------------------------------------------- |
| `pre-commit` | `git commit` | Runs `python3 scripts/check.py --fast` (lint + format); rejects commit on failure                   |
| `commit-msg` | `git commit` | Enforces conventional format (`type: description`), valid types, ≤72-char title, no co-author lines |
| `pre-push`   | `git push`   | Runs `python3 scripts/check.py` (full suite: tests + build + lint); blocks push on failure          |

Activate with: `git config core.hooksPath .githooks`

---

## Scripts

| Script            | Command                           | Description                                                     |
| ----------------- | --------------------------------- | --------------------------------------------------------------- |
| `check.py`        | `python3 scripts/check.py`        | Full quality check: lint, format, tests, build                  |
| `check.py --fast` | `python3 scripts/check.py --fast` | Fast check: lint + format only (used by pre-commit hook)        |
| `release.py`      | `python3 scripts/release.py`      | Interactive release manager: bumps version, tags, and publishes |

---

## Justfile Recipes (`common.just`)

| Recipe           | Command               | Description                                                                                          |
| ---------------- | --------------------- | ---------------------------------------------------------------------------------------------------- |
| `check`          | `just check`          | Fast quality check — lint + format only                                                              |
| `check-full`     | `just check-full`     | Full quality check — tests + build + lint                                                            |
| `format`         | `just format`         | Auto-fix formatting: `cargo fmt`, `cargo clippy --fix`, frontend formatters                          |
| `migrate`        | `just migrate`        | Run pending SQLx database migrations                                                                 |
| `generate-types` | `just generate-types` | Regenerate Specta TypeScript bindings after adding or changing Tauri commands (project-configurable) |
| `prepare-sqlx`   | `just prepare-sqlx`   | Regenerate SQLx offline query cache after schema or query changes                                    |
| `release`        | `just release`        | Interactive release manager                                                                          |
| `sync-kit`       | `just sync-kit`       | Sync this kit into the project (latest release tag)                                                  |
| `clean-db`       | `just clean-db`       | **Destructive** — deletes local database and recreates schema                                        |
| `clean-branches` | `just clean-branches` | **Destructive** — removes stale remote-tracking branches                                             |
| `stat`           | `just stat`           | Line count stats via `cloc`                                                                          |
