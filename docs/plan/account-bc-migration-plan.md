# Account BC Migration Plan

## Goal

Consolidate `Transaction` into the `Account` bounded context, eliminate the
misplaced `context/transaction/` BC and `use_cases/record_transaction/` orchestrator.

---

## Target Architecture

```
context/account/           ← Account (root), Holding (internal), Transaction (internal)
context/asset/             ← Asset (root), AssetPrice (internal)
use_cases/account_details/ ← cross-BC read (account portfolio view)
use_cases/delete_asset/    ← cross-BC guard (check holdings before asset delete)
```

---

## Key Decision: auto_record_price moves to frontend

The `record_price: bool` flag is removed from `CreateTransactionDTO`. Recording a price
(Asset BC) and recording a transaction (Account BC) are independent operations — the coupling
was UX sugar, not a domain invariant. The frontend calls both commands in sequence when
auto-record is enabled. No cross-BC orchestration or UoW needed for this feature.

---

## Migration Phases

### Phase 1 — Move Transaction domain into Account BC

- Move `context/transaction/domain/transaction.rs` → `context/account/domain/`
- Move `context/transaction/domain/error.rs` → `context/account/domain/`
- Move `TransactionRepository` trait into `context/account/domain/`
- Move `context/transaction/repository/` → `context/account/repository/`
- Update `context/account/domain/mod.rs` and `context/account/mod.rs` re-exports
- Delete `context/transaction/`

### Phase 2 — Enrich Account aggregate with domain logic

**Naming principle**: method names on `Account` reflect what the caller tells the account
to do — in holding terms, not transaction terms. The transaction is an internal
consequence, invisible at the aggregate surface.

> Method names below are **placeholders pending user validation** (see `docs/ubiquitous-language.md`).

- `Account` entity gains `Vec<Holding>` and `Vec<Transaction>` as owned fields
- Domain logic moves INTO the entity:
  - `account.buy_holding(asset_id, date, qty, unit_price, exchange_rate, fees, note)` — creates Transaction internally, updates Holding (VWAP, quantity)
  - `account.sell_holding(asset_id, date, qty, unit_price, exchange_rate, fees, note)` — creates Transaction internally, updates Holding (VWAP, P&L, oversell guard)
  - `account.correct_transaction(transaction_id, ...)` — replaces Transaction internally, recalculates affected Holding (VWAP, P&L)
  - `account.cancel_transaction(transaction_id)` — deletes Transaction internally, recalculates or removes Holding
- VWAP, P&L, oversell guard, cascading recalculation all live inside `Account` entity methods
- External code MUST NOT call `Transaction::new()` or `Holding::new()` directly — those are called only inside `Account` methods
- `AccountRepository` gains `get_with_holdings_and_transactions()` and `save()` (persists whole aggregate atomically)

### Phase 3 — Thin AccountService (Application Service)

- `AccountService` methods for holding operations become thin orchestrators:
  - Load aggregate via `account_repo.get_with_holdings_and_transactions()`
  - Call the appropriate `Account` entity method (`buy_holding`, `sell_holding`, etc.)
  - Save via `account_repo.save()`
  - Emit `AccountUpdated` event
- `TransactionService` disappears entirely
- VWAP / P&L logic is no longer in the service — it lives in the entity (Phase 2)

### Phase 4 — Move Tauri commands to Account api.rs

- Holding operation commands move from `use_cases/record_transaction/api.rs` → `context/account/api.rs`
- `add_transaction` (single command with `transaction_type` discriminator) → split into two type-safe commands:
  - `buy_holding(BuyHoldingDTO)` — purchase path only
  - `sell_holding(SellHoldingDTO)` — sell path only, includes oversell / closed position guards
- `update_transaction` → `correct_transaction(id, CorrectTransactionDTO)` — `TypeImmutable` error variant removed (type is now implicit in the command)
- `delete_transaction` → `cancel_transaction(id)`
- `get_transactions` — unchanged, moves to account api.rs
- `CreateTransactionDTO` retired: `transaction_type` and `record_price` fields removed; replaced by `BuyHoldingDTO`, `SellHoldingDTO`, `CorrectTransactionDTO`
- Each command is a thin adapter: parse args → call `AccountService` → return result
- `use_cases/record_transaction/` deleted

### Phase 5 — Add UoW infrastructure (foundation)

- Add `core/uow.rs`:
  - `TransactionManager` trait (generic mechanism, no domain knowledge)
  - `SqlxTransactionManager` implementation (holds pool, begins/commits/rolls back)
- Wire `SqlxTransactionManager` in `lib.rs`
- No use case currently requires UoW — this phase lays the foundation for future cross-aggregate atomic operations

### Phase 6 — Collapse remaining use cases

- `archive_asset/` → cross-BC guard: `AccountService.has_active_holdings_for_asset()` + `AssetService.archive()`. Keep as thin `use_cases/archive_asset/` orchestrator (B8: no cross-BC from api.rs). OQ-6 guard is preserved — archive is blocked if any account holds an active position.
- `delete_asset/` → cross-BC guard: `AccountService.has_holding_entries_for_asset()` + `AssetService.delete()`. Keep as thin `use_cases/delete_asset/` orchestrator (B8: no cross-BC from api.rs)
- `use_cases/account_details/` survives as legitimate cross-BC read use case

### Phase 7 — Tests, bindings, docs

- `just generate-types` → refresh `bindings.ts`
- Move integration tests to match new BC structure
- Update `ARCHITECTURE.md`, `backend-rules.md`, `docs/spec/`

---

## DDD Challenge

### Aggregate — B2, B3, B4

**B2 — External code must not mutate internal entities directly.**

`account_details/` use case reads `Transaction` and `Holding` data for display.
Resolution: export `Transaction` and `Holding` as read-only data types only (CQRS-lite).
Mutations go exclusively through `AccountService`. A stricter approach would expose DTOs
instead of raw entities, but adds overhead without clear benefit at this scale.

**B3 — Mutations to internal entities go through the root or its Application Service.**

Satisfied by Phase 2 and 3: all mutations (`buy_holding`, `sell_holding`, etc.)
go through `Account` entity methods, invoked by `AccountService`. No external code
constructs or mutates `Holding` or `Transaction` directly.

**B4 — One transaction modifies at most one aggregate.**

`buy_holding` / `sell_holding`: writes Account aggregate only (Holding + Transaction
in same BC) → ✅ one aggregate. `AccountRepository::save()` handles atomicity internally.

Cross-aggregate writes: no current use case requires them after the `auto_record_price`
decision. UoW infrastructure (Phase 5) is in place if a future operation requires it.

---

### Application Service — B21

`AccountService` is a thin orchestrator: load aggregate → call root method → save → emit event.
It MUST NOT contain domain logic (VWAP, P&L, invariants) — those belong in the `Account` entity.

Risk: the recalculation logic is complex (700+ lines in the current orchestrator). The temptation
will be to leave it in `AccountService` rather than push it into the entity. This must be
resisted — domain logic in the application layer is the original sin of the current design.

---

### api.rs — B8

`context/account/api.rs` handles holding operations — single-BC, calls `AccountService` only. ✅

`context/asset/api.rs` handles price recording — single-BC, calls `AssetService` only. ✅

`delete_asset`: `asset/api.rs` cannot call `AccountService` (B8 violation) — kept as
`use_cases/delete_asset/` orchestrator. ✅

No api.rs crosses BC boundaries.

---

## UoW — Status

UoW infrastructure (`core/uow.rs`) is added in Phase 5 as a foundation. No current use case
requires cross-aggregate atomicity after the `auto_record_price` decision (moved to frontend).

If a future operation requires cross-aggregate atomicity:
- Define `AppUnitOfWork: RepoA + RepoB` in the use case folder
- Inject `TransactionManager` into that use case only
- Load aggregate outside UoW (read) → call aggregate method (pure domain) → inside UoW: `uow.save_account()` + `uow.other_repo_write()` → commit → delegate event notification to each BC service

See `docs/adr/006-unit-of-work.md` for the full architectural decision.
