# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> Full architecture reference: [ARCHITECTURE.md](ARCHITECTURE.md)

## ⚠️ Workflow & Planning
**IMPORTANT**: Claude Code will NOT commit, create branches, or create PRs. The user handles all git operations.

### CRITICAL: Implementation task
- Any code file is considered as implementation task
- ONLY exception is doc files
- Every task should follow *Plan Before Implementation*

### Workflow

1. **(Optional)** Spec — run `/spec-writer` → `spec-reviewer` → `feature-planner` for features with unclear requirements.
2. Read docs — backend: `docs/backend-rules.md` / frontend: `docs/frontend-rules.md`.
3. Analyze the request and current codebase.
4. **Propose a TODO plan** — CRITICAL: immediately create a TaskList (`TaskCreate`) with one task per remaining step (5–17). Ask user to validate; loop back to step 3 if changes needed.
5. **(Optional)** Stitch mockup — for significant new/redesigned UI (see 🎨 Stitch Workflow section).
6. Implementation.
7. `./scripts/check.sh` — all checks must pass.
8. `reviewer` → show full report → fix criticals → re-run until 0 critical.
9. `ux-reviewer` — if any `.tsx` modified → show full report → fix criticals → re-run until 0 critical.
10. `script-reviewer` — if any `.sh`, `.py`, or `.githooks` modified.
11. `maintainer` — if any CI/config file modified (`.github/workflows/`, `tauri.conf.json`, `Cargo.toml`, `package.json`, `justfile`).
12. `i18n-checker` — if any UI text was added or changed.
13. Tests — if non-trivial logic added (backend: `#[cfg(test)]` inline; frontend: `.test.ts` colocated).
14. `dep-audit` — if preparing a release (CVEs are a release blocker).
15. Update docs — `ARCHITECTURE.md` if new files/modules added; `docs/todo.md` if new tech debt or resolved items; spec in `docs/` if new business rules; then run `spec-checker` if a spec exists.
16. **CRITICAL: run `workflow-validator`** — blocks commit if any required step is incomplete.
17. Ask user if a commit is needed → follow `/commit` skill.

### Task tracking (within a conversation)
**MANDATORY** for every implementation task — use `TaskCreate` / `TaskUpdate`:
- At step 4: create one task per applicable step (steps 5–15 and 17, plus conditional steps if triggered) before implementing anything — do NOT create a task for step 16 (workflow-validator runs outside the TaskList)
- Mark each task `in_progress` when starting, `completed` when done
- The `workflow-validator` (step 16) reads this TaskList — missing or incomplete tasks = blocked commit

### Available Subagents (`.claude/agents/`)

**Pre-implementation (spec & planning)**
- `spec-reviewer` — reviews a draft spec doc for quality before implementation: rule atomicity, scope coverage, DDD alignment, UX completeness, conflicts; use between spec-writer and feature-planner
- `feature-planner` — reads a validated spec doc + architecture, produces an exact TODO plan (file paths, function names, layers); use at step 1 for complex features

**Post-implementation (review & quality)**
- `reviewer` — DDD + backend/frontend rules compliance check (step 8)
- `ux-reviewer` — M3 + Clinical Atelier compliance, empty/loading/error states, form UX, accessibility, consistency (step 9, if .tsx modified)
- `script-reviewer` — Bash and Python expert reviewer; checks safety, robustness, portability (step 10, if .sh/.py/.githooks modified)
- `maintainer` — reviews CI/config files (`workflows/`, `tauri.conf.json`, `Cargo.toml`, `package.json`, `justfile`) for correctness, security, reliability (step 11, if config modified)
- `i18n-checker` — finds hardcoded strings, missing/dead translation keys fr + en (step 12, if UI text changed)
- `spec-checker` — verifies all Rn rules in a feature spec are implemented and tested (step 15, if spec exists)
- `workflow-validator` — **mandatory at step 16**: reads TaskList + git diff, reports ✅/❌ for each workflow step, blocks commit if incomplete

**Meta**
- `ia-reviewer` — meta-reviewer for AI configuration: audits all agent definitions, skills, and CLAUDE.md for correctness, clarity, completeness, and internal consistency

### Subagent workflow map

```
1.  spec-writer → spec-reviewer → feature-planner  [optional]
          ↓
2–4.  Read docs + Analyze + Plan + TaskCreate (mandatory)
          ↓
5.    Stitch mockup  [optional]
          ↓
6.    Implementation
          ↓
7.    check.sh
          ↓
8.    reviewer
9.    ux-reviewer        — if any .tsx modified
10.   script-reviewer    — if any .sh/.py/.githooks modified
11.   maintainer         — if any CI/config file modified
12.   i18n-checker       — if UI text changed
          ↓
13.   Tests              — if non-trivial logic
14.   dep-audit          — if release
          ↓
15.   Update docs (ARCHITECTURE.md + todo.md + spec-checker)
          ↓
16.   workflow-validator  ← MANDATORY, blocks commit
          ↓
17.   /commit (skill)
```

### Available Skills (`.claude/skills/`)
- `/spec-writer` — interactive spec writer: interviews the user, reads the domain, produces `docs/{feature}.md` with Rn rules + UX draft; optional Stitch mockup generation (step 1)
- `/commit` — smart-commit: conventional commit avec validation tests + linters + confirmation (step 17)
- `/dep-audit` — dependency audit: checks npm + Cargo for outdated packages and CVEs using live web search; run before every release or after dependency changes (step 14)

---

## 🎨 Stitch Workflow

### When to use Stitch
Use Stitch when the task involves a **significant new or redesigned UI component** (new page, new modal, major UX change). Not for small fixes or backend-only work. Insert as optional **step 5**, between plan validation and implementation.

### Process
```
Step 3b-1: Claude generates initial mockup
           → mcp__stitch__generate_screen_from_text (project: ProjectSF / 7705025027636758446)
           → device: DESKTOP, model: GEMINI_3_1_PRO
           Optional: generate variants for design exploration
           → mcp__stitch__generate_variants (2-5 variants, EXPLORE range)
           → present variants to user, user picks one
Step 3b-2: User refines the chosen design on stitch.withgoogle.com
           Minor corrections can be done by Claude via mcp__stitch__edit_screens
           (e.g. "move the search field below the section label")
Step 3b-3: Claude downloads the result
           → mcp__stitch__list_screens → mcp__stitch__get_screen
           → curl HTML to docs/stitch/{feature}.stitch  (.stitch = no linting, gitignored, ephemeral)
Step 3b-4: Claude reads the HTML and extracts structure as implementation reference
```

### Adapting Stitch output to the codebase
- **Layout/structure** → reimplement in TSX using `ui/components` — never copy-paste Stitch HTML
- **Colors** → map Stitch tokens to our M3 tokens (same semantic names, our values in `tailwind.css`)
- **Fonts/shadows/glassmorphism** → only use if already adopted in our design system (see design system alignment)
- **Stitch HTML is reference only** — it shows intent, not implementation

### UX changes made during Stitch edition
After downloading, Claude identifies UX elements added/changed by the user in Stitch (e.g. new button, new section). These become **complementary todos** — not blocking the current implementation. Implementation follows two phases:
1. **UI structure** — match the Stitch screen layout and visual design
2. **UX wiring** — implement the behavior behind new UI elements (separate task)

### .stitch file lifecycle
- Created at step 3b-3
- Used during implementation (step 5) as visual reference
- **Delete when `ux-reviewer` passes** on the implemented component — the reference is done

### Design system alignment
When Stitch introduces new design patterns (new tokens, shadows, component styles), create a **dedicated todo** for design system alignment — never block feature implementation on it. After alignment, update the `ux-reviewer` agent rules to enforce the new patterns. Stitch project design system and our `tailwind.css` stay naturally in sync once T20 is done.

### Design system reference
- Stitch project: `projects/7705025027636758446` — use this single project for all features, never create a new one
- Design system spec: `docs/stitch/design-system.md` ("The Clinical Atelier") — committed to git
- Target alignment: indigo/purple M3 palette, Manrope (headlines) + Inter (body), primary-tinted ambient shadows, no structural borders (tonal surfaces instead), gradient primary CTAs, glassmorphism modals

## 🛠 Commands
- Dev: `./scripts/start-app.sh`
- Quality: `./scripts/check.sh` (Full check)
- Tests: `npm run test` (Frontend) | `cd src-tauri && cargo test` (Backend)
- Types: `just generate-types` (Sync Rust to TS via Specta)
- Database schema update: `just clean-db`
- Release: `python3 scripts/release.py [--dry-run] [--version X.Y.Z] [-y]`

## 🏗 Architecture Summary
Tauri 2 app (React 19 + Rust) using Domain-Driven Design.

**Backend (`src-tauri/src/`)**:
- `core/specta_builder.rs` — Tauri command registry (DO NOT add commands elsewhere)
- `context/{domain}/` — Bounded contexts (self-contained, no cross-context imports):
  - `account/`, `asset/`
  - Each has: `domain/`, `repository/`, `service.rs`, `api.rs`, `mod.rs`
- `use_cases/` — Cross-cutting application use cases (placeholder)

**Frontend (`src/`)**:
- `bindings.ts` — Auto-generated from Rust via Specta (DO NOT EDIT)
- `features/{domain}/` — Feature modules (gold layout: `assets/`):
  - `gateway.ts` at root — only file allowed to call `commands.*`
  - Sub-feature subdirectories with colocated component + hook + test
  - `shared/presenter.ts` — domain → UI transformations; `shared/validate*.ts` — validation

**Data Flow**: Component → Hook → Gateway → Tauri Command → Rust Service → Repository

## 📏 Standards
- **Commits**: Conventional commits (`feat:`, `fix:`, etc.).
- **Style**: React functional components, Rust traits for repositories.
- **Lints**: Oxlint & Biome (FE), Clippy (BE). All must pass.

## ⚠️ Critical Patterns

### Tauri Service Layer - Gateway Pattern
All Tauri invocations in services MUST match `bindings.ts` signatures EXACTLY:
- ✅ `commands.addAsset(name, assetClass, categoryId, currency, riskLevel, reference)` - positional parameters
- ❌ `commands.addAsset({ name, assetClass, categoryId, currency, riskLevel, reference })` - object wrap (WRONG)
- **Rule**: Match parameter COUNT, ORDER, and NAMES from bindings.ts
- When binding has 5 params: call with 5 args in correct order, never wrapped

### Domain Entities - Factory Methods
All domain objects use factory methods (NEVER direct struct literals):
- `new()` - Create new entity: generates ID + validates
- `update_from()` - Update existing entity: uses provided ID + validates
- `from_storage()` - Reconstruct from database: no validation (already validated at storage)
- Repository ONLY uses these factory methods, never direct literals

---

## 📋 Plan Format Guidelines
When proposing a TODO plan, Claude Code MUST:
- List exact file paths, not abstract locations
- Name the specific functions/methods/components to create or modify
- Separate clearly by architectural layer (backend / frontend)
- Include validation and testing steps
- Wait for explicit user approval before implementing
