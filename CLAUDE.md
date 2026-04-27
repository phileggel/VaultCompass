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

- **At session start**: run `/whats-next` to triage pending work across TODOs, plans, specs, and in-flight git.
- **At task start**: run `/start [scope]` (`fix`, `chore`, `test`, `feature`, `refactor`) to pick the right workflow.
- **Revisiting an existing feature**: run `/spec-diff` first to detect drift between the current spec and the last plan, and to flag stale tasks/tests.

**IMPORTANT**: Claude Code will NOT commit, create branches, or create PRs. The user handles all git operations.

### CRITICAL: Implementation task

- Any code file is considered as implementation task
- ONLY exception is doc files
- Every task should follow _Plan Before Implementation_

### Project-specific workflow additions

On top of the standard kit workflow, this project requires:

1. **Before implementing**: read `docs/backend-rules.md` (backend changes) and/or `docs/frontend-rules.md` (frontend changes).
2. **Plan step**: after proposing the TODO plan, immediately create a TaskList (`TaskCreate`) with one task per remaining step. Ask user to validate before implementing.
3. **Docs update**: at the end, update `ARCHITECTURE.md` if new files/modules added; `docs/todo.md` if new tech debt or resolved items; for new business rules use `/spec-writer` to author/extend the spec in `docs/spec/` and `/contract` to derive the matching `docs/contracts/{domain}-contract.md`. Use `/adr-manager` to record architectural decisions in `docs/adr/`.
4. **Commit**: ask user if a commit is needed → use `/smart-commit` skill.

### Task tracking (within a conversation)

**MANDATORY** for every implementation task — use `TaskCreate` / `TaskUpdate`:

- Create tasks before implementing anything — do NOT create a task for `workflow-validator` (it runs outside the TaskList)
- Mark each task `in_progress` when starting, `completed` when done
- The `workflow-validator` reads this TaskList — missing or incomplete tasks = blocked commit

---

## 🛠 Commands

- Dev: `./scripts/start-app.sh`
- Quality: `just check-full` (full check) | `just check` (fast lint+format only)
- Tests: `just test` (frontend) | `just test-rust` (backend) | `just test-all` (both)
- Types: `just generate-types` (Sync Rust to TS via Specta)
- Database schema update: `just clean-db`
- Pre-release audit: `/dep-audit` (npm + Cargo CVEs and outdated versions)
- Release: `just release [--dry-run] [--version X.Y.Z] [-y]` (run `/dep-audit` first)
- After `just sync-kit` with a non-trivial delta: run `/kit-discover` to reconcile this file with the kit.

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
