# Kit Reference Guide

Onboarding guide for tauri-claude-kit. For the full inventory of agents, skills, scripts, hooks, and justfile recipes, see `.claude/kit-tools.md`.

**Location**: `.claude/kit-readme.md` (read-only reference)

---

## Standard Workflows

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

## Spec Rule Numbering System (TRIGRAM-NNN)

Specs use **TRIGRAM-NNN** format for business rules (e.g., `REF-010`, `PAY-020`, `INV-030`):

- **TRIGRAM** = 3-letter identifier unique per feature domain (e.g., REF for Refund, PAY for Payment)
- **NNN** = 3-digit number, organized by topic:
  - 010–019: Eligibility & initiation
  - 020–029: Creation
  - 030–039: Updates & status changes
  - 040–049: Deletion
  - 050–059: Extensions & future

**Immutability**: Once assigned, a rule number never changes. If a rule is removed, its number stays vacant.

**Registry**: All trigrams are registered in `docs/spec-index.md` (created by `/spec-writer`).

---

## Handling `[DECISION]` Criticals

Some reviewer criticals are tagged `[DECISION]`. These indicate that the correct fix requires an architectural choice — not a mechanical code change — and cannot be resolved without domain or team input.

**Recommended rule for your project's `CLAUDE.md`:**

> **Reviewer `[DECISION]` criticals must not be fixed unilaterally.** When a reviewer flags a Critical with `[DECISION]`, stop and present the finding to the user before writing any code. The reviewer's guidance describes the direction, not the final answer — the architectural boundary must be agreed upon first.

**Why this matters:** a cross-boundary import can be "fixed" in several structurally valid ways (new use-case, shared port, merged context). Choosing the wrong one silently encodes an architectural assumption that is hard to undo. The `[DECISION]` tag is the reviewer's signal that human judgment is required.

---

## Customizing / Extending Agents

Your project can:

- ✅ Add **local agents** in `.claude/agents/{your-agent}.md` (project-specific reviewers, validators, etc.)
- ✅ Add **local skills** in `.claude/skills/{your-skill}/SKILL.md`
- ❌ Do NOT modify kit agents/skills directly — they are overwritten on the next sync

If you need to extend a kit agent's behaviour:

1. Create a new local agent that invokes or wraps the kit agent's logic.
2. Add project-context or domain-specific validation.
3. Document it in your project's local `.claude/` directory.

---

## Before Major Project Releases

- Run agents on sample specs/code to validate output.
- Execute scripts to confirm they work in your environment.
- Check that `docs/spec-index.md` is up-to-date.
- Run `just check-full` (or `python3 scripts/check.py`) — all checks must pass.

---

## Troubleshooting

**Agent not found?**

- Check if the agent file exists: `ls -la .claude/agents/`
- Re-sync the kit: `./scripts/sync-config.sh`

**Agent gives wrong output?**

- Verify your spec/code is at the expected path (e.g., `docs/spec/{feature}.md`, not `docs/{feature}.md`).
- Check that required files exist (e.g., `ARCHITECTURE.md`, `docs/adr/`).

**Trigram collision?**

- Check `docs/spec-index.md` for registered trigrams.
- Use a different 3-letter prefix for the new feature spec.
