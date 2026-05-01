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
  - When `/whats-next` identifies a ⚠️ likely-done item, immediately clean up the source doc (remove/cross off the entry in `docs/todo.md`, close open questions in specs, update the plan file, etc.) — do not just list it as a cleanup candidate.
- **After completing any action**: immediately update the source doc that tracked it — remove or tick off the entry in `docs/todo.md`, close the open question in the spec, tick the plan step, etc. Do not wait for the next `/whats-next` run.
- **At task start**: run `/start [scope]` (`fix`, `chore`, `test`, `feature`, `refactor`) to pick the right workflow.

**IMPORTANT**: Claude Code will NOT commit, create branches, or create PRs autonomously. Use `/create-pr` to push the current branch and open a GitHub PR (requires `gh` CLI). The user handles all git operations.

### CRITICAL: Implementation task

- Any code file is considered as implementation task
- ONLY exception is doc files
- Every task should follow _Plan Before Implementation_

### Project-specific workflow additions

On top of the standard kit workflow, this project requires:

1. **Before implementing**: read `docs/backend-rules.md` (backend changes), `docs/frontend-rules.md` (frontend changes), and/or `docs/e2e-rules.md` (E2E test changes).
2. **Plan step**: after proposing the TODO plan, immediately create a TaskList (`TaskCreate`) with one task per remaining step. Ask user to validate before implementing.
3. **Docs update**: at the end, update `ARCHITECTURE.md` if new files/modules added; `docs/todo.md` if new tech debt or resolved items; for new business rules use `/spec-writer` to author/extend the spec in `docs/spec/` and `/contract` to derive the matching `docs/contracts/{domain}-contract.md`. Use `/adr-manager` to record architectural decisions in `docs/adr/`.
4. **E2E tests** (after frontend impl, before release): run `test-writer-e2e` agent with the domain contract to write failing WebDriver tests against the live app. Run `/setup-e2e` once first if not yet initialized.
5. **Commit**: ask user if a commit is needed → use `/smart-commit` skill.

### Task tracking (within a conversation)

**MANDATORY** for every implementation task — use `TaskCreate` / `TaskUpdate`:

- Create tasks before implementing anything
- Mark each task `in_progress` when starting, `completed` when done

---

## 🛠 Commands

- Dev: `./scripts/start-app.sh`
- Quality: `just check-full` (full check) | `just check` (fast lint+format only)
- Tests: `just test` (frontend) | `just test-rust` (backend) | `just test-all` (both)
- Types: `just generate-types` (Sync Rust to TS via Specta) | `just prepare-sqlx` (after schema/query changes)
- Database: `just migrate` (run migrations) | `just clean-db` (⚠️ destructive reset)
- E2E setup (once): `/setup-e2e` (installs WebDriver deps + generates `wdio.conf.ts`)
- E2E tests: `npm run test:e2e` (local) | `npm run test:e2e:xvfb` (Linux headless)
- Pre-release audit: `/dep-audit` (npm + Cargo CVEs and outdated versions)
- Code audit: `/prune` (dead code, verbose patterns, KISS review)
- Release: `just release [--dry-run] [--version X.Y.Z] [-y]` (run `/dep-audit` first)
- PR: `/create-pr` (push branch + open GitHub PR; drafts title + body from commits and plan doc; requires `gh` CLI)
- Merge branch: `just merge` (fast-forward current branch into main, then delete it)
- After `just sync-kit` with a non-trivial delta: run `/kit-discover` to reconcile this file with the kit.

## 📖 Ubiquitous Language

`docs/ubiquitous-language.md` is the authoritative dictionary of domain terms.

- New code MUST use confirmed UL terms in identifiers, comments, and log messages.
- Do not extend usage of a discrepant term — fix it or flag it before adding more callsites.
- When spawning reviewer, spec-writer, or feature-planner agents, include the UL doc in the prompt so they can check term consistency.

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
