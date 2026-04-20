# Sell Transaction (SEL) -- Implementation Plan

> Spec: `docs/spec/sell-transaction.md`
> Trigram: SEL (registered in `docs/spec-index.md`, status: planning)
> ADRs: ADR-001 (i64 micro-units), ADR-005 (inject TransactionService into account_details)

---

## 1. Workflow TaskList

- [x] Review Architecture & Rules (`ARCHITECTURE.md`, `backend-rules.md`, `frontend-rules.md`)
- [x] Database Migration (`just migrate` + `just prepare-sqlx`)
- [x] Backend Implementation (Domain, Repository, Service, Orchestrator)
- [x] Type Synchronization (`just generate-types`)
- [x] Commit: backend layer -- `feat(sell): implement sell transaction backend with realized P&L`
- [x] Frontend Implementation (Gateway, Hook, Component, i18n)
- [x] Formatting & Linting (`just format` + `python3 scripts/check.py`)
- [x] Code Review (`reviewer` + `reviewer-backend` + `reviewer-frontend` + `reviewer-sql`)
- [ ] Commit: frontend layer -- `feat(sell): implement sell transaction frontend with P&L display`
- [ ] i18n Review (`i18n-checker`)
- [ ] Unit & Integration Tests
- [ ] Documentation Update (`ARCHITECTURE.md` + `docs/todo.md`)
- [ ] Spec check (`spec-checker`)
- [ ] Commit: tests & docs -- `test(sell): add sell transaction tests and update docs`

---

## 2. Detailed Implementation Plan

### Phase 1 -- Database Migrations

Two migrations are required.

#### Migration 1: Add `realized_pnl` and `created_at` to `transactions`

**File**: `src-tauri/migrations/202604190001_add_realized_pnl_and_created_at_to_transactions.sql`

**Columns**:

- `realized_pnl INTEGER` -- nullable; present only for Sell transactions (NULL for Purchase)
- `created_at TEXT NOT NULL DEFAULT (datetime('now'))` -- ISO 8601 timestamp for chronological tie-breaking (SEL-024)

**Backfill**: Existing rows get `created_at` set to `CURRENT_TIMESTAMP` at migration time (per spec open question resolution -- app is not live).

```
ALTER TABLE transactions ADD COLUMN realized_pnl INTEGER;
ALTER TABLE transactions ADD COLUMN created_at TEXT NOT NULL DEFAULT (datetime('now'));
```

**Post-migration**: Run `just migrate` then `just prepare-sqlx` before writing backend code.

**Note**: The existing chronological ordering index `idx_transactions_account_asset` sorts by `(account_id, asset_id, date)`. After adding `created_at`, the tie-breaking for same-date transactions changes from `rowid ASC` to `created_at ASC` (SEL-024). The existing index is still usable; the `ORDER BY` clause in queries will be updated to `date ASC, created_at ASC`.

---

### Phase 2 -- Backend

#### 2.1 Domain Layer (`src-tauri/src/context/transaction/domain/transaction.rs`)

**Modify `TransactionType` enum**:

- Add `Sell` variant (currently only `Purchase` exists)

**Modify `Transaction` struct**:

- Add field `realized_pnl: Option<i64>` -- present only for Sell transactions
- Add field `created_at: String` -- ISO 8601 timestamp

**Modify factory methods**:

- `new()` -- add `realized_pnl: Option<i64>` parameter; generate `created_at` with `chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()`
- `with_id()` -- add `realized_pnl: Option<i64>` and `created_at: String` parameters
- `restore()` -- add `realized_pnl: Option<i64>` and `created_at: String` parameters

**Modify `validate()`**:

- For Sell transactions: `total_amount` must be positive (SEL-020), same date/quantity/exchange_rate rules apply
- Validation is type-agnostic (same rules for Purchase and Sell per SEL-020)
- Add `transaction_type` parameter to `validate()` so that `total_amount` for sells uses the sell formula check (or remove formula check since both types compute total on backend)

**SEL-035 -- transaction_type immutability**: No domain-level change needed; enforced in orchestrator at update time.

#### 2.2 Repository Layer (`src-tauri/src/context/transaction/repository/transaction.rs`)

**Modify `TransactionRow` struct**:

- Add `realized_pnl: Option<i64>`
- Add `created_at: String`

**Modify `From<TransactionRow> for Transaction`**:

- Pass `realized_pnl` and `created_at` through to `Transaction::restore()`

**Modify all SQL queries**:

- `get_by_id`: SELECT adds `realized_pnl, created_at`
- `get_by_account_asset`: SELECT adds `realized_pnl, created_at`; change ORDER BY to `date ASC, created_at ASC` (SEL-024)
- `create`: INSERT adds `realized_pnl, created_at`
- `update`: UPDATE adds `realized_pnl` (created_at is immutable after insert)
- `delete`: no change

**Add new method to `TransactionRepository` trait** (in `domain/transaction.rs`):

- `get_realized_pnl_by_account(&self, account_id: &str) -> Result<Vec<(String, i64)>>` -- returns `Vec<(asset_id, sum_realized_pnl)>` for SEL-038

**Implement in `SqliteTransactionRepository`**:

- SQL: `SELECT asset_id, COALESCE(SUM(realized_pnl), 0) as total_pnl FROM transactions WHERE account_id = ? AND transaction_type = 'Sell' GROUP BY asset_id`

#### 2.3 Service Layer (`src-tauri/src/context/transaction/service.rs`)

**Add method to `TransactionService`** (SEL-038):

- `get_realized_pnl_by_account(&self, account_id: &str) -> Result<Vec<(String, i64)>>` -- delegates to repository
- Returns sum of `realized_pnl` grouped by `asset_id` for the given account

#### 2.4 Orchestrator -- Record Transaction (`src-tauri/src/use_cases/record_transaction/orchestrator.rs`)

This is the most complex part. The orchestrator currently only handles `Purchase` transactions.

**Modify `CreateTransactionDTO`**:

- Add `transaction_type: String` field -- "Purchase" or "Sell" (received from frontend)

**Modify `create_transaction()`**:

- Parse `dto.transaction_type` into `TransactionType` enum
- Branch logic based on transaction type:
  - **Purchase**: existing logic (unchanged)
  - **Sell**: new logic per SEL-010 through SEL-028

**Add sell-specific logic in `create_transaction()`** (Sell branch):

- SEL-037: Check `asset.is_archived` -- reject if true
- SEL-012: Check current `Holding.quantity` -- reject if zero (closed position guard)
- SEL-021: Check `dto.quantity <= Holding.quantity` -- reject if oversell
- SEL-023: Compute `total_amount` using sell formula: `floor(floor(qty * price / MICRO) * rate / MICRO) - fees`
- SEL-024: Compute `realized_pnl`:
  - Run full chronological recalculation to get VWAP immediately before this sell
  - `realized_pnl = total_sell_amount - floor(vwap_before_sell * sold_quantity / MICRO)`
- SEL-025: Decrease `Holding.quantity` by sold quantity
- SEL-026: Retain holding at qty=0 (no delete)
- SEL-027: VWAP unchanged by sells
- SEL-028: All within a single DB transaction

**Add helper methods**:

- `compute_sell_total(quantity, unit_price, exchange_rate, fees) -> i64` -- SEL-023 formula (fees subtracted, not added)
- `compute_realized_pnl(total_sell_amount, vwap_before_sell, sold_quantity) -> i64` -- SEL-024
- `compute_holding_state(account_id, asset_id, transactions) -> (i64, i64, Vec<i64>)` -- full chronological recalculation returning `(final_quantity, final_average_price, realized_pnls_per_sell)` for all transactions in order (used by create, edit, delete)

**Refactor `compute_vwap_holding()`** into a more general `recalculate_holding()` that:

- Processes transactions in `date ASC, created_at ASC` order
- For Purchase: accumulates quantity and updates VWAP (existing logic)
- For Sell: decreases quantity, computes `realized_pnl` for each sell, leaves VWAP unchanged (SEL-027)
- Returns: updated `Holding` + a map of `transaction_id -> realized_pnl` for all sell transactions

**Modify `update_transaction()`**:

- SEL-035: Reject if `dto.transaction_type` differs from existing `transaction.transaction_type`
- SEL-030: Same field validation for sells
- SEL-031: Trigger full recalculation on edit
- SEL-032: During recalculation, if any sell in the sequence would exceed the holding quantity at that point, reject with error identifying the offending sell
- Update `realized_pnl` values for all sell transactions in the pair within the same DB transaction

**Modify `delete_transaction()`**:

- SEL-033: Trigger full recalculation of holding and realized_pnl values
- Update all affected sell transactions' `realized_pnl` within the same DB transaction

#### 2.5 Orchestrator -- Account Details (`src-tauri/src/use_cases/account_details/orchestrator.rs`)

**Modify `AccountDetailsUseCase`** (ADR-005):

- Inject `Arc<TransactionService>` alongside existing `AccountService` and `AssetService`
- Constructor: `new(account_service, asset_service, transaction_service)`

**Modify `get_account_details()`**:

- Call `transaction_service.get_realized_pnl_by_account(account_id)` (SEL-038)
- Build a `HashMap<String, i64>` from `asset_id -> realized_pnl`
- Enrich each `HoldingDetail` with `realized_pnl` from the map (default 0 if absent)

**Modify `HoldingDetail` struct**:

- Add field `realized_pnl: i64` (SEL-042)

**Modify `AccountDetailsResponse` struct**:

- Add field `total_realized_pnl: i64` -- sum across all holdings

#### 2.6 App Wiring (`src-tauri/src/lib.rs`)

**Modify `AccountDetailsUseCase::new()` call**:

- Pass `transaction_service.clone()` as third argument

#### 2.7 Specta Builder (`src-tauri/src/core/specta_builder.rs`)

No new commands are added. The existing commands are modified in place. No changes to the builder unless new types are introduced that need explicit `.typ::<>()` registration. The `TransactionType::Sell` variant will be automatically exported.

#### 2.8 Account Details API (`src-tauri/src/use_cases/account_details/api.rs`)

No changes needed -- the command signature is unchanged; only the response shape grows.

---

### Phase 3 -- Type Synchronization

Run `just generate-types` to regenerate `src/bindings.ts` with:

- `TransactionType` enum now includes `"Sell"`
- `Transaction` type now includes `realized_pnl: number | null` and `created_at: string`
- `CreateTransactionDTO` now includes `transaction_type: string`
- `HoldingDetail` now includes `realized_pnl: number`
- `AccountDetailsResponse` now includes `total_realized_pnl: number`

---

### Phase 4 -- Frontend

#### 4.1 Shared Utilities

**Modify `src/lib/microUnits.ts`**:

- Add `computeSellTotalMicro(qtyMicro, priceMicro, rateMicro, feesMicro) -> number` -- SEL-023 formula: `floor(floor(qty * price / MICRO) * rate / MICRO) - fees`

**Modify `src/features/transactions/shared/types.ts`**:

- `TransactionFormData`: no structural change needed (same fields for sell form); the `transactionType` is implicit from context

**Modify `src/features/transactions/shared/presenter.ts`**:

- `TransactionRowViewModel`: add `realizedPnl: string | null` field
- `toTransactionRow()`: map `tx.realized_pnl` to formatted string via `microToDecimal()` (null if Purchase)

**Modify `src/features/transactions/shared/validateTransaction.ts`**:

- Add `validateSellForm()` function or extend existing `validateTransactionForm()` with a `maxQuantityMicro` parameter for the oversell frontend guard (SEL-022)

#### 4.2 Gateway

**Modify `src/features/transactions/gateway.ts`**:

- `addTransaction(dto)`: no signature change (dto now includes `transaction_type`)
- No new gateway methods needed

#### 4.3 Sell Transaction Modal (new sub-feature)

**Create `src/features/transactions/sell_transaction/`**:

**`SellTransactionModal.tsx`** -- New component (SEL-010, SEL-011, SEL-029, SEL-036):

- Props: `isOpen`, `onClose`, `accountId`, `assetId`, `holdingQuantity` (micro-units), `assetCurrency`, `accountCurrency`
- Pre-fills account and asset (read-only) per SEL-011
- Shows max quantity hint per SEL-022
- Date defaults to today, exchange_rate to 1.0, fees to 0 per SEL-029
- Exchange rate field visible only when `assetCurrency !== accountCurrency` per SEL-036
- Displays computed total proceeds read-only (sell formula)
- Inline validation: quantity > holding triggers error, disables Save per SEL-022
- On submit: builds `CreateTransactionDTO` with `transaction_type: "Sell"` and calls gateway
- Success: closes modal, shows snackbar per SEL-045
- Loading/error states per SEL-044

**`useSellTransaction.ts`** -- New hook:

- State: `formData: TransactionFormData`, `error`, `isSubmitting`, `isFormValid`
- `handleChange`: updates form fields; recomputes total via sell formula (SEL-023)
- `handleSubmit`: validates (SEL-022 max quantity check), calls `transactionGateway.addTransaction(dto)` with `transaction_type: "Sell"`
- Returns: `formData`, `totalAmountDisplay`, `error`, `isSubmitting`, `isFormValid`, `handleChange`, `handleSubmit`, `maxQuantityDisplay`

**`useSellTransaction.test.ts`** -- Colocated tests:

- Test: quantity exceeding holding disables form
- Test: sell formula computes correct total (fees subtracted)
- Test: submit builds correct DTO with `transaction_type: "Sell"`

#### 4.4 Account Details View (entry point for Sell)

**Modify `src/features/account_details/account_details_view/AccountDetailsView.tsx`**:

- Import `SellTransactionModal`
- Add "Sell" `IconButton` to each `HoldingRow` (SEL-010) -- visible when `quantity > 0` (always true in active holdings), disabled when asset is archived (SEL-037)
- Track `sellTarget` state: `{ accountId, assetId, holdingQuantity, assetCurrency, accountCurrency } | null`
- Render `SellTransactionModal` when `sellTarget` is set
- Add "Realized P&L" column to holdings table header (SEL-042)
- Display `realizedPnl` in each holding row with color tokens (SEL-043)

**Modify `src/features/account_details/shared/presenter.ts`**:

- `HoldingRowViewModel`: add `realizedPnl: string`, `realizedPnlRaw: number` (for color logic)
- `toHoldingRow()`: map `detail.realized_pnl` via `microToDecimal()`, include raw value for sign-based styling
- `AccountSummaryViewModel`: add `totalRealizedPnl: string`
- `toAccountSummary()`: map `response.total_realized_pnl`

**Modify `src/features/account_details/account_details_view/useAccountDetails.ts`**:

- No structural changes -- the existing `data.holdings.map(toHoldingRow)` automatically picks up the new `realized_pnl` field after the presenter is updated

**Note on asset metadata for sell modal**: The `SellTransactionModal` needs `assetCurrency` and `accountCurrency` (for SEL-036). The `HoldingDetail` from the backend does not currently expose asset currency. Two options:

- Option A: Add `asset_currency` to `HoldingDetail` on the backend
- Option B: Look up the asset from the global store `useAppStore.assets` by `assetId`
- **Decision**: Option B -- the global store already holds all assets including archived ones; this avoids backend changes. The account currency is not in the store but the account_details response does not carry it either. We need to add `account_currency: String` to `AccountDetailsResponse` (requires fetching from `Account` entity in the orchestrator). Alternatively, look up the account from the global store. **Decision**: Look up from global store via `useAppStore.accounts.find(a => a.id === accountId)`. However, `Account` does not currently have a `currency` field. Check: the Account entity has `id`, `name`, `update_frequency` only -- no currency. For the sell form exchange rate visibility, we need to compare asset currency with account currency. Since accounts don't have a currency field, the existing purchase form uses a hardcoded `"EUR"` check (see `AddTransactionModal.tsx` line 56: `selectedAsset.currency !== "EUR"`). **Decision**: Follow the same pattern as the existing purchase form for now; this is a pre-existing simplification, not introduced by the sell feature.

#### 4.5 Transaction List (display sell rows)

**Modify `src/features/transactions/transaction_list/TransactionListPage.tsx`**:

- SEL-040: Sell rows already display via `t(`transaction.type\_${row.type.toLowerCase()}`)` -- just need i18n key `transaction.type_sell`
- SEL-041: Add "Realized P&L" column to the transaction table; show value for Sell rows, dash for Purchase rows
- Style P&L values with gain/loss color tokens (SEL-043)

#### 4.6 Edit Transaction Modal

**Modify `src/features/transactions/edit_transaction_modal/EditTransactionModal.tsx`**:

- Detect `transaction.transaction_type` -- if "Sell", use sell formula for total display and adjust field labels
- The form fields are identical; only the total computation differs
- SEL-035 is backend-enforced; frontend does not allow changing transaction type

**Modify `src/features/transactions/edit_transaction_modal/useEditTransactionModal.ts`**:

- Compute total using sell formula when `transactionType === "Sell"`

#### 4.7 i18n Keys

**Add to `src/i18n/locales/en/common.json`**:

- `transaction.type_sell`: "Sell"
- `transaction.action_sell`: "Sell"
- `transaction.sell_modal_title`: "Sell position"
- `transaction.form_max_quantity_hint`: "Max: {{max}}"
- `transaction.error_validation_oversell`: "Quantity exceeds current holding ({{max}})."
- `transaction.error_closed_position`: "No units available to sell."
- `transaction.error_archived_asset_sell`: "Cannot sell an archived asset."
- `transaction.column_realized_pnl`: "Realized P&L"
- `transaction.success_sell_created`: "Sell transaction recorded."
- `transaction.success_sell_updated`: "Sell transaction updated."
- `transaction.success_sell_deleted`: "Sell transaction deleted."
- `account_details.column_realized_pnl`: "Realized P&L"
- `account_details.total_realized_pnl`: "Total Realized P&L"
- `account_details.pnl_placeholder`: "--"

**Add to `src/i18n/locales/fr/common.json`**: French translations for all the above keys.

---

## 3. Rules Coverage

### SEL-010 -- Sell entry point (frontend)

- `AccountDetailsView.tsx`: Add "Sell" IconButton per holding row
- Visible only when `quantity > 0` (implicit: active holdings only displayed)
- Disabled when asset is archived (SEL-037 frontend guard)

### SEL-011 -- Contextual pre-filling (frontend)

- `SellTransactionModal.tsx`: Account + Asset pre-filled, read-only

### SEL-012 -- Closed position guard (backend)

- `orchestrator.rs` `create_transaction()`: Check `Holding.quantity == 0` before sell

### SEL-020 -- Sell field validation (backend)

- `transaction.rs` `validate()`: Same rules as TRX-020 applied to Sell type
- `orchestrator.rs`: Validate account_id and asset_id exist

### SEL-021 -- Oversell guard (backend)

- `orchestrator.rs`: Check `dto.quantity <= Holding.quantity`

### SEL-022 -- Maximum quantity hint (frontend)

- `SellTransactionModal.tsx`: Display holding quantity as max
- `useSellTransaction.ts`: Inline validation when quantity > max

### SEL-023 -- Sell total amount formula (backend)

- `orchestrator.rs` `compute_sell_total()`: `floor(floor(qty * price / MICRO) * rate / MICRO) - fees`

### SEL-024 -- Realized P&L computation (backend)

- `orchestrator.rs` `recalculate_holding()`: Compute VWAP before each sell, then `realized_pnl = total_sell - floor(vwap * qty / MICRO)`
- `transaction.rs`: `realized_pnl: Option<i64>` field
- Migration: `created_at` column for tie-breaking

### SEL-025 -- Holding quantity decrease (backend)

- `orchestrator.rs`: Sell decreases `Holding.quantity` in recalculation

### SEL-026 -- Zero quantity retention (backend)

- `orchestrator.rs`: After sell, if `quantity == 0`, retain holding (no delete)

### SEL-027 -- VWAP unchanged by sells (backend)

- `orchestrator.rs` `recalculate_holding()`: Skip sells in VWAP accumulator

### SEL-028 -- Atomicity (backend)

- `orchestrator.rs`: All writes within `pool.begin()` / `commit()`

### SEL-029 -- Sell form defaults (frontend)

- `useSellTransaction.ts`: Date=today, exchange_rate=1.0, fees=0

### SEL-030 -- Edit sell validation (backend)

- `orchestrator.rs` `update_transaction()`: Same validation for sells; oversell re-evaluated in recalculation

### SEL-031 -- Full recalculation on edit (backend)

- `orchestrator.rs` `update_transaction()`: Calls `recalculate_holding()` for all transactions in pair

### SEL-032 -- Cascading oversell on purchase edit (backend)

- `orchestrator.rs` `recalculate_holding()`: During recalculation, if a sell exceeds the running quantity, reject with error

### SEL-033 -- Delete sell transaction (backend)

- `orchestrator.rs` `delete_transaction()`: Triggers full recalculation

### SEL-034 -- Delete confirmation (frontend)

- Already implemented via `ConfirmationDialog` in `TransactionListPage.tsx` (shared with TRX-035)

### SEL-035 -- Transaction type immutability (backend)

- `orchestrator.rs` `update_transaction()`: Reject if `dto.transaction_type != existing.transaction_type`

### SEL-036 -- Exchange rate field visibility (frontend)

- `SellTransactionModal.tsx`: Show exchange rate only when asset currency differs from account currency

### SEL-037 -- Archived asset sell guard (backend + frontend)

- `orchestrator.rs`: Reject sell if `asset.is_archived`
- `AccountDetailsView.tsx`: Disable Sell button when asset is archived

### SEL-038 -- Realized P&L aggregation service (backend)

- `TransactionService.get_realized_pnl_by_account()` method
- `TransactionRepository.get_realized_pnl_by_account()` trait method + SQLite impl

### SEL-040 -- Sell type indicator (frontend)

- `TransactionListPage.tsx`: i18n key `transaction.type_sell` renders "Sell" label

### SEL-041 -- Realized P&L per sell row (frontend)

- `TransactionListPage.tsx`: Add P&L column, show `realizedPnl` for Sell rows
- `presenter.ts`: Map `realized_pnl` in `TransactionRowViewModel`

### SEL-042 -- Cumulative realized P&L in Account Details (frontend + backend)

- Backend: `AccountDetailsUseCase` fetches via `TransactionService`, enriches `HoldingDetail`
- Frontend: `AccountDetailsView.tsx` adds "Realized P&L" column

### SEL-043 -- P&L visual differentiation (frontend)

- `AccountDetailsView.tsx` + `TransactionListPage.tsx`: success color for positive, error color for negative, neutral placeholder for zero

### SEL-044 -- Loading and error states (frontend)

- `SellTransactionModal.tsx`: Loading/error patterns consistent with rest of app

### SEL-045 -- Success feedback (frontend)

- `SellTransactionModal.tsx`: Close modal + snackbar on success
- Account details refreshes via `TransactionUpdated` event (existing mechanism)

---

## 4. Dependency Graph

```
Migration (Phase 1)
  |
  v
just migrate + just prepare-sqlx
  |
  v
Backend Domain (2.1) --> Backend Repository (2.2) --> Backend Service (2.3)
  |                                                        |
  v                                                        v
Backend Orchestrator record_transaction (2.4)     Backend Orchestrator account_details (2.5)
  |                                                        |
  v                                                        v
App Wiring (2.6)                                  Specta Builder (2.7 -- verify)
  |
  v
just generate-types (Phase 3)
  |
  v
Frontend Shared Utils (4.1) --> Frontend Gateway (4.2)
  |                                    |
  v                                    v
Sell Modal (4.3)              Account Details View (4.4)
  |                                    |
  v                                    v
Transaction List (4.5)        Edit Modal (4.6)
  |
  v
i18n (4.7)
```

---

## 5. Commit Phases

### Commit 1 -- Backend

`feat(sell): implement sell transaction backend with realized P&L`

Includes: migration, domain changes, repository changes, service changes, both orchestrators, app wiring.

### Commit 2 -- Frontend

`feat(sell): implement sell transaction frontend with P&L display`

Includes: gateway, sell modal, account details view changes, transaction list changes, edit modal changes, i18n keys, shared utils.

### Commit 3 -- Tests & Docs

`test(sell): add sell transaction tests and update docs`

Includes: backend tests (domain validation for Sell, recalculation logic, P&L computation, oversell guard, cascading oversell), frontend tests (useSellTransaction hook), ARCHITECTURE.md update, spec-index status change to "active".
