# Backend Rules

> For DDD concept definitions, see [docs/ddd-reference.md](ddd-reference.md).

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
│   ├── uow.rs            # TransactionManager trait + SqlxTransactionManager
│   └── event_bus/
│       ├── bus.rs
│       ├── event.rs
│       └── mod.rs
├── context/          # DDD bounded contexts — no cross-context imports
│   └── {domain}/
│       ├── domain/       # Entities + repository traits
│       ├── repository/   # SQLite implementations of repository traits
│       ├── service.rs    # BC-scoped Application Service — optional, only if it adds value beyond trivial CRUD
│       ├── api.rs        # Tauri command handlers (thin adapter only — no business logic)
│       └── mod.rs        # Public re-exports only
├── use_cases/        # Cross-context orchestrators (if needed)
└── lib.rs            # App wiring: state construction + Tauri setup
```

**B0a** — `core/` MUST only contain infrastructure utilities with no domain knowledge.

**B0b** — `context/{domain}/repository/` MUST only contain SQLite implementations of traits declared in `context/{domain}/domain/`. No business logic.

**B0c** — `core/specta_builder.rs` is the ONLY place where Tauri commands are registered.

## Domain Object

**B1** — Domain objects MUST be created with a factory method:

- `new()` — validates fields and generates id (use in service or use case)
- `with_id()` — validates fields, uses provided id (use in service, use case, or api)
- `restore()` — direct restore from database, no validation (use in repository only)

Exception: internal aggregate entities (e.g. `Holding`, `Transaction` within `Account`) have
factory methods that are called ONLY from within the Aggregate Root's methods — never from
services, use cases, or api.rs directly.

Immutable domain concepts with no identity SHOULD be modelled as Value Objects (no ID, no factory method — constructed directly).

## Ubiquitous Language

**B29** — Domain vocabulary (entity names, aggregate method names, event names, domain concepts)
MUST be defined and validated by the user before use in code, tests, or documentation.
The agent MUST NOT unilaterally decide on domain terms — it MUST propose and wait for
explicit confirmation. All confirmed terms MUST be recorded in `docs/ubiquitous-language.md`
and used consistently everywhere.

## Aggregate

**B2** — The BC's root entity (named after the BC folder, e.g. `Account` in `context/account/`) is the Aggregate Root. External code MUST NOT mutate internal entities directly. Reading internal entities for query purposes is acceptable (CQRS-lite).

**B3** — All mutations to internal entities (e.g. `Holding`, `Transaction` within `Account`) MUST go through the Aggregate Root methods or its BC Application Service. No external code constructs or mutates internal entities directly.

**B4** — One database transaction SHOULD modify at most one aggregate. Cross-aggregate writes require the UnitOfWork pattern (B22).

**B28** — Aggregate Root methods MUST use domain/business vocabulary — they describe what
happens to the aggregate, not the internal mechanism.

> ✅ `account.buy_holding(...)` — `account.sell_holding(...)`
> ❌ `account.create_transaction(...)` — `account.upsert_holding(...)`

## Bounded Context (`/context`)

**B5** — MUST never import from another context.

**B6** — MUST share its external API directly through its main `mod.rs`.

- Outside the context, never import `crate::context::account::domain::MyDomainObject` — always import `crate::context::account::MyDomainObject`.

**B7** — SHOULD always publish a `{Domain}Updated` event when its state changes (create, update, delete, etc.). The BC Application Service (`service.rs`) is responsible for event emission. If no Application Service exists, the `api.rs` handler is responsible.

**B8** — `api.rs` is the framework boundary — the only layer that knows Tauri exists.
Its sole responsibilities are:

1. **Deserialize** — translate Tauri command arguments into domain types
2. **Delegate** — make exactly one call to its own BC Application Service
3. **Serialize** — map the result to `Result<T, String>` for Tauri

It MUST only call the Application Service of its own bounded context.
It MUST NOT call another BC's service, another BC's repository, or a use case.
Cross-BC coordination belongs in a use case with its own `api.rs` (B13).

**B9** — MUST declare its Tauri commands in the `api.rs` file.

## Use Cases (`/use_cases`)

**B10** — MAY import from contexts, MUST NOT import from another use case.

**B11** — MUST share its external API directly through its main `mod.rs`.

**B12** — MUST NOT publish a `{Domain}Updated` event directly (orchestrators do not own state).
For cross-aggregate UoW operations, MUST delegate notification to each BC service's notify
method after commit — the service owns the event, not the use case.

**B13** — MUST declare its Tauri commands in its own `api.rs` file. This `api.rs` follows
the same framework boundary role as B8: deserialize → delegate to the use case orchestrator
→ serialize. It MUST NOT contain coordination logic — that belongs in the orchestrator.

**B14** — SHOULD have an orchestrator as its main entry point (after api) that handles the global logic.

## Repository

**B15** — MUST use sqlx macros for queries. Use `just clean-db` to reset the database if needed.

## Logging

**B16** — MUST use `tracing::{info, debug, warn, error}` with structured fields. Never use `println!`.

**B17** — MUST use `target:` field when adding a new backend specific log.

**B18** — When using the `target:` field in tracing calls, MUST use the `BACKEND` or `FRONTEND` constant from `crate::core::logger` instead of string literals:

```rust
use crate::core::logger::BACKEND;
tracing::info!(target: BACKEND, field = value, "message");
```

## Application Service & Use Case

**B19** — Use cases MAY depend on any domain abstraction: repository traits, domain entities,
or bounded context services. They MUST NOT depend on infrastructure: concrete repository
implementations, `sqlx::Pool`, `sqlx::Transaction`, `sqlx::query!`, or any other sqlx type.

**B20** — For write operations that must emit an event, use cases SHOULD go through the BC Application Service rather than the repository trait directly to ensure the event is properly fired.

**B21** — A bounded context service (`service.rs`) is a BC-scoped Application Service. Its role
is to orchestrate the aggregate: load via repository → call Aggregate Root method → save →
emit event. It MUST NOT contain domain logic (VWAP, P&L, invariants) — that belongs in the
Aggregate Root entity. It MUST only exist when this orchestration adds value; trivial CRUD
with no event or aggregate coordination does not justify a service. A service MUST NOT expose
repository types or sqlx types in its public signature.

**B22** — For cross-aggregate writes (operations that must write to more than one aggregate
atomically), the use case orchestrator MUST use the UnitOfWork pattern (`TransactionManager`
from `core/uow.rs`). Single-aggregate writes do NOT use UoW — `AccountRepository::save()`
handles atomicity internally. See `docs/adr/006-unit-of-work.md`.

## General

**B23** — MUST use `anyhow::Result<T>` for error handling.

- Exception: Tauri command responses use `Result<T, String>`.

**B24** — MAY use `#[allow(clippy::too_many_arguments)]` on domain factory methods.

## Tests

**B25** — Tests MUST NOT be trivial. A trivial test is one that verifies:

- A constructor does not panic
- An empty input returns empty output (no logic traversed)
- A getter returns what was just passed in
- A test helper disguised as a test

**B26** — Unit tests (mod tests inside src/) MUST mock repository dependencies using mockall-generated mocks.

**B27** — Integration tests (tests/ folder) MUST use real SQLite repos. They test cross-layer behavior end-to-end and MUST NOT use mocks.
