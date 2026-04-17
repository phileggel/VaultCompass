# Implementation Plan -- Account Details (ACD)

Spec: `docs/spec/account-details.md`
ADRs: ADR-003 (sequential service calls), ADR-004 (inject services not repos)

---

## 1. Workflow TaskList

- [ ] Backend Implementation (Domain DTOs, AccountService method, Use Case, API)
- [ ] Type Synchronization (`just generate-types`)
- [ ] Frontend Implementation (Gateway, Hook, Components, Presenter, i18n)
- [ ] Formatting and Linting (`just format` + `python3 scripts/check.py`)
- [ ] Code Review (`reviewer` + `reviewer-backend` + `reviewer-frontend`)
- [ ] i18n Review (`i18n-checker`)
- [ ] Unit and Integration Tests
- [ ] Documentation Update (`ARCHITECTURE.md` + `docs/todo.md`)
- [ ] Final Validation (`spec-checker` + `workflow-validator`)

---

## 2. Detailed Implementation Plan

### 2.1 Backend -- AccountService: add `get_holdings_for_account`

**Context**: `AccountService` already has `get_by_id` (returns `Option<Account>`). It does NOT expose holding reads. Per ADR-004, the use case must call `AccountService` (not `HoldingRepository` directly), so a pass-through method is needed.

**File**: `src-tauri/src/context/account/service.rs`

- Add field `holding_repo: Box<dyn HoldingRepository>` to `AccountService` struct
- Update `AccountService::new()` to accept `holding_repo: Box<dyn HoldingRepository>`
- Add method `get_holdings_for_account(&self, account_id: &str) -> Result<Vec<Holding>>`
  - Delegates to `self.holding_repo.get_by_account(account_id)`
- Import `Holding` and `HoldingRepository` from `super::domain`

**File**: `src-tauri/src/context/account/mod.rs` -- no change needed (already re-exports all from domain)

**File**: `src-tauri/src/lib.rs`

- Update `AccountService::new()` call to also pass `Box::new(SqliteHoldingRepository::new(db.pool.clone()))` as the holding repo
- Note: a `SqliteHoldingRepository` is already instantiated for the `RecordTransactionUseCase`; create a second instance for `AccountService` (they share the pool, not the instance)

**Rules covered**: ADR-004 (service wraps repo access)

---

### 2.2 Backend -- DTOs: `HoldingDetail` and `AccountDetailsResponse`

**File**: `src-tauri/src/use_cases/account_details/orchestrator.rs` (new file)

Define two Specta-exported structs:

```
HoldingDetail {
    asset_id: String,
    asset_name: String,
    asset_reference: String,
    quantity: i64,
    average_price: i64,
    cost_basis: i64,
}
```

- Derive: `Debug, Serialize, Clone, Type`

```
AccountDetailsResponse {
    account_name: String,
    holdings: Vec<HoldingDetail>,
    total_holding_count: usize,
    total_cost_basis: i64,
}
```

- Derive: `Debug, Serialize, Clone, Type`

**Rules covered**: ACD-041 (i64 micro-units), spec entity definitions

---

### 2.3 Backend -- Use Case: `AccountDetailsUseCase`

**Directory**: `src-tauri/src/use_cases/account_details/` (new)

- `mod.rs` -- module declaration, re-exports
- `orchestrator.rs` -- `AccountDetailsUseCase` struct + DTOs
- `api.rs` -- Tauri command handler

**File**: `src-tauri/src/use_cases/account_details/orchestrator.rs`

`AccountDetailsUseCase` struct:
- Fields: `account_service: Arc<AccountService>`, `asset_service: Arc<AssetService>`
- Constructor: `new(account_service: Arc<AccountService>, asset_service: Arc<AssetService>) -> Self`

Method `get_account_details(&self, account_id: &str) -> anyhow::Result<AccountDetailsResponse>`:
1. Call `self.account_service.get_by_id(account_id)` -- if `None`, bail with not-found error (ACD-012)
2. Call `self.account_service.get_holdings_for_account(account_id)` -- returns `Vec<Holding>`
3. Store `total_holding_count = all_holdings.len()` (ACD-034 distinction)
4. Filter to `active_holdings` where `quantity > 0` (ACD-020)
5. For each active holding, call `self.asset_service.get_asset_by_id(&holding.asset_id)` to get `asset_name` and `asset_reference` (ACD-022, ACD-021 -- archived assets included because `get_asset_by_id` returns regardless of archive status)
6. Compute `cost_basis` per holding: `(holding.quantity as i128 * holding.average_price as i128 / 1_000_000) as i64` using i128 intermediates (ACD-023, ACD-024)
7. Build `Vec<HoldingDetail>`, sort alphabetically by `asset_name` ascending (ACD-033)
8. Compute `total_cost_basis`: sum of all `cost_basis` values, 0 if empty (ACD-031)
9. Return `AccountDetailsResponse { account_name, holdings, total_holding_count, total_cost_basis }` (ACD-032)

**Rules covered**: ACD-012, ACD-020, ACD-021, ACD-022, ACD-023, ACD-024, ACD-031, ACD-032, ACD-033, ACD-034, ACD-041

---

### 2.4 Backend -- API: Tauri command

**File**: `src-tauri/src/use_cases/account_details/api.rs` (new)

```rust
#[tauri::command]
#[specta::specta]
pub async fn get_account_details(
    state: State<'_, AccountDetailsUseCase>,
    account_id: String,
) -> Result<AccountDetailsResponse, String>
```

- Delegates to `state.get_account_details(&account_id).await.map_err(|e| e.to_string())`
- Add `#![allow(clippy::unreachable)]` at module top (same pattern as `record_transaction/api.rs`)

**File**: `src-tauri/src/use_cases/account_details/mod.rs` (new)

```rust
mod api;
mod orchestrator;

pub use api::*;
pub use orchestrator::*;
```

**File**: `src-tauri/src/use_cases/mod.rs`

- Add: `pub mod account_details;`

---

### 2.5 Backend -- Specta registration and state management

**File**: `src-tauri/src/core/specta_builder.rs`

- Add import: `use crate::use_cases::account_details;`
- Add types: `.typ::<account_details::HoldingDetail>()` and `.typ::<account_details::AccountDetailsResponse>()`
- Add command: `account_details::get_account_details` to the `collect_commands![]` macro

**File**: `src-tauri/src/lib.rs`

- Import `AccountDetailsUseCase` from `use_cases::account_details`
- Wrap `AccountService` and `AssetService` in `Arc` so they can be shared between `AppState` and `AccountDetailsUseCase`
- After creating `account_service` and `asset_service`, create: `let account_details_uc = AccountDetailsUseCase::new(Arc::clone(&account_service), Arc::clone(&asset_service));`
- Register: `app_handle.manage(account_details_uc);`
- Update `AppState` fields to use `Arc<AccountService>` and `Arc<AssetService>` -- update all existing api.rs handlers that access these via `AppState` to dereference the Arc

**Rules covered**: B0c (specta_builder only), B9 (use case owns its API)

---

### 2.6 Type synchronization

Run `just generate-types` after backend compiles. This regenerates `src/bindings.ts` with:
- `HoldingDetail` type
- `AccountDetailsResponse` type
- `commands.getAccountDetails(accountId: string)` function

---

### 2.7 Frontend -- Gateway

**File**: `src/features/account_details/gateway.ts` (new)

```typescript
export const accountDetailsGateway = {
  async getAccountDetails(accountId: string): Promise<Result<AccountDetailsResponse, string>> {
    return await commands.getAccountDetails(accountId);
  },
};
```

- Import `commands`, `Result`, `AccountDetailsResponse` from `@/bindings`

---

### 2.8 Frontend -- Presenter

**File**: `src/features/account_details/shared/presenter.ts` (new)

Functions:
- `formatMicroAmount(micros: number, decimals?: number): string` -- converts i64 micro-units to display string (reuse logic from `microToDecimal` in transactions, or import it)
- `toHoldingRow(detail: HoldingDetail): HoldingRowViewModel` -- maps `HoldingDetail` to display-ready object with formatted quantity, average_price, cost_basis
- `toAccountSummary(response: AccountDetailsResponse): AccountSummaryViewModel` -- maps response to header display data

Types:
- `HoldingRowViewModel { assetId, assetName, assetReference, quantity, averagePrice, costBasis }` (all display strings)
- `AccountSummaryViewModel { accountName, totalCostBasis, holdingCount, isEmpty, isAllClosed }`
  - `isEmpty`: `totalHoldingCount === 0`
  - `isAllClosed`: `totalHoldingCount > 0 && holdings.length === 0`

**Rules covered**: ACD-034 (empty state distinction), F5 (presenter pattern)

---

### 2.9 Frontend -- Hook: `useAccountDetails`

**File**: `src/features/account_details/account_details_view/useAccountDetails.ts` (new)

State:
- `data: AccountDetailsResponse | null`
- `isLoading: boolean`
- `error: string | null`

Logic:
- `fetchDetails(accountId)`: calls `accountDetailsGateway.getAccountDetails(accountId)`, sets state
- On mount (when `accountId` changes): fetch details (ACD-037 loading state)
- Listen to `TransactionUpdated` event: re-fetch (ACD-039)
- Listen to `AssetUpdated` event: re-fetch (ACD-040)
- Event listeners via `events.event.listen()` with cleanup in `useEffect` return (F9)
- `retry()` callback: re-fetches (ACD-038)
- Return: `{ data, isLoading, error, retry, holdings: HoldingRowViewModel[], summary: AccountSummaryViewModel }`
  - `holdings` and `summary` derived via `useMemo` through presenter functions

**Rules covered**: ACD-037, ACD-038, ACD-039, ACD-040, F9, F10, F13

---

### 2.10 Frontend -- Component: `AccountDetailsView`

**File**: `src/features/account_details/account_details_view/AccountDetailsView.tsx` (new)

Props: `{ accountId: string, onBack: () => void }`

Structure:
- Uses `useAccountDetails(accountId)`
- **Header**: Account name + formatted total cost basis (ACD-032, ACD-031)
  - Back button to return to account list
- **Holdings Table** (ACD-033 order is already handled by backend):
  - Columns: Asset (name + reference), Quantity, Avg. Price, Cost Basis
  - Each row from `holdings` view model
- **States**:
  - Loading: skeleton screens for header and table rows (ACD-037)
  - Empty (no positions): "No positions yet" message + "Add Transaction" button (ACD-034, ACD-035)
  - Empty (all closed): "All positions are closed" message + "Add Transaction" button (ACD-034, ACD-035)
  - Error: generic error message + "Retry" button (ACD-038)
  - Non-empty: table + FAB "Add Transaction" button (ACD-036)
- **Add Transaction CTA** (ACD-035, ACD-036): opens `AddTransactionModal` with pre-filled `accountId` (per TRX-011 contract)
- Log on mount: `logger.info("[AccountDetailsView] mounted")` (F13)

**Rules covered**: ACD-031, ACD-032, ACD-034, ACD-035, ACD-036, ACD-037, ACD-038, F11, F13, F16

---

### 2.11 Frontend -- Navigation: Account selection in `AccountManager`

**File**: `src/features/accounts/AccountManager.tsx`

- Add state: `const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null)`
- When `selectedAccountId` is set, render `AccountDetailsView` instead of the account list (ACD-011)
- Pass `onBack={() => setSelectedAccountId(null)}` to `AccountDetailsView`

**File**: `src/features/accounts/account_table/AccountTable.tsx`

- Add `onAccountClick: (accountId: string) => void` prop
- Make account name cell (or entire row excluding action buttons) clickable: `onClick={() => onAccountClick(account.id)}` (ACD-010)
- Add cursor-pointer styling to clickable rows
- Pass `onAccountClick={setSelectedAccountId}` from `AccountManager`

**Rules covered**: ACD-010, ACD-011

---

### 2.12 Frontend -- i18n

**Files**: `src/i18n/locales/fr/common.json` and `src/i18n/locales/en/common.json`

New keys under `"account_details"` namespace:

| Key | EN | FR |
|-----|----|----|
| `account_details.title` | Account Details | Details du compte |
| `account_details.column_asset` | Asset | Actif |
| `account_details.column_quantity` | Quantity | Quantite |
| `account_details.column_avg_price` | Avg. Price | Prix moyen |
| `account_details.column_cost_basis` | Cost Basis | Cout de revient |
| `account_details.total_cost_basis` | Total Cost Basis | Cout de revient total |
| `account_details.empty_no_positions` | No positions yet | Aucune position |
| `account_details.empty_all_closed` | All positions are closed | Toutes les positions sont cloturees |
| `account_details.add_transaction` | Add Transaction | Ajouter une transaction |
| `account_details.error_load` | Failed to load account details | Erreur lors du chargement des details |
| `account_details.back` | Back to accounts | Retour aux comptes |
| `account_details.loading` | Loading... | Chargement... |

**Rules covered**: F16, ACD-034, ACD-035, ACD-036

---

### 2.13 Frontend -- Feature module structure

Final directory layout:

```
src/features/account_details/
  gateway.ts
  account_details_view/
    AccountDetailsView.tsx
    useAccountDetails.ts
    useAccountDetails.test.ts
  shared/
    presenter.ts
    presenter.test.ts
  index.ts
```

`index.ts` exports: `AccountDetailsView`, `accountDetailsGateway`

---

## 3. Rules Coverage Matrix

### Backend rules

| Rule | Description | Implementation location |
|------|-------------|----------------------|
| ACD-012 | Invalid account guard | `orchestrator.rs` -- `get_by_id` returns None, bail with not-found |
| ACD-020 | Active holding filter (qty > 0) | `orchestrator.rs` -- filter step |
| ACD-021 | Archived asset inclusion | `orchestrator.rs` -- `get_asset_by_id` returns regardless of archive status |
| ACD-022 | Asset metadata enrichment | `orchestrator.rs` -- calls `AssetService::get_asset_by_id` per holding |
| ACD-023 | Cost basis calculation | `orchestrator.rs` -- `qty * avg_price / MICRO` |
| ACD-024 | i128 intermediate precision | `orchestrator.rs` -- cast to i128 before multiply |
| ACD-031 | Total account cost basis | `orchestrator.rs` -- sum of cost_basis, 0 if empty |
| ACD-032 | Account name in response | `orchestrator.rs` -- fetched via `AccountService::get_by_id` |
| ACD-033 | Holdings sort by asset_name | `orchestrator.rs` -- sort after enrichment |
| ACD-041 | i64 micro-unit serialization | DTO definitions -- all financial fields are i64 |

### Frontend rules

| Rule | Description | Implementation location |
|------|-------------|----------------------|
| ACD-010 | View entry point (row click) | `AccountTable.tsx` -- clickable row |
| ACD-011 | Account selection persistence | `AccountManager.tsx` -- `selectedAccountId` state |
| ACD-034 | Empty account state | `AccountDetailsView.tsx` -- conditional rendering |
| ACD-035 | Empty state CTA | `AccountDetailsView.tsx` -- "Add Transaction" button |
| ACD-036 | Non-empty state CTA | `AccountDetailsView.tsx` -- FAB button |
| ACD-037 | Loading state | `AccountDetailsView.tsx` -- skeleton screens |
| ACD-038 | Error state with retry | `AccountDetailsView.tsx` -- error + retry button |
| ACD-039 | Reactivity to TransactionUpdated | `useAccountDetails.ts` -- event listener re-fetch |
| ACD-040 | Reactivity to AssetUpdated | `useAccountDetails.ts` -- event listener re-fetch |

---

## 4. Dependency Graph

```
1. AccountService: add holding_repo field + get_holdings_for_account method
   |
2. lib.rs: wrap AccountService/AssetService in Arc, pass holding_repo
   |
3. DTOs + AccountDetailsUseCase (orchestrator.rs)
   |
4. API command (api.rs) + use_cases/mod.rs registration
   |
5. specta_builder.rs: register types + command
   |
6. lib.rs: instantiate and manage AccountDetailsUseCase
   |
7. just generate-types  <-- synchronization barrier
   |
8. Frontend gateway.ts
   |
9. Frontend presenter.ts
   |
10. Frontend useAccountDetails.ts hook
    |
11. Frontend AccountDetailsView.tsx component
    |
12. Frontend AccountManager.tsx + AccountTable.tsx (navigation wiring)
    |
13. i18n keys (fr + en)
    |
14. Tests (backend: orchestrator logic; frontend: hook + presenter)
```

---

## 5. Key Technical Notes

### Arc wrapping of services

Currently `AppState` holds `AssetService` and `AccountService` by value. To share them with `AccountDetailsUseCase`, they must be wrapped in `Arc`. This means:
- `AppState.asset_service: Arc<AssetService>`
- `AppState.account_service: Arc<AccountService>`
- All existing `api.rs` handlers in `context/asset/` and `context/account/` that access these fields will need to work through the Arc (which is transparent for `&self` method calls)

### No new database migration

All data already exists in the `holdings` and `accounts` tables. No schema change needed.

### Event-driven reactivity

The hook listens to `TransactionUpdated` and `AssetUpdated` events directly (not through the global store). This is because account details data is view-scoped, not global state. The global store's `TransactionUpdated` handler (`refreshHoldings` stub in `useTransactionStore`) remains separate per ACD-039's note about scope separation.
