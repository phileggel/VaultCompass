# Implementation Plan -- Transaction List (TXL)

Spec: `docs/spec/transaction-list.md`
ADRs: ADR-001 (i64 micro-units for monetary amounts), ADR-004 (use cases inject services not repos)

---

## 1. Workflow TaskList

- [ ] Backend Implementation (new repository method, service method, API command for TXL-013)
- [ ] Type Synchronization (`just generate-types`)
- [ ] Frontend Implementation (Gateway, Route, Hook, Components, Presenter, i18n)
- [ ] Formatting and Linting (`just format` + `python3 scripts/check.py`)
- [ ] Code Review (`reviewer` + `reviewer-backend` + `reviewer-frontend`)
- [ ] i18n Review (`i18n-checker`)
- [ ] Unit and Integration Tests
- [ ] Documentation Update (`ARCHITECTURE.md` + `docs/todo.md`)
- [ ] Final Validation (`spec-checker` + `workflow-validator`)

---

## 2. Detailed Implementation Plan

### 2.1 Backend -- TXL-013: New repository method `get_asset_ids_for_account`

The new command queries the `transactions` table only and lives within a single bounded context. Decision: **place the command in `context/transaction/`** (B5 pattern), not in `use_cases/record_transaction/`. Reasoning:

- The query reads exclusively from the `transactions` table -- no cross-context orchestration is needed.
- `record_transaction` is a write-oriented use case that orchestrates `transaction`, `account`, and `asset` contexts for mutations. A pure read on a single table does not belong there.
- B9 ("use cases MUST declare their commands in api.rs") applies to cross-context orchestration. B5 ("bounded context MUST declare its Tauri commands in api.rs") is the correct rule for single-context reads.
- The existing `context/transaction/api.rs` is currently a stub comment; this gives it a real purpose.

#### 2.1.1 Domain trait extension

**File**: `src-tauri/src/context/transaction/domain/transaction.rs`

- Add method to `TransactionRepository` trait:
  ```
  async fn get_asset_ids_for_account(&self, account_id: &str) -> Result<Vec<String>>;
  ```
- Returns distinct `asset_id` values from `transactions` where `account_id` matches.
- Returns empty `Vec` if no transactions exist (TXL-013: no error on unknown/empty account).

#### 2.1.2 Repository implementation

**File**: `src-tauri/src/context/transaction/repository/transaction.rs`

- Add `get_asset_ids_for_account` implementation to `SqliteTransactionRepository`:
  ```sql
  SELECT DISTINCT asset_id FROM transactions WHERE account_id = ? ORDER BY asset_id
  ```
- Map rows to `Vec<String>`.

#### 2.1.3 Service method

**File**: `src-tauri/src/context/transaction/service.rs`

- Add method to `TransactionService`:
  ```
  pub async fn get_asset_ids_for_account(&self, account_id: &str) -> Result<Vec<String>>
  ```
- Delegates to `self.repo.get_asset_ids_for_account(account_id)`.

#### 2.1.4 API command

**File**: `src-tauri/src/context/transaction/api.rs`

- Replace the stub comment with a real Tauri command:
  ```
  #[tauri::command]
  #[specta::specta]
  pub async fn get_asset_ids_for_account(
      state: State<'_, TransactionService>,
      account_id: String,
  ) -> Result<Vec<String>, String>
  ```
- Calls `state.get_asset_ids_for_account(&account_id)`.

**File**: `src-tauri/src/context/transaction/mod.rs`

- Ensure `api::get_asset_ids_for_account` is re-exported publicly (add `pub use api::*;` or explicit re-export).

#### 2.1.5 Command registration

**File**: `src-tauri/src/core/specta_builder.rs`

- Add `transaction::get_asset_ids_for_account` to the `collect_commands![]` macro.

#### 2.1.6 Tauri state injection

**File**: `src-tauri/src/lib.rs`

- Verify that `TransactionService` is already managed as Tauri state. It is (per ARCHITECTURE.md line 28: "Bounded context services: AssetService, AccountService, TransactionService"). No change expected, but verify that the State extractor type matches.

**Rules covered**: TXL-013, B5, B11, B13

---

### 2.2 Type Synchronization

Run `just generate-types` to regenerate `src/bindings.ts` with the new `getAssetIdsForAccount` command.

---

### 2.3 Frontend -- Route Registration

#### 2.3.1 New route

**File**: `src/router.tsx`

- Add a new route:
  ```
  const transactionListRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/accounts/$accountId/transactions/$assetId",
    component: TransactionListPage,
  });
  ```
- Import `TransactionListPage` from `@/features/transactions` (via index.ts re-export).
- Add `transactionListRoute` to the `routeTree` children array.

**Rules covered**: TXL-010, TXL-011, F22

---

### 2.4 Frontend -- Gateway Extension

**File**: `src/features/transactions/gateway.ts`

- Add method to `transactionGateway`:
  ```
  async getAssetIdsForAccount(accountId: string): Promise<Result<string[], string>> {
    return await commands.getAssetIdsForAccount(accountId);
  }
  ```

**Rules covered**: TXL-013, TXL-014, F3

---

### 2.5 Frontend -- Presenter Extension

**File**: `src/features/transactions/shared/presenter.ts`

- The existing `toTransactionRow()` function already produces a `TransactionRowViewModel` with all fields needed by TXL-022/TXL-025 (date, quantity, unitPrice, exchangeRate, fees, totalAmount -- all formatted to 3 decimal places via `microToDecimal`).
- Add a `type` field to `TransactionRowViewModel`:
  ```
  type: string;  // "Purchase" for now (TXL-023)
  ```
- Update `toTransactionRow()` to set `type` from `tx.transaction_type` (the Specta-generated string value).

**Rules covered**: TXL-022, TXL-023, TXL-025, F5

---

### 2.6 Frontend -- Transaction List Hook

**File**: `src/features/transactions/transaction_list/useTransactionList.ts` (new)

State managed by the hook:

- `selectedAccountId: string` -- initialized from route param `accountId`
- `selectedAssetId: string | null` -- initialized from route param `assetId`
- `assetIdsForAccount: string[]` -- fetched via `getAssetIdsForAccount`
- `transactions: Transaction[]` -- fetched via `getTransactions`
- `isLoadingAssets: boolean`, `isLoadingTransactions: boolean`
- `assetListError: string | null`, `transactionError: string | null`
- `sortDirection: "asc" | "desc"` -- default `"desc"` (TXL-024), reset on filter change (TXL-016)

Key behaviors:

- **On mount**: read `accountId` and `assetId` from route params (TXL-011); fetch asset IDs for account (TXL-013); fetch transactions (TXL-021).
- **On account change** (TXL-012): reset `selectedAssetId` to `null`, clear transactions, re-fetch asset IDs, reset sort direction (TXL-016).
- **On asset change** (TXL-014): re-fetch transactions, reset sort direction (TXL-016).
- **toggleSortDirection**: flip `sortDirection` (TXL-024).
- **sortedTransactions**: `useMemo` -- sort the `TransactionRowViewModel[]` by date using current `sortDirection`.
- **refreshTransactions**: re-fetch transactions for current `(accountId, assetId)` pair, preserve sort direction (TXL-026).
- **handleDeleteSuccess**: call `refreshTransactions`; if result is empty, navigate to `/accounts/$selectedAccountId` (TXL-043).
- **handleEditSuccess**: call `refreshTransactions` (TXL-031).
- **retryAssetList**: re-fetch asset IDs (TXL-054).
- **retryTransactions**: re-fetch transactions (TXL-053).

Uses `useAppStore` to read `accounts` (for account dropdown, TXL-012) and `assets` (to resolve asset names from IDs, TXL-014).

**Rules covered**: TXL-011, TXL-012, TXL-014, TXL-016, TXL-021, TXL-024, TXL-026, TXL-031, TXL-042, TXL-043, TXL-050, TXL-051, TXL-052, TXL-053, TXL-054, F10

---

### 2.7 Frontend -- Transaction List Page Component

**File**: `src/features/transactions/transaction_list/TransactionListPage.tsx` (new)

Structure (uses `ManagerLayout` wrapper):

#### Header section
- Back link: navigates to `/accounts/$selectedAccountId` (TXL-015) -- uses current dropdown value, not route param.
- Account `SelectField` dropdown: lists all accounts from `useAppStore`, pre-selected from route (TXL-011, TXL-012).
- Asset `SelectField` dropdown: lists only assets from `assetIdsForAccount`, resolved to names via global asset list (TXL-014). Pre-selected from route (TXL-011).

#### Content section -- conditional rendering

1. **Asset list loading** (`isLoadingAssets`): skeleton in asset dropdown area.
2. **Asset list error** (`assetListError`): error message + retry button in asset dropdown (TXL-054).
3. **Incomplete filter** (`selectedAssetId === null`): prompt "Select an asset to view transactions" (TXL-052).
4. **Transaction loading** (`isLoadingTransactions`): skeleton rows in table area (TXL-050).
5. **Transaction error** (`transactionError`): error message + retry button (TXL-053).
6. **Empty state** (transactions.length === 0): "No transactions" message + "Add Transaction" shortcut opening `AddTransactionModal` with `prefillAccountId` and `prefillAssetId` (TXL-051).
7. **Table**: renders columns per TXL-022. Date column header is clickable with `SortIcon` (TXL-024).

#### Row actions
- Edit `IconButton`: opens `EditTransactionModal` with the selected `Transaction` (TXL-030).
- Delete `IconButton`: opens `ConfirmationDialog` (TXL-040, TXL-041).

#### Modals and dialogs managed via local state:
- `editingTransaction: Transaction | null` -- controls `EditTransactionModal` open state.
- `deletingTransactionId: string | null` -- controls `ConfirmationDialog` open state.
- `isAddModalOpen: boolean` -- controls `AddTransactionModal` for empty-state CTA.

#### Delete flow (TXL-040 through TXL-044):
- On confirm: call `deleteTransaction(id)` via `useTransactions` hook.
- On success: close dialog, show success snackbar (`transaction.success_deleted`), call `refreshTransactions` (TXL-042). If result is empty, navigate back (TXL-043).
- On failure: close dialog, show error snackbar (TXL-044).

#### Edit flow (TXL-030, TXL-031):
- `EditTransactionModal` `onClose` callback: call `refreshTransactions` (TXL-031).

**Rules covered**: TXL-010, TXL-015, TXL-022, TXL-023, TXL-024, TXL-030, TXL-031, TXL-040, TXL-041, TXL-042, TXL-043, TXL-044, TXL-050, TXL-051, TXL-052, TXL-053, TXL-054, F6, F7, F8, F11, F13, F16

---

### 2.8 Frontend -- Account Details Inspect Action (ACD-042 / TXL-010)

**File**: `src/features/account_details/account_details_view/AccountDetailsView.tsx`

- Add an inspect action (magnifier `IconButton`) to each holding row in the holdings table.
- On click: `navigate({ to: "/accounts/$accountId/transactions/$assetId", params: { accountId, assetId: row.assetId } })`.
- Import `Search` or `Eye` icon from `lucide-react`.

No cross-feature imports -- navigation is through the router (F22).

**Rules covered**: TXL-010, ACD-042, F22

---

### 2.9 Frontend -- Index Re-export

**File**: `src/features/transactions/index.ts`

- Add re-export: `export { TransactionListPage } from "./transaction_list/TransactionListPage";`

---

### 2.10 Frontend -- i18n Keys

**Files**: `src/i18n/locales/en/common.json`, `src/i18n/locales/fr/common.json`

New keys under the `"transaction"` namespace:

| Key | EN | FR |
|-----|----|----|
| `transaction.list_title` | Transactions | Transactions |
| `transaction.column_type` | Type | Type |
| `transaction.column_date` | Date | Date |
| `transaction.column_quantity` | Quantity | Quantite |
| `transaction.column_unit_price` | Unit Price | Prix unitaire |
| `transaction.column_exchange_rate` | Exchange Rate | Taux de change |
| `transaction.column_fees` | Fees | Frais |
| `transaction.column_total_amount` | Total Amount | Montant total |
| `transaction.column_actions` | Actions | Actions |
| `transaction.type_purchase` | Purchase | Achat |
| `transaction.no_transactions` | No transactions recorded for this position. | Aucune transaction enregistree pour cette position. |
| `transaction.select_asset_prompt` | Select an asset to view transactions. | Selectionnez un actif pour voir les transactions. |
| `transaction.back_to_account` | Back to account | Retour au compte |
| `transaction.error_load_assets` | Failed to load asset list. | Erreur lors du chargement de la liste des actifs. |

Existing keys reused (no changes needed):
- `transaction.delete_confirm_title`, `transaction.delete_confirm_message` (TXL-041)
- `transaction.success_deleted` (TXL-042)
- `transaction.error_load` (TXL-053)
- `action.retry` (TXL-053, TXL-054)

**Rules covered**: TXL-022, TXL-023, TXL-041, TXL-051, TXL-052, F16

---

## 3. Rules Coverage Matrix

| Rule | Layer | Task | Section |
|------|-------|------|---------|
| TXL-010 | Frontend | Inspect action in AccountDetailsView + route navigation | 2.3, 2.8 |
| TXL-011 | Frontend | Route params as initial filter state in useTransactionList | 2.6 |
| TXL-012 | Frontend | Account dropdown in TransactionListPage, reset asset on change | 2.6, 2.7 |
| TXL-013 | Backend | `get_asset_ids_for_account` command in transaction context | 2.1 |
| TXL-014 | Frontend | Asset dropdown filtered by TXL-013 results, name resolution | 2.4, 2.6, 2.7 |
| TXL-015 | Frontend | Back link using current dropdown accountId | 2.7 |
| TXL-016 | Frontend | Sort direction reset on filter change, preserved on refresh | 2.6 |
| TXL-020 | Backend | Existing `get_transactions` command (no change) | -- |
| TXL-021 | Frontend | Transaction fetch on mount and on filter change | 2.6 |
| TXL-022 | Frontend | Table columns in TransactionListPage | 2.7, 2.10 |
| TXL-023 | Frontend | Type column shows "Purchase" via presenter | 2.5 |
| TXL-024 | Frontend | Default desc sort + toggle via Date header | 2.6, 2.7 |
| TXL-025 | Frontend | Financial values formatted to 3 decimals via presenter | 2.5 |
| TXL-026 | Frontend | Re-fetch after edit/delete, sort preserved | 2.6 |
| TXL-030 | Frontend | Edit action opens EditTransactionModal | 2.7 |
| TXL-031 | Frontend | Edit success triggers refresh | 2.6, 2.7 |
| TXL-040 | Frontend | Delete action opens ConfirmationDialog | 2.7 |
| TXL-041 | Frontend | Confirmation dialog uses existing i18n keys | 2.7, 2.10 |
| TXL-042 | Frontend | Delete success: close dialog, snackbar, refresh | 2.7 |
| TXL-043 | Frontend | Navigate back if 0 records after delete | 2.6 |
| TXL-044 | Frontend | Delete failure: close dialog, snackbar error | 2.7 |
| TXL-050 | Frontend | Loading state with skeleton rows | 2.7 |
| TXL-051 | Frontend | Empty state with Add Transaction shortcut | 2.7 |
| TXL-052 | Frontend | Incomplete filter prompt | 2.7 |
| TXL-053 | Frontend | Transaction fetch error with retry | 2.7 |
| TXL-054 | Frontend | Asset list fetch error with retry | 2.7 |

---

## 4. Implementation Order

1. Backend: TXL-013 (sections 2.1.1 through 2.1.6)
2. Type sync: `just generate-types` (section 2.2)
3. Frontend gateway extension (section 2.4)
4. Frontend presenter extension (section 2.5)
5. Frontend hook: `useTransactionList` (section 2.6)
6. Frontend component: `TransactionListPage` (section 2.7)
7. Frontend route registration (section 2.3)
8. Frontend account details inspect action (section 2.8)
9. Frontend index re-export (section 2.9)
10. i18n keys (section 2.10)
11. `just format` + `python3 scripts/check.py`
12. Reviewers: `reviewer`, `reviewer-backend`, `reviewer-frontend`
13. `i18n-checker`
14. Tests (backend inline `#[cfg(test)]` if non-trivial logic added; frontend colocated `.test.ts` for `useTransactionList` hook)
15. Documentation: update `ARCHITECTURE.md` (add TXL-013 command to transaction context section, add `transaction_list/` sub-feature to transactions feature section), update `docs/todo.md` if new tech debt
16. `spec-checker` + `workflow-validator`

---

## 5. Files Created or Modified

### New files
- `src/features/transactions/transaction_list/TransactionListPage.tsx`
- `src/features/transactions/transaction_list/useTransactionList.ts`
- `src/features/transactions/transaction_list/useTransactionList.test.ts`

### Modified files
- `src-tauri/src/context/transaction/domain/transaction.rs` -- add trait method
- `src-tauri/src/context/transaction/repository/transaction.rs` -- add implementation
- `src-tauri/src/context/transaction/service.rs` -- add service method
- `src-tauri/src/context/transaction/api.rs` -- add Tauri command (replace stub)
- `src-tauri/src/context/transaction/mod.rs` -- add public re-export of api
- `src-tauri/src/core/specta_builder.rs` -- register new command
- `src/bindings.ts` -- auto-regenerated (DO NOT EDIT)
- `src/features/transactions/gateway.ts` -- add `getAssetIdsForAccount`
- `src/features/transactions/shared/presenter.ts` -- add `type` field
- `src/features/transactions/index.ts` -- add `TransactionListPage` export
- `src/router.tsx` -- add transaction list route
- `src/features/account_details/account_details_view/AccountDetailsView.tsx` -- add inspect action
- `src/i18n/locales/en/common.json` -- add TXL i18n keys
- `src/i18n/locales/fr/common.json` -- add TXL i18n keys
- `ARCHITECTURE.md` -- document new command and sub-feature
