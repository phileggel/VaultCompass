# Implementation Plan — Financial Asset Transaction (TRX)

Spec: `docs/spec/financial-asset-transaction.md`
Rules: TRX-010 to TRX-040

---

## 1. Workflow TaskList

- [x] Read Architecture & Rules (`ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/frontend-rules.md`)
- [x] Backend — Step 1: Add `TransactionUpdated` event to `core/event_bus/event.rs`
- [x] Backend — Step 2: Create `Holding` entity + `HoldingRepository` trait in `context/account/domain/`
- [x] Backend — Step 3: Create `SqliteHoldingRepository` in `context/account/repository/`
- [x] Backend — Step 4: Extend `context/account/mod.rs` to re-export Holding types
- [x] Backend — Step 5: Create `Transaction` entity + `TransactionRepository` trait in `context/transaction/domain/`
- [x] Backend — Step 6: Create `SqliteTransactionRepository` in `context/transaction/repository/`
- [x] Backend — Step 7: Create `TransactionService` in `context/transaction/service.rs`
- [x] Backend — Step 8: Create `context/transaction/mod.rs` and `context/transaction/api.rs` (stub — commands live in use case)
- [x] Backend — Step 9: Create DB migration for `transactions` table
- [x] Backend — Step 10: Create `use_cases/record_transaction/` (orchestrator + api.rs + mod.rs)
- [x] Backend — Step 11: Wire new service + use case into `lib.rs` and `core/specta_builder.rs`
- [x] Backend — Step 12: Register `context/transaction` in `context/mod.rs`
- [x] Type Synchronization (`just generate-types` + `cargo sqlx prepare`)
- [x] Frontend — Step 1: Create `features/transactions/gateway.ts`
- [x] Frontend — Step 2: Create `features/transactions/useTransactions.ts` hook
- [x] Frontend — Step 3: Create shared types + presenter + validator in `features/transactions/shared/`
- [x] Frontend — Step 4: Create `add_transaction/` sub-feature (modal + hook + test)
- [x] Frontend — Step 5: Create `edit_transaction_modal/` sub-feature (modal + hook + test)
- [x] Frontend — Step 6: Add "Buy" action entry point in `features/assets/asset_table/`
- [x] Frontend — Step 7: Wire `TransactionUpdated` into `lib/store.ts` event dispatch
- [x] Frontend — Step 8: Create `features/transactions/index.ts`
- [x] i18n: Add `transaction.*` keys to `fr/common.json` and `en/common.json`
- [x] Formatting & Linting (`just format` + `./scripts/check.sh`)
- [x] Code Review (`reviewer`)
- [x] UX Review (`ux-reviewer` — .tsx files modified)
- [x] i18n Review (`i18n-checker` — UI text added)
- [x] Unit & Integration Tests (backend: `#[cfg(test)]` inline; frontend: `.test.ts` colocated)
- [x] Documentation Update (`ARCHITECTURE.md` + `docs/todo.md`)
- [x] Final Validation (`spec-checker` + `workflow-validator`)

---

## 2. Detailed Implementation Plan

### Constraints from ADRs

- **ADR-001**: All financial fields (`quantity`, `unit_price`, `exchange_rate`, `fees`, `total_amount`, `average_price`) are `i64` in Rust and `INTEGER` in SQLite. Conversion between decimal user input and micro-units happens only at the UI boundary.
- **ADR-002**: `Holding` replaces `AssetAccount`. The `holdings` table already exists via migration `202604120001`. No additional schema migration for holdings is needed.
- **B8**: `TransactionService` (owned by `context/transaction/`) publishes `TransactionUpdated`. The use case orchestrator does NOT publish events.

---

### Layer 0 — Database Migration

The `holdings` table is already created by `src-tauri/migrations/202604120001_replace_asset_accounts_with_holdings.sql`.

A new migration is required for the `transactions` table.

**New file**: `src-tauri/migrations/202604120002_create_transactions.sql`

Schema:

```sql
CREATE TABLE IF NOT EXISTS transactions (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    transaction_type TEXT NOT NULL DEFAULT 'Purchase',
    date TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    unit_price INTEGER NOT NULL,
    exchange_rate INTEGER NOT NULL,
    fees INTEGER NOT NULL DEFAULT 0,
    total_amount INTEGER NOT NULL,
    note TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_transactions_account_asset
    ON transactions (account_id, asset_id, date);
```

Notes:

- `ON DELETE CASCADE` on `account_id` implements TRX's implicit cascade (deleting an account purges its transactions).
- `ON DELETE RESTRICT` on `asset_id` prevents orphan transactions (assets must be deleted explicitly after removing transactions).
- The composite index on `(account_id, asset_id, date)` supports chronological queries (TRX-036).
- After adding this migration, run `just clean-db` then `cargo sqlx prepare` to regenerate the offline query cache.

---

### Layer 1 — Backend

#### 1.1 — Event Bus: Add `TransactionUpdated`

**File**: `src-tauri/src/core/event_bus/event.rs`

Add `TransactionUpdated` variant to the `Event` enum. The variant carries no payload (consistent with `AssetUpdated`, `AccountUpdated`). This satisfies **TRX-037**.

```
TransactionUpdated,
```

#### 1.2 — `Holding` Entity + Repository Trait

**New file**: `src-tauri/src/context/account/domain/holding.rs`

The `Holding` struct fields (all per ADR-001 i64 micro-units):

- `id: String`
- `account_id: String`
- `asset_id: String`
- `quantity: i64`
- `average_price: i64`

Factory methods (B1):

- `new(account_id, asset_id, quantity, average_price) -> Result<Self>` — generates UUID, validates quantity >= 0 and average_price >= 0.
- `with_id(id, account_id, asset_id, quantity, average_price) -> Result<Self>` — uses provided ID, same validation.
- `restore(id, account_id, asset_id, quantity, average_price) -> Self` — no validation, for repository reconstruction.

The `HoldingRepository` trait (async_trait):

- `get_by_account(account_id: &str) -> Result<Vec<Holding>>`
- `get_by_account_asset(account_id: &str, asset_id: &str) -> Result<Option<Holding>>`
- `create(holding: Holding) -> Result<Holding>`
- `update(holding: Holding) -> Result<Holding>`
- `upsert(holding: Holding) -> Result<Holding>` — `INSERT OR REPLACE` pattern needed by use case for idempotent update
- `delete(id: &str) -> Result<()>`
- `delete_by_account_asset(account_id: &str, asset_id: &str) -> Result<()>` — used by TRX-034 when quantity reaches zero

**Update**: `src-tauri/src/context/account/domain/mod.rs`

Add `mod holding; pub use holding::*;`

#### 1.3 — `SqliteHoldingRepository`

**New file**: `src-tauri/src/context/account/repository/holding.rs`

Implement `HoldingRepository` for `SqliteHoldingRepository`:

- Uses `sqlx::query_as!` macros (B11).
- `upsert` uses `INSERT INTO holdings (...) VALUES (...) ON CONFLICT(account_id, asset_id) DO UPDATE SET ...` to atomically create-or-update a holding.
- `get_by_account` fetches all holdings for an account (used for holdings display).
- `delete_by_account_asset` removes the holding when no transactions remain (TRX-034).

**Update**: `src-tauri/src/context/account/repository/mod.rs`

Add `mod holding; pub use holding::SqliteHoldingRepository;`

#### 1.4 — Extend `context/account/mod.rs`

**File**: `src-tauri/src/context/account/mod.rs`

Add public re-exports for `Holding`, `HoldingRepository`, `SqliteHoldingRepository`. These are consumed by the use case layer.

#### 1.5 — `Transaction` Entity + Repository Trait

**New directory**: `src-tauri/src/context/transaction/`

**New file**: `src-tauri/src/context/transaction/domain/transaction.rs`

```
TransactionType enum: Purchase  (Sell deferred — TRX-040)
  derives: Debug, Serialize, Deserialize, Clone, Type, PartialEq, Eq, strum Display, EnumString
```

`Transaction` struct fields (all i64 per ADR-001):

- `id: String`
- `account_id: String`
- `asset_id: String`
- `transaction_type: TransactionType`
- `date: String` (ISO 8601, "YYYY-MM-DD")
- `quantity: i64`
- `unit_price: i64`
- `exchange_rate: i64`
- `fees: i64`
- `total_amount: i64`
- `note: Option<String>`

Factory methods (B1):

- `new(account_id, asset_id, transaction_type, date, quantity, unit_price, exchange_rate, fees, total_amount, note) -> Result<Self>` — generates UUID, applies validation rules (TRX-020, TRX-026).
  - `date` must be parseable as a NaiveDate, not in the future, and not before 1900-01-01.
  - `quantity` must be > 0.
  - `unit_price` must be >= 0.
  - `total_amount` must be > 0.
  - Total amount invariant: `total_amount == (quantity * unit_price / 1_000_000) * exchange_rate / 1_000_000 + fees`. All values are i64 micro-units; integer arithmetic must satisfy this exact equality (TRX-026).
- `with_id(id, ...) -> Result<Self>` — identical validation with provided ID (used by use case on update).
- `restore(...) -> Self` — no validation, used only in repository.

Mark `new()` and `with_id()` with `#[allow(clippy::too_many_arguments)]` (B14).

`TransactionRepository` trait:

- `get_by_id(id: &str) -> Result<Option<Transaction>>`
- `get_by_account_asset(account_id: &str, asset_id: &str) -> Result<Vec<Transaction>>` — ordered by `date ASC` (TRX-036)
- `create(tx: Transaction) -> Result<Transaction>`
- `update(tx: Transaction) -> Result<Transaction>`
- `delete(id: &str) -> Result<()>`

**New file**: `src-tauri/src/context/transaction/domain/mod.rs`

```rust
mod transaction;
pub use transaction::*;
```

#### 1.6 — `SqliteTransactionRepository`

**New file**: `src-tauri/src/context/transaction/repository/transaction.rs`

Implement `TransactionRepository` for `SqliteTransactionRepository`:

- `get_by_account_asset` orders by `date ASC` then `rowid ASC` for stable chronological ordering (TRX-036).
- Uses `sqlx::query_as!` with a `TransactionRow` struct that maps to the DB columns.
- `restore()` is used to reconstruct `Transaction` from rows.

**New file**: `src-tauri/src/context/transaction/repository/mod.rs`

```rust
mod transaction;
pub use transaction::SqliteTransactionRepository;
```

#### 1.7 — `TransactionService`

**New file**: `src-tauri/src/context/transaction/service.rs`

Responsibilities (B4 — publishes `TransactionUpdated` after each mutation):

- `new(repo: Box<dyn TransactionRepository>) -> Self`
- `with_event_bus(bus: Arc<SideEffectEventBus>) -> Self`
- `get_by_account_asset(account_id, asset_id) -> Result<Vec<Transaction>>` — delegates to repo
- `create(tx: Transaction) -> Result<Transaction>` — persists via repo, then publishes `TransactionUpdated`
- `update(tx: Transaction) -> Result<Transaction>` — persists via repo, then publishes `TransactionUpdated`
- `delete(id: &str) -> Result<()>` — deletes via repo, then publishes `TransactionUpdated`

The service does NOT orchestrate holdings. That is the use case's responsibility (B8).

#### 1.8 — `context/transaction/api.rs` and `mod.rs`

**New file**: `src-tauri/src/context/transaction/api.rs`

This file is intentionally minimal. Commands for the transaction feature live in the use case (B9). The file exists to satisfy B5 layout but declares no commands — or holds helper types used only in the transaction context.

**New file**: `src-tauri/src/context/transaction/mod.rs`

```rust
mod api;
mod domain;
mod repository;
mod service;

pub use domain::*;
pub use repository::*;
pub use service::*;
```

#### 1.9 — `context/mod.rs` — Register new context

**File**: `src-tauri/src/context/mod.rs`

Add:

```rust
pub mod transaction;
```

#### 1.10 — Use Case: `record_transaction`

**New directory**: `src-tauri/src/use_cases/record_transaction/`

This use case orchestrates across `transaction/`, `account/`, and queries `asset/` (B6, B10). It does NOT publish any `*Updated` event directly (B8).

##### Orchestrator: `src-tauri/src/use_cases/record_transaction/orchestrator.rs`

Struct `RecordTransactionUseCase`:

```
fields:
  transaction_service: Arc<TransactionService>
  holding_repo: Arc<dyn HoldingRepository>
  asset_repo: Arc<dyn AssetRepository>   // for existence + archived-status checks
```

Constructor: `new(transaction_service, holding_repo, asset_repo) -> Self`

Method: `create_transaction(dto: CreateTransactionDTO) -> Result<Transaction>`

1. Validate asset existence via `asset_repo.get_by_id(dto.asset_id)` — bail if not found (TRX-020).
2. Validate account existence via `account_repo.get_by_id(dto.account_id)` — bail if not found (TRX-020).
   - Note: The orchestrator receives an `Arc<dyn AccountRepository>` for existence checks only.
3. Check if asset is archived (TRX-028): if archived, call `asset_repo.unarchive(dto.asset_id)` within the same DB transaction (see atomicity note below).
4. Build `Transaction` via `Transaction::new(...)` — validates TRX-020 + TRX-026.
5. Atomically (single sqlx transaction wrapping all steps — TRX-027):
   a. Persist transaction via `transaction_service.create(transaction)`.
   b. Recompute holding via `recalculate_holding(account_id, asset_id)` (see below).
6. Return the created `Transaction`.

Method: `update_transaction(id: String, dto: CreateTransactionDTO) -> Result<Transaction>`

1. Fetch existing transaction via `transaction_service.get_by_id(id)` to capture old `(account_id, asset_id)`.
2. Validate new asset + account existence (TRX-033).
3. Check archived status of new asset (TRX-028).
4. Build updated `Transaction` via `Transaction::with_id(id, ...)` — validates TRX-033.
5. Atomically (TRX-027):
   a. Persist updated transaction via `transaction_service.update(transaction)`.
   b. If `(account_id, asset_id)` changed: recalculate holding for the OLD pair.
   c. Recalculate holding for the NEW pair.

Method: `delete_transaction(id: &str) -> Result<()>`

1. Fetch existing transaction to capture `(account_id, asset_id)`.
2. Atomically (TRX-027):
   a. Delete via `transaction_service.delete(id)`.
   b. Recalculate holding for `(account_id, asset_id)`.
   c. If no transactions remain for the pair, delete the Holding via `holding_repo.delete_by_account_asset(...)` (TRX-034).

Private method: `recalculate_holding(account_id, asset_id)` — implements TRX-030 + TRX-036:

1. Fetch all transactions for the pair (chronological) via `transaction_service.get_by_account_asset(...)`.
2. Filter to `Purchase` only (TRX-030).
3. Compute:
   - `total_quantity = sum(t.quantity)`
   - VWAP numerator: `sum(t.quantity * t.unit_price / 1_000_000 * t.exchange_rate / 1_000_000)`
   - `average_price = if total_quantity > 0 { vwap_numerator / total_quantity } else { 0 }`
   - Note: all arithmetic in i64; the division order matters to avoid overflow. Use 128-bit intermediates (`i128`) for products before dividing.
4. Upsert the holding via `holding_repo.upsert(Holding::with_id(...))`.

##### DTO types: `src-tauri/src/use_cases/record_transaction/orchestrator.rs` (or a separate `dto.rs`)

```rust
pub struct CreateTransactionDTO {
    pub account_id: String,
    pub asset_id: String,
    pub date: String,
    pub quantity: i64,
    pub unit_price: i64,
    pub exchange_rate: i64,
    pub fees: i64,
    pub total_amount: i64,
    pub note: Option<String>,
}
```

##### API handlers: `src-tauri/src/use_cases/record_transaction/api.rs`

Tauri commands (B9, B5):

- `add_transaction(state, account_id, asset_id, date, quantity, unit_price, exchange_rate, fees, total_amount, note) -> Result<Transaction, String>`
- `update_transaction(state, id, account_id, asset_id, date, quantity, unit_price, exchange_rate, fees, total_amount, note) -> Result<Transaction, String>`
- `delete_transaction(state, id) -> Result<(), String>`
- `get_transactions(state, account_id, asset_id) -> Result<Vec<Transaction>, String>`

All handlers extract `RecordTransactionUseCase` from Tauri state and delegate.

State injection: `RecordTransactionUseCase` is managed as Tauri state in `lib.rs`.

##### Module wiring: `src-tauri/src/use_cases/record_transaction/mod.rs`

```rust
mod api;
mod orchestrator;

pub use api::*;
pub use orchestrator::*;
```

**Update**: `src-tauri/src/use_cases/mod.rs` — add `pub mod record_transaction;`

#### 1.11 — Wire into `lib.rs`

**File**: `src-tauri/src/lib.rs`

In `run()`, after `account_service` construction:

1. Create `SqliteTransactionRepository::new(pool.clone())`.
2. Create `TransactionService::new(Box::new(transaction_repo)).with_event_bus(event_bus.clone())`.
3. Create `SqliteHoldingRepository::new(pool.clone())`.
4. Create `RecordTransactionUseCase::new(Arc::new(transaction_service), Arc::new(holding_repo), Arc::new(asset_repo_for_uc))`.
5. `app_handle.manage(record_transaction_use_case)`.

Add `AppState` field: `transaction_service` and/or manage `RecordTransactionUseCase` separately.

#### 1.12 — `core/specta_builder.rs`

**File**: `src-tauri/src/core/specta_builder.rs`

Add:

```rust
.typ::<transaction::Transaction>()
.typ::<transaction::TransactionType>()
.typ::<account::Holding>()
```

Add to `collect_commands!`:

```
record_transaction::add_transaction,
record_transaction::update_transaction,
record_transaction::delete_transaction,
record_transaction::get_transactions,
```

---

### Layer 2 — Type Synchronization

After completing all backend changes:

```
just clean-db          # reset SQLite + re-apply migrations
cargo sqlx prepare     # regenerate offline query cache
just generate-types    # regenerate src/bindings.ts
```

Verify `src/bindings.ts` contains: `Transaction`, `TransactionType`, `Holding`, `CreateTransactionDTO` (if exported as a Specta type), and the four new commands.

---

### Layer 3 — Frontend

#### 3.1 — Gateway

**New file**: `src/features/transactions/gateway.ts`

Methods wrapping the four Tauri commands (match bindings.ts positional signatures exactly — CLAUDE.md critical pattern):

- `addTransaction(accountId, assetId, date, quantity, unitPrice, exchangeRate, fees, totalAmount, note) -> Promise<Result<Transaction, string>>`
- `updateTransaction(id, accountId, assetId, date, quantity, unitPrice, exchangeRate, fees, totalAmount, note) -> Promise<Result<Transaction, string>>`
- `deleteTransaction(id) -> Promise<Result<null, string>>`
- `getTransactions(accountId, assetId) -> Promise<Result<Transaction[], string>>`

Import from `../../bindings`.

#### 3.2 — `useTransactions` hook

**New file**: `src/features/transactions/useTransactions.ts`

Provides CRUD callbacks wrapping gateway calls with error normalization (same pattern as `useAccounts`):

- `addTransaction(dto)`, `updateTransaction(dto)`, `deleteTransaction(id)`, `getTransactions(accountId, assetId)`

Does not hold domain state (holdings are managed in a feature-scoped store).

#### 3.3 — Shared types, presenter, validator

**New file**: `src/features/transactions/shared/types.ts`

```ts
export interface TransactionFormData {
  accountId: string;
  assetId: string;
  date: string; // YYYY-MM-DD string
  quantity: string; // decimal string — converted to i64 at submit
  unitPrice: string; // decimal string
  exchangeRate: string; // decimal string, default "1.000000"
  fees: string; // decimal string, default "0"
  totalAmount: string; // decimal string — read-only, auto-calculated
  note: string;
}
```

**New file**: `src/features/transactions/shared/microUnits.ts`

Pure utility functions (ADR-001 — conversion happens only at the UI boundary, TRX-024):

- `decimalToMicro(value: string): i64` — parses decimal string, multiplies by 1_000_000, returns bigint or number (choose number if < Number.MAX_SAFE_INTEGER, otherwise note the limitation).
- `microToDecimal(micros: number, decimals?: number): string` — divides by 1_000_000, formats to `decimals` places (default 3 per TRX-024 spec: "3 decimal places").
- `calculateTotalAmount(quantity: string, unitPrice: string, exchangeRate: string, fees: string): string` — computes `(quantity × unit_price × exchange_rate) + fees` in micro-units, returns decimal string for display. Used for auto-calculation in the form (TRX-026).

**New file**: `src/features/transactions/shared/presenter.ts`

Pure transformers:

- `toTransactionRow(tx: Transaction, assetName: string, accountName: string) -> TransactionRowViewModel` — maps raw backend `Transaction` to a display-ready shape (formatted date, formatted amounts with 3 decimals).

**New file**: `src/features/transactions/shared/validateTransaction.ts`

Pure validation (client-side, mirrors TRX-020):

- `validateTransactionForm(data: TransactionFormData): string | null` — returns first error message key or null if valid.
  - account and asset must be non-empty.
  - date must not be empty.
  - quantity must be > 0.
  - totalAmount must be > 0.

#### 3.4 — `add_transaction` sub-feature

**New file**: `src/features/transactions/add_transaction/useAddTransaction.ts`

Hook props:

```ts
interface UseAddTransactionProps {
  prefillAccountId?: string;
  prefillAssetId?: string;
  onSubmitSuccess?: () => void;
}
```

State:

- `formData: TransactionFormData` — initialized with defaults per TRX-023: `date = today`, `exchangeRate = "1.000000"`, `fees = "0"`, `transaction_type = "Purchase"` (hidden).
- `error: string | null`
- `isSubmitting: boolean`
- `showArchivedConfirm: boolean` — TRX-029: whether to show archived-asset confirmation dialog.

Key behaviors:

- Pre-fill `accountId` and `assetId` from props (TRX-011).
- When `assetId` changes, check if the selected asset `is_archived` (from the global store) and set `showArchivedConfirm = true` if so (TRX-029).
- `totalAmount` is auto-calculated and read-only: recomputed via `calculateTotalAmount(...)` on every change to `quantity`, `unitPrice`, `exchangeRate`, or `fees`.
- `handleSubmit`: converts all string fields to i64 micro-units using `decimalToMicro()`, calls `addTransaction()`, handles success/error.
- `handleConfirmArchived` / `handleCancelArchived` for TRX-029 dialog flow.

**New file**: `src/features/transactions/add_transaction/AddTransactionModal.tsx`

`FormModal`-based component:

- Fields per spec UX Draft: Account (SelectField), Asset (ComboboxField with fuzzy search), Date (DateField), Quantity (TextField), Unit Price (AmountField with asset currency), Exchange Rate (TextField, visible only if asset currency != account currency), Fees (AmountField with account currency), Total Amount (AmountField read-only), Note (textarea).
- `transaction_type` is NOT shown (TRX-023).
- If `showArchivedConfirm`, renders a `ConfirmationDialog` warning the user the asset will be unarchived (TRX-029).
- Loading, error, empty states as per spec UX Draft.
- Logs `info` on mount (F13).

**New file**: `src/features/transactions/add_transaction/useAddTransaction.test.ts`

Tests to cover (F18 — non-trivial logic):

- Pre-fill with `prefillAssetId` sets `formData.assetId` on init.
- `totalAmount` is recomputed when `quantity` changes.
- When the selected asset is archived, `showArchivedConfirm` becomes true.
- Cancelling the archived confirmation does not submit.
- `handleSubmit` calls `addTransaction` with correct micro-unit values.
- Backend error sets `error` and does not call `onSubmitSuccess`.
- Success clears form and calls `onSubmitSuccess`.

#### 3.5 — `edit_transaction_modal` sub-feature

**New file**: `src/features/transactions/edit_transaction_modal/useEditTransactionModal.ts`

Props: `transaction: Transaction`, `onSubmitSuccess?: () => void`

Similar to `useAddTransaction` but pre-fills all fields from the existing transaction (converting micro-units to decimal strings via `microToDecimal`). On submit, calls `updateTransaction`.

Also handles TRX-029 archived-asset guard on the (possibly changed) asset.

**New file**: `src/features/transactions/edit_transaction_modal/EditTransactionModal.tsx`

Reuses `TransactionForm` shared component (same form fields as add) with an edit title.

**New file**: `src/features/transactions/edit_transaction_modal/useEditTransactionModal.test.ts`

Tests: pre-fill populates formData correctly (micro-unit to decimal conversion), submit calls `updateTransaction` with correct args.

#### 3.6 — Delete confirmation

There is no separate sub-feature directory for delete; it is handled inline by a `ConfirmationDialog` triggered from the transaction list or table. The confirmation requirement is TRX-035.

The delete action lives in `useTransactions` (or the table hook) and calls `deleteTransaction(id)`.

#### 3.7 — Entry point: "Buy" action in Asset Table

**File**: `src/features/assets/asset_table/AssetTable.tsx` (modify)

Add a "Buy" `IconButton` or contextual action per row (TRX-010). When clicked, opens `AddTransactionModal` with `prefillAssetId` set to the row's asset ID (TRX-011).

**File**: `src/features/assets/asset_table/useAssetTable.ts` (may need extension)

Add `selectedAssetForTransaction: string | null` and `setSelectedAssetForTransaction` to manage the entry-point state, or keep it in the component if purely UI.

#### 3.8 — `TransactionUpdated` event in global store

**File**: `src/lib/store.ts`

The `TransactionUpdated` event must trigger a holdings refresh. Holdings are not currently in the global store (they are account-specific and fetched per-account). The recommended approach:

Add `fetchHoldingsForAccount(accountId: string)` as a store action, or signal to the active account view to re-fetch.

Preferred implementation (feature-scoped store):

Create `src/features/transactions/store.ts` — a Zustand slice for the transactions feature:

```ts
interface TransactionStore {
  holdingsByAccount: Record<string, Holding[]>;
  isLoadingHoldings: boolean;
  fetchHoldings: (accountId: string) => Promise<void>;
}
```

On `TransactionUpdated`, call `fetchHoldings(affectedAccountId)`. However, the event carries no `accountId` payload. Options:

- Option A (recommended): The store dispatches a full re-fetch for all currently displayed accounts on `TransactionUpdated`. Since the event bus carries no account info, the simplest approach is to refresh the holdings for the account currently in view.
- Option B: Extend the `TransactionUpdated` event variant to carry `account_id: String` as payload (requires backend change). This gives targeted refresh.

**Decision for plan**: Use Option A initially (re-fetch holdings for the current account on `TransactionUpdated`). This keeps the event bus minimal. Wire in `lib/store.ts` `eventMap`:

```ts
TransactionUpdated: () => {
  // signal the transactions feature store to re-fetch
  useTransactionStore.getState().refreshHoldings();
};
```

`refreshHoldings()` in the feature store refreshes the holdings of the last-known active account.

**File**: `src/lib/store.ts` — Add `TransactionUpdated` to `eventMap`, delegating to the transaction feature store's refresh function (TRX-038).

#### 3.9 — `features/transactions/index.ts`

Public re-exports: `AddTransactionModal`, `EditTransactionModal`, `useTransactions`, `transactionGateway`.

---

### Layer 4 — i18n

**Files**: `src/i18n/locales/fr/common.json` and `src/i18n/locales/en/common.json`

Add a `transaction` key group. Required keys:

```
transaction.add_modal_title
transaction.edit_modal_title
transaction.delete_confirm_title
transaction.delete_confirm_message
transaction.archived_asset_confirm_title
transaction.archived_asset_confirm_message
transaction.form_account_label
transaction.form_asset_label
transaction.form_date_label
transaction.form_quantity_label
transaction.form_unit_price_label
transaction.form_exchange_rate_label
transaction.form_fees_label
transaction.form_total_amount_label
transaction.form_note_label
transaction.form_note_placeholder
transaction.action_buy        (entry point label in asset table)
transaction.error_load
transaction.error_generic
transaction.error_validation_quantity
transaction.error_validation_total
transaction.error_invariant_mismatch
transaction.success_created
transaction.success_updated
transaction.success_deleted
```

---

### 3. Rules Coverage

### TRX-010 — Purchase entry point (frontend)

Layer: Frontend
Implementation: Add "Buy" `IconButton` to `AssetTable` row → opens `AddTransactionModal` with `prefillAssetId`. FAB or button in Account Details view (if Account Details view exists at time of implementation) → opens `AddTransactionModal` with `prefillAccountId`.
File: `src/features/assets/asset_table/AssetTable.tsx` (modify)

### TRX-011 — Contextual pre-filling (frontend)

Layer: Frontend
Implementation: `useAddTransaction` hook accepts `prefillAccountId` and `prefillAssetId` props, initializes `formData` fields from those values. Both can be set when entry point has full context.
File: `src/features/transactions/add_transaction/useAddTransaction.ts`

### TRX-020 — Field validation (backend)

Layer: Backend
Implementation: `Transaction::new()` and `Transaction::with_id()` validate: `date` not in future, not before 1900-01-01; `quantity > 0`; `unit_price >= 0`; `total_amount > 0`. Account and asset existence verified in `RecordTransactionUseCase.create_transaction()` and `update_transaction()`.
Files: `context/transaction/domain/transaction.rs`, `use_cases/record_transaction/orchestrator.rs`

### TRX-021 — Multi-currency semantics (backend)

Layer: Backend
Implementation: `unit_price` stored as-is in asset's native currency. `exchange_rate` stored explicitly with the transaction as an i64 micro-unit. No currency conversion happens in the service; the invariant check (TRX-026) treats all values as already in their correct currencies.
File: `context/transaction/domain/transaction.rs`

### TRX-022 — Holding quantity update (backend)

Layer: Backend
Implementation: `recalculate_holding()` in `RecordTransactionUseCase` computes `sum(t.quantity)` for all purchases and upserts the `Holding`.
File: `use_cases/record_transaction/orchestrator.rs`

### TRX-023 — Form default values (frontend)

Layer: Frontend
Implementation: `useAddTransaction` initializes `formData.date = today` (ISO string), `formData.exchangeRate = "1.000000"`. `transaction_type` not shown in form and hardcoded to `"Purchase"` in the submit call.
File: `src/features/transactions/add_transaction/useAddTransaction.ts`

### TRX-024 — Micro-unit representation (full stack)

Layer: Full stack
Implementation: All backend fields are `i64`. `decimalToMicro()` and `microToDecimal()` in `shared/microUnits.ts` handle conversions at the UI boundary only.
Files: `context/transaction/domain/transaction.rs` (backend), `src/features/transactions/shared/microUnits.ts` (frontend)

### TRX-025 — Holding cost basis update (backend)

Layer: Backend
Implementation: `recalculate_holding()` in orchestrator computes VWAP and upserts `average_price`. Called after every transaction mutation.
File: `use_cases/record_transaction/orchestrator.rs`

### TRX-026 — Total amount formula invariant (backend)

Layer: Backend
Implementation: `Transaction::new()` and `Transaction::with_id()` verify that `total_amount == (quantity × unit_price / 1_000_000 × exchange_rate / 1_000_000) + fees` using i128 intermediates to prevent overflow. Returns error if invariant fails.
File: `context/transaction/domain/transaction.rs`

### TRX-027 — Atomicity (backend)

Layer: Backend
Implementation: `create_transaction()`, `update_transaction()`, and `delete_transaction()` in `RecordTransactionUseCase` wrap all DB operations in a single `sqlx::Transaction`. The orchestrator requires access to a `Pool<Sqlite>` directly (or a shared connection) for the database transaction boundary. Note: the orchestrator will need to accept a `Pool<Sqlite>` (or `Arc<Pool<Sqlite>>`) to begin and commit the sqlx transaction. The inner service and repo calls receive `&mut Transaction<Sqlite>` as a parameter, or the orchestrator handles it at the pool level.

Implementation approach: The orchestrator holds an `Arc<Pool<Sqlite>>` and opens a `sqlx::Transaction` explicitly, then calls service/repo methods that accept an `Executor` parameter (distinct from the standard Arc-pool-based implementations). This requires the orchestrator to use low-level sqlx directly for the atomic block, or introduce `*_with_tx(tx: &mut sqlx::Transaction)` method variants.

Files: `use_cases/record_transaction/orchestrator.rs`, potentially `context/transaction/repository/transaction.rs` and `context/account/repository/holding.rs`

### TRX-028 — Archived asset auto-unarchive (backend)

Layer: Backend
Implementation: In `create_transaction()` and `update_transaction()` in the orchestrator: if `asset.is_archived`, call `asset_repo.unarchive(asset_id)` within the same database transaction (TRX-027). If the overall operation fails, the unarchive is rolled back.
File: `use_cases/record_transaction/orchestrator.rs`

### TRX-029 — Archived asset confirmation (frontend)

Layer: Frontend
Implementation: `useAddTransaction` checks `is_archived` on the selected asset (from global store). Sets `showArchivedConfirm = true`. `AddTransactionModal` renders a `ConfirmationDialog`. Submission proceeds only after user confirms.
Files: `src/features/transactions/add_transaction/useAddTransaction.ts`, `AddTransactionModal.tsx`

### TRX-030 — VWAP Calculation (backend)

Layer: Backend
Implementation: `recalculate_holding()` in orchestrator filters to `Purchase` type only, computes `sum(q * p * er) / sum(q)` using i128 intermediates.
File: `use_cases/record_transaction/orchestrator.rs`

### TRX-031 — Transaction modification triggers recalculation (backend)

Layer: Backend
Implementation: `update_transaction()` in orchestrator calls `recalculate_holding()` for affected `(account_id, asset_id)` pair(s) after persisting the update.
File: `use_cases/record_transaction/orchestrator.rs`

### TRX-032 — All fields modifiable + cross-pair handling (backend)

Layer: Backend
Implementation: `update_transaction()` accepts a full `CreateTransactionDTO`. Captures old `(account_id, asset_id)` before updating. After update, recalculates old pair (if changed) and new pair.
File: `use_cases/record_transaction/orchestrator.rs`

### TRX-033 — Update validation (backend)

Layer: Backend
Implementation: `Transaction::with_id()` applies the same validation as `new()`. Orchestrator re-checks asset and account existence for the updated values.
Files: `context/transaction/domain/transaction.rs`, `use_cases/record_transaction/orchestrator.rs`

### TRX-034 — Delete + recalculation (backend)

Layer: Backend
Implementation: `delete_transaction()` in orchestrator deletes the transaction, recalculates holding, removes holding if no transactions remain via `holding_repo.delete_by_account_asset()`.
File: `use_cases/record_transaction/orchestrator.rs`

### TRX-035 — Delete confirmation (frontend)

Layer: Frontend
Implementation: Delete action in transaction list/table triggers a `ConfirmationDialog` before calling `deleteTransaction(id)`.
File: Component that renders the transaction list (TBD per Account Details view implementation).

### TRX-036 — Chronological integrity (backend)

Layer: Backend
Implementation: `get_by_account_asset` in `SqliteTransactionRepository` orders by `date ASC, rowid ASC`. `recalculate_holding()` processes transactions in the returned order.
Files: `context/transaction/repository/transaction.rs`, `use_cases/record_transaction/orchestrator.rs`

### TRX-037 — TransactionUpdated event (backend)

Layer: Backend
Implementation: `TransactionService.create()`, `.update()`, `.delete()` each publish `Event::TransactionUpdated` via the event bus (B4). The use case orchestrator does NOT publish (B8).
Files: `core/event_bus/event.rs` (variant added), `context/transaction/service.rs` (publication)

### TRX-038 — Holdings refresh on event (frontend)

Layer: Frontend
Implementation: `lib/store.ts` adds `TransactionUpdated` to `eventMap`, delegating to `useTransactionStore.getState().refreshHoldings()`. The transaction feature store re-fetches holdings for the active account.
Files: `src/lib/store.ts`, `src/features/transactions/store.ts`

### TRX-040 — Zero quantity handling (backend)

Layer: Backend (deferred)
Status: Out of scope for initial implementation. `Holding` remains in DB if quantity reaches zero. No code change required now. Forward-documented only.

---

### 4. Key Notes and Pitfalls

#### VWAP integer arithmetic and overflow

`quantity * unit_price` can overflow `i64` for large values (e.g., 10^6 micro-units \* 10^6 micro-units). Use `i128` intermediates throughout `recalculate_holding()`. The final `average_price` fits comfortably in `i64`.

#### Atomicity implementation detail (TRX-027)

The sqlx transaction boundary must wrap both `transaction_service.create()` and `holding_repo.upsert()`. The standard service and repository structs use a pool. To achieve atomicity:

Option A (preferred): The orchestrator obtains a `sqlx::Transaction<Sqlite>` from the pool, then calls the raw sqlx queries directly (bypassing the repository trait) within that transaction.

Option B: Introduce `*_with_executor(executor: &mut impl Executor)` variants in the repository and service. This is more idiomatic but more verbose.

**Choose the approach during implementation** and document it inline. Both are acceptable; Option A minimizes boilerplate for the use case.

#### `just clean-db` vs `cargo sqlx prepare`

- Run `just clean-db` before `cargo sqlx prepare` after adding the new migration. This ensures the offline query cache is generated against the new schema.
- `cargo sqlx prepare` requires the `DATABASE_URL` env var (set by `just clean-db`).

#### `account_asset_details` placeholder

Per ADR-002 and ARCHITECTURE.md, the `account_asset_details` feature is a placeholder that will be rebuilt on top of `Holding`. This plan does not rebuild it, but the `AddTransactionModal` entry point from the Account Details view (TRX-010) is noted as TBD pending that rebuild.

#### Frontend: `transaction_type` field

The form never shows `transaction_type`. The gateway call always passes it as `"Purchase"`. It is never declared as a TypeScript constant in the frontend code (F21 — do not redeclare Specta enum values; use the generated `TransactionType` from `bindings.ts` directly where needed).
