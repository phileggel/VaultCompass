# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> Full architecture reference: [ARCHITECTURE.md](ARCHITECTURE.md)

## 🧭 Behavioral Principles

Before coding:

- State assumptions explicitly. If multiple interpretations exist, present them — don't pick silently.
- If something is unclear, stop. Name what's confusing. Ask.

While coding:

- Every changed line must trace directly to the user's request.
- If you notice unrelated dead code, mention it — don't delete it.
- If 200 lines could be 50, stop and rewrite. Ask: "Would a senior engineer say this is overcomplicated?"

## ⚠️ Workflow & Planning

See `.claude/kit-readme.md` for the full workflow guide and `.claude/kit-tools.md` for the agent/skill reference.

**IMPORTANT**: Claude Code will NOT commit, create branches, or create PRs. The user handles all git operations.

### CRITICAL: Implementation task

- Any code file is considered as implementation task
- ONLY exception is doc files
- Every task should follow _Plan Before Implementation_

### Project-specific workflow additions

On top of the standard kit workflow, this project requires:

1. **Before implementing**: read `docs/backend-rules.md` (backend changes) and/or `docs/frontend-rules.md` (frontend changes).
2. **Plan step**: after proposing the TODO plan, immediately create a TaskList (`TaskCreate`) with one task per remaining step. Ask user to validate before implementing.
3. **Optional UI step**: Stitch mockup for significant new/redesigned UI — see 🎨 Stitch Workflow section below.
4. **Docs update**: at the end, update `ARCHITECTURE.md` if new files/modules added; `docs/todo.md` if new tech debt or resolved items; spec in `docs/` if new business rules.
5. **Commit**: ask user if a commit is needed → use `/smart-commit` skill.

### Task tracking (within a conversation)

**MANDATORY** for every implementation task — use `TaskCreate` / `TaskUpdate`:

- Create tasks before implementing anything — do NOT create a task for `workflow-validator` (it runs outside the TaskList)
- Mark each task `in_progress` when starting, `completed` when done
- The `workflow-validator` reads this TaskList — missing or incomplete tasks = blocked commit

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
- **Delete when `reviewer-frontend` passes** on the implemented component — the reference is done

### Design system alignment

When Stitch introduces new design patterns (new tokens, shadows, component styles), create a **dedicated todo** for design system alignment — never block feature implementation on it. After alignment, update the `reviewer-frontend` agent rules to enforce the new patterns. Stitch project design system and our `tailwind.css` stay naturally in sync once T20 is done.

### Design system reference

- Stitch project: `projects/7705025027636758446` — use this single project for all features, never create a new one
- Design system spec: `docs/stitch/design-system.md` ("The Clinical Atelier") — committed to git
- Target alignment: indigo/purple M3 palette, Manrope (headlines) + Inter (body), primary-tinted ambient shadows, no structural borders (tonal surfaces instead), gradient primary CTAs, glassmorphism modals

## 🛠 Commands

- Dev: `./scripts/start-app.sh`
- Quality: `python3 scripts/check.py` (Full check)
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
