# Backend Rules

**AI AGENT SHOULD NEVER UPDATE THIS DOCUMENT**

## Folder Structure

**B0** — The backend source tree MUST follow this layout:

```
src-tauri/src/
├── core/             # Shared infrastructure (db, logger, event_bus, specta)
│   ├── db.rs
│   ├── logger.rs
│   ├── specta_types.rs
│   ├── specta_builder.rs
│   └── event_bus/
│       ├── bus.rs
│       ├── event.rs
│       └── mod.rs
├── context/          # DDD bounded contexts — no cross-context imports
│   └── {domain}/
│       ├── domain/       # Entities + repository traits
│       ├── repository/   # SQLite implementations of repository traits
│       ├── service.rs    # Domain service — optional, only if it adds domain value beyond CRUD
│       ├── api.rs        # Tauri command handlers
│       └── mod.rs        # Public re-exports only
├── use_cases/        # Cross-context orchestrators (if needed)
└── lib.rs            # App wiring: state construction + Tauri setup
```

**B0a** — `core/` MUST only contain infrastructure utilities with no domain knowledge.

**B0b** — `context/{domain}/repository/` MUST only contain SQLite implementations of traits declared in `context/{domain}/domain/`. No business logic.

**B0c** — `core/specta_builder.rs` is the ONLY place where Tauri commands are registered.

## Domain Object

**B1** — MUST be created with a factory method:

- `new()` — validates fields and generates id (use in service)
- `with_id()` — validates fields, uses provided id (use in api/service)
- `restore()` — direct restore from database, no validation (use in repository only)

## Bounded Context (`/context`)

**B2** — MUST never import from another context.

**B3** — MUST share its external API directly through its main `mod.rs`.

- Outside the context, never import `crate::context::patient::domain::::MyDomainObject` — always import `crate::context::patient::MyDomainObject`.

**B4** — SHOULD always publish a `{Domain}Updated` event when its state changes (create, update, delete, etc.).

**B5** — MUST declare its Tauri commands in the `api.rs` file.

## Use Cases (`/use_cases`)

**B6** — MAY import from contexts, MUST NOT import from another use case.

**B7** — MUST share its external API directly through its main `mod.rs`.

**B8** — MUST NOT publish a `{Domain}Updated` event (orchestrators do not own state).

**B9** — MUST declare its Tauri commands in the `api.rs` file.

**B10** — SHOULD have an orchestrator as its main entry point (after api) that handles the global logic.

## Repository

**B11** — MUST use sqlx macros for queries. Use `just clean-db` to reset the database if needed.

## Logging

**B12** — MUST use `tracing::{info, debug, warn, error}` with structured fields. Never use `println!`.

**B17** - MUST use `target:` field when adding a new backend specific log.

**B16** — When using the `target:` field in tracing calls, MUST use the `BACKEND` or `FRONTEND` constant from `crate::core::logger` instead of string literals:

```rust
use crate::core::logger::BACKEND;
tracing::info!(target: BACKEND, field = value, "message");
```

## UseCase - Service

**B18** — Use cases MAY depend on any domain abstraction: repository traits, domain entities,
or bounded context services. They MUST NOT depend on infrastructure: concrete repository
implementations, `sqlx::Pool`, `sqlx::Transaction`, `sqlx::query!`, or any other sqlx type.

**B19** — A bounded context service (`service.rs`) is a Domain Service. It MUST only exist if
it encapsulates non-trivial domain logic: cross-entity invariants, event emission, or
coordination that does not belong to a single entity. Trivial CRUD with no added logic does
not justify a service — the use case should depend on the repository trait directly.
A service MUST NOT expose repository types or sqlx types in its public signature.

**B20** - If atomic transaction is needed, orchestrator or service MUST use UnitOfWork pattern.

## General

**B13** — MUST use `anyhow::Result<T>` for error handling.

- Exception: Tauri command responses use `Result<T, String>`.

**B14** — MAY use `#[allow(clippy::too_many_arguments)]` on domain factory methods.

## Tests

**B15** — Tests MUST NOT be trivial. A trivial test is one that verifies:

- A constructor does not panic
- An empty input returns empty output (no logic traversed)
- A getter returns what was just passed in
- A test helper disguised as a test

**B21** — Unit tests (mod tests inside src/) MUST mock repository dependencies using mockall-generated mocks. 

**B22** — Integration tests (tests/ folder) MUST use real SQLite repos. They test cross-layer behavior end-to-end and MUST NOT use mocks.