# Kit Tools Reference

Thematic index of all agents, skills, scripts, git hooks, and justfile recipes
provided by **tauri-claude-kit**. Use this file to discover what is available
without reading each agent definition individually.

Each item lists its **trigger** (when to invoke it) and a one-line description.

---

## Workflows

### Option A — Full Feature Workflow

_Use for: New features, new business logic, significant UI changes, or complex refactoring._

**Phase 1: Pre-implementation (Spec & Plan)**

1. Run **`/spec-writer`** skill → produces `docs/spec/{feature}.md`.
2. _(Optional)_ Run **`/adr-manager`** skill → produces `docs/adr/{ref}.md` if an architectural decision is needed.
3. Run **`spec-reviewer`** agent to validate spec quality (DDD alignment, rule atomicity, UX completeness).
4. Run **`feature-planner`** agent → produces `docs/plan/{feature}-plan.md` with a task checklist.

**Phase 2: Execution**

1. Read `docs/plan/{feature}-plan.md` — this is your Primary TaskList. Do not deviate from it.
2. Implement the feature layer by layer, updating checkboxes (`[ ]` → `[x]`) in the plan file after each completed task.

**Phase 3: Review & Quality**

1. Run `python3 scripts/check.py` (or `just check-full`) and fix all issues.
2. Write missing tests.
3. Run the reviewer gauntlet:
   - **`reviewer`** agent → fix issues.
   - If `.rs` modified: **`reviewer-backend`** agent → fix issues.
   - If `.ts` / `.tsx` modified: **`reviewer-frontend`** agent → fix issues.
   - If `migrations/` modified: **`reviewer-sql`** agent → fix issues.
   - If `.sh`, `.py`, or `.githooks` modified: **`script-reviewer`** agent.
   - If `capabilities/*.json` or `tauri.conf.json` modified: **`maintainer`** agent.
   - If UI text changed: **`i18n-checker`** agent.

**Phase 4: Validation & Closure**

1. Run **`spec-checker`** agent to confirm all spec rules are covered — tick its checkbox in the plan.
2. Use **`/smart-commit`** skill to commit.
3. Run **`workflow-validator`** agent as the final gate — verifies all plan checkboxes are ticked. Its successful completion is the sign-off; no checkbox needed.

---

### Option B — Simple Technical Workflow

_Use for: Bug fixes, dependency updates, minor maintenance (no new business rules or features)._

1. **Analysis**: Read relevant documentation and analyze the codebase.
2. **Direct Plan**: Propose a concise TODO plan with exact file paths in the chat. Ask user to validate.
3. **Tracking**: Use `TaskCreate` / `TaskUpdate` tools to track workflow steps (`in_progress` when starting, `completed` when done).
4. **Implementation**: Execute the code changes.
5. **Review & Quality**: Run `python3 scripts/check.py` (or `just check-full`), write missing tests, and run relevant subagents (`reviewer`, `script-reviewer`, etc.) as in Phase 3 above.
6. **Closure**: Ask user if another task is needed before commit, otherwise use **`/smart-commit`** skill.

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

| Agent             | Trigger                                   | Description                                                                                                                                    |
| ----------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `spec-reviewer`   | After spec-writer, before feature-planner | Quality gate on a spec doc: rule atomicity, scope, DDD alignment, UX completeness, conflicts                                                   |
| `retro-spec`      | Onboarding an existing feature to the kit | Infers TRIGRAM-NNN rules from existing code and writes a first-pass `docs/spec/{domain}.md` with `retro-inferred` annotations for human review |
| `feature-planner` | After spec-reviewer approves              | Translates spec into `docs/plan/{feature}-plan.md` with DDD layer breakdown, rule-to-task mapping, Workflow TaskList                           |
| `spec-checker`    | After implementation, before commit       | Verifies every TRIGRAM-NNN rule from the spec is implemented in code and covered by tests                                                      |

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

| Skill          | Command         | Description                                                                                                       |
| -------------- | --------------- | ----------------------------------------------------------------------------------------------------------------- |
| `smart-commit` | `/smart-commit` | Conventional commit with sensitive-file check, linter run, suggested title with char count, and user confirmation |
| `dep-audit`    | `/dep-audit`    | Audit npm + Cargo dependencies for outdated versions and CVEs; run before every release                           |
| `adr-manager`  | `/adr-manager`  | Create, update (supersede), or index Architecture Decision Records in `docs/adr/`                                 |
| `spec-writer`  | `/spec-writer`  | Same as the `spec-writer` agent, available as an interactive skill                                                |

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

| Recipe           | Command               | Description                                                                 |
| ---------------- | --------------------- | --------------------------------------------------------------------------- |
| `check`          | `just check`          | Fast quality check — lint + format only                                     |
| `check-full`     | `just check-full`     | Full quality check — tests + build + lint                                   |
| `format`         | `just format`         | Auto-fix formatting: `cargo fmt`, `cargo clippy --fix`, frontend formatters |
| `migrate`        | `just migrate`        | Run pending SQLx database migrations                                        |
| `prepare-sqlx`   | `just prepare-sqlx`   | Regenerate SQLx offline query cache after schema or query changes           |
| `release`        | `just release`        | Interactive release manager                                                 |
| `sync-kit`       | `just sync-kit`       | Sync this kit into the project (latest release tag)                         |
| `clean-db`       | `just clean-db`       | **Destructive** — deletes local database and recreates schema               |
| `clean-branches` | `just clean-branches` | **Destructive** — removes stale remote-tracking branches                    |
| `stat`           | `just stat`           | Line count stats via `cloc`                                                 |
