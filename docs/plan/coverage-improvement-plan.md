# Coverage Improvement Plan

**Baseline (2026-05-02):** Frontend 52.6% · Backend 55.9%
**Target:** ~75% frontend · ~75% backend
**Approach:** Backend first (integration → unit), then frontend (gateways → hooks → utilities), then selective E2E.

---

## Phase 1 — Backend

### Skipped intentionally

- All `api.rs` files — Tauri command wrappers, no domain logic, implicitly exercised by integration tests.
- `use_cases/update_checker/service.rs` (0%, 0/66) — `check()`, `download()`, `install()` require a live `AppHandle`; not unit-testable without a full Tauri runtime.
- `core/db.rs`, `core/logger.rs`, `core/uow.rs` — infrastructure; no domain logic.
- `main.rs` — app entry point; not testable without a runtime.

---

### 1.1 Integration tests — `src-tauri/tests/`

New file: `src-tauri/tests/account_service_crud.rs`
Pattern: real `sqlite::memory:` pool, `sqlx::migrate!`, `AccountService` via public API only (B27).

Tests to write (each references the spec rule it protects):

| Test function                                           | What it proves                                                            |
| ------------------------------------------------------- | ------------------------------------------------------------------------- |
| `create_account_rejects_name_already_exists`            | `create()` returns `NameAlreadyExists` when name collides                 |
| `get_all_returns_created_accounts`                      | `get_all()` returns all non-deleted rows                                  |
| `get_by_id_returns_none_for_missing`                    | `get_by_id()` returns `Ok(None)` on unknown id                            |
| `delete_account_removes_it_from_get_all`                | `delete()` makes the account disappear                                    |
| `get_deletion_summary_counts_holdings_and_transactions` | `get_deletion_summary()` returns correct `(holding_count, tx_count)` pair |
| `get_holdings_for_account_returns_empty_before_any_buy` | `get_holdings_for_account()` returns empty vec before any transaction     |
| `get_transactions_returns_chronological_order`          | `get_transactions()` orders by date ascending                             |
| `get_asset_ids_for_account_deduplicates`                | `get_asset_ids_for_account()` returns unique asset IDs                    |

New file: `src-tauri/tests/asset_service_crud.rs`
Pattern: same pool setup, `AssetService` with `SideEffectEventBus`, `AssetCategory` pre-seeded.

| Test function                                   | What it proves                                                     |
| ----------------------------------------------- | ------------------------------------------------------------------ |
| `create_category_and_retrieve_it`               | `create_category()` + `get_category_by_id()` roundtrip             |
| `update_category_changes_label`                 | `update_category()` persists new label                             |
| `delete_category_removes_it`                    | `delete_category()` makes it disappear from `get_all_categories()` |
| `update_asset_rejected_when_archived`           | `update_asset()` returns `Archived` error                          |
| `update_asset_rejected_when_category_not_found` | `update_asset()` returns category `NotFound` error                 |
| `delete_asset_publishes_asset_updated_event`    | `delete_asset()` fires `AssetUpdated` on the event bus             |
| `archive_asset_publishes_asset_updated_event`   | `archive_asset()` fires `AssetUpdated`                             |
| `unarchive_asset_publishes_asset_updated_event` | `unarchive_asset()` fires `AssetUpdated`                           |
| `create_asset_publishes_asset_updated_event`    | `create_asset()` fires `AssetUpdated`                              |

---

### 1.2 Unit tests — inline `mod tests` in `src/`

#### `src-tauri/src/context/account/service.rs` (currently 83%)

Missing coverage: lines 97–100 (`delete`), 108–120 (holding reads), 128–148 (tx reads), 334–340 (event helper).

Add to existing `mod tests` block using mockall mocks (B26):

| Test function                                    | Missing line(s) covered |
| ------------------------------------------------ | ----------------------- |
| `delete_delegates_to_repo_and_emits_event`       | 97–100                  |
| `get_all_delegates_to_repo`                      | 44–46                   |
| `get_by_id_delegates_to_repo`                    | 49–51                   |
| `get_holdings_for_account_delegates_to_repo`     | 108–110                 |
| `get_holding_by_account_asset_delegates_to_repo` | 113–121                 |
| `get_transaction_by_id_delegates_to_repo`        | 128–130                 |
| `get_transactions_delegates_to_repo`             | 133–141                 |
| `get_asset_ids_for_account_delegates_to_repo`    | 144–148                 |

All use `MockAccountRepository` / `MockHoldingRepository` / `MockTransactionRepository` (already declared in mockall derives in the domain files).

#### `src-tauri/src/context/asset/service.rs` (currently 79%)

Missing coverage: `update_asset()` error paths (archived guard, category not found), event bus branches on `archive_asset` / `unarchive_asset` / `delete_asset` / `create_category` / `update_category` / `delete_category`.

Add to existing `mod tests` block:

| Test function                                     | Missing line(s) covered |
| ------------------------------------------------- | ----------------------- |
| `update_asset_returns_archived_error`             | 97–100                  |
| `update_asset_returns_category_not_found`         | 103–107                 |
| `archive_asset_emits_event_when_bus_present`      | 128–132                 |
| `unarchive_asset_emits_event_when_bus_present`    | 138–142                 |
| `delete_asset_emits_event_when_bus_present`       | 148–151                 |
| `create_category_emits_event_when_bus_present`    | 170–175                 |
| `update_category_emits_event_when_bus_present`    | 188–193                 |
| `delete_category_emits_event_when_bus_present`    | 205                     |
| `record_asset_price_emits_event_when_bus_present` | 233                     |

---

## Phase 2 — Frontend

### Skipped intentionally

- All modal/form display components (`BuyTransactionModal`, `SellTransactionModal`, `PriceModal`, `PriceHistoryModal`, `EditTransactionModal`, `AddTransactionModal`, `AssetForm`, `AccountForm`, `CategoryForm`, etc.) — presentational; best covered by E2E or existing RTL tests on their hooks.
- `ui/components/field/useDateField.ts` (42%) and `useAmountField.ts` (0%) — DOM viewport calculations and ref manipulation; unit tests would be brittle.
- `ui/components/field/useComboboxField.ts` (0%) — keyboard navigation + portal positioning; defer to RTL.
- `DesignSystemPage.tsx` — dev-only page, no business logic.
- `main.tsx` — app entry point.
- All `index.ts` barrel re-exports.

---

### 2.1 Gateway unit tests

Rule F3: gateways are the only files that call `commands.*`. Test them by mocking `@tauri-apps/api/core` (already done in existing gateway tests — follow the same `vi.hoisted` pattern).

#### `src/features/transactions/gateway.ts` (12% → ~100%)

New file: `src/features/transactions/gateway.test.ts`

Methods to cover:

- `getTransactions(accountId, assetId)` — success path, propagates error
- `buyHolding(...)` — success path, propagates error
- `sellHolding(...)` — success path, propagates error
- `correctTransaction(...)` — success path, propagates error
- `cancelTransaction(accountId, txId)` — success path, propagates error
- `getTransactionById(id)` — success path
- `getDeletableAssetIds(accountId)` — success path

#### `src/features/account_details/gateway.ts` (22% → ~100%)

File: `src/features/account_details/gateway.test.ts` (already exists — extend it)

Uncovered methods:

- `getHoldingByAccountAsset(accountId, assetId)` — success + returns null
- `getAccountById(id)` — success path
- `subscribeToAccountEvents(handler)` — verify listener is registered and returns unlisten function
- `openHolding(...)` — success path (if not already tested)

#### `src/features/accounts/gateway.ts` (33% → ~100%)

File: `src/features/account_details/gateway.test.ts` → actually `src/features/accounts/gateway.test.ts` (already exists — extend it)

Uncovered: `updateAccount(...)`, `deleteAccount(...)`, `getDeletionSummary(id)`

#### `src/features/assets/gateway.ts` (33% → ~100%)

File: `src/features/assets/gateway.test.ts` (check if exists; create if not)

Uncovered: `updateAsset(...)`, `archiveAsset(id)`, `unarchiveAsset(id)`, `deleteAsset(id)`, `lookupAssetWeb(query)`, event subscription

#### `src/features/categories/gateway.ts` (40% → ~100%)

File: `src/features/categories/gateway.test.ts`

Uncovered: `addCategory(label)`, `updateCategory(id, label)`, `deleteCategory(id)`

---

### 2.2 Hook unit tests

Follow the established pattern: `vi.hoisted` for spy references, `vi.mock` for gateway/store/i18n/logger, `renderHook` + `act`.

#### `src/features/categories/useCategories.ts` (0%)

New file: `src/features/categories/useCategories.test.ts`

| Test                                                      | What it covers |
| --------------------------------------------------------- | -------------- |
| `addCategory success calls gateway and shows snackbar`    | happy path     |
| `addCategory error logs and shows error snackbar`         | error path     |
| `updateCategory success calls gateway and shows snackbar` | happy path     |
| `updateCategory error shows error snackbar`               | error path     |
| `deleteCategory success calls gateway and shows snackbar` | happy path     |
| `deleteCategory error shows error snackbar`               | error path     |

Mocks needed: `categoryGateway`, `useSnackbar`, `logger`, `react-i18next`.

#### `src/features/assets/useAssets.ts` (0%)

New file: `src/features/assets/useAssets.test.ts`

| Test                                   | What it covers |
| -------------------------------------- | -------------- |
| `addAsset success calls gateway`       | happy path     |
| `addAsset error shows error snackbar`  | error path     |
| `updateAsset success calls gateway`    | happy path     |
| `archiveAsset success calls gateway`   | happy path     |
| `unarchiveAsset success calls gateway` | happy path     |
| `deleteAsset success calls gateway`    | happy path     |
| `activeCount excludes archived assets` | memo logic     |

Mocks needed: `assetGateway`, `useSnackbar`, `useAppStore`, `logger`, `react-i18next`.

#### `src/features/accounts/useAccounts.ts` (22%)

File: extend existing `useAccounts.test.ts` (if exists) or create `src/features/accounts/useAccounts.test.ts`

Uncovered paths — missing tests for:

| Test                                                 | What it covers               |
| ---------------------------------------------------- | ---------------------------- |
| `deleteAccount success calls gateway`                | `deleteAccount()` happy path |
| `deleteAccount error shows error snackbar`           | error path                   |
| `getDeletionSummary success returns summary`         | `getDeletionSummary()`       |
| `getDeletionSummary propagates error`                | error path                   |
| `addAccount NameAlreadyExists shows inline error`    | error code branch            |
| `updateAccount NameAlreadyExists shows inline error` | error code branch            |

#### `src/features/transactions/useTransactions.ts` (0%)

New file: `src/features/transactions/useTransactions.test.ts`

| Test                                       | What it covers |
| ------------------------------------------ | -------------- |
| `buyHolding success calls gateway`         | happy path     |
| `buyHolding error shows error snackbar`    | error path     |
| `sellHolding success calls gateway`        | happy path     |
| `sellHolding error shows error snackbar`   | error path     |
| `correctTransaction success calls gateway` | happy path     |
| `cancelTransaction success calls gateway`  | happy path     |
| `getTransactions returns list`             | read path      |

Mocks needed: `transactionGateway`, `useSnackbar`, `logger`, `react-i18next`.

#### `src/features/shell/useHeaderConfig.ts` (43%)

File: `src/features/shell/useHeaderConfig.test.ts`

Uncovered: route matching for account detail path, asset-detail path, top-level nav fallback.

| Test                                                         | What it covers            |
| ------------------------------------------------------------ | ------------------------- |
| `returns account name for /accounts/:id route`               | account detail path match |
| `returns asset name for /accounts/:id/transactions/:assetId` | nested route match        |
| `returns nav item label for top-level route`                 | navItems lookup           |
| `returns undefined for unknown route`                        | fallback                  |

Mocks needed: `useLocation` (tanstack router), `useAppStore`, `react-i18next`.

#### `src/features/about/about_page/useAboutPage.ts` (42%)

Extend existing test or create `src/features/about/about_page/useAboutPage.test.ts`

Missing: `checkForUpdates()` error path, `latestVersion` display when update available.

---

### 2.3 Utility / simple logic

#### `src/features/account_details/shared/formatDate.ts` (0%)

New file: `src/features/account_details/shared/formatDate.test.ts`

Tests: `formatIsoDate("2024-01-15")` → expected locale string; edge: invalid date input.

#### `src/lib/useFuzzySearch.ts` (0%)

New file: `src/lib/useFuzzySearch.test.ts`

Tests: query < 2 chars → returns full list; matching query → filters; no match → returns empty.

#### `src/features/account_details/shared/PnlCell.tsx` (0%)

New file: `src/features/account_details/shared/PnlCell.test.tsx`

Tests (RTL): positive value → green class; negative value → red class; zero → neutral class.

---

## Phase 3 — E2E (selective)

Principle: only test flows where the full request→render chain cannot be validated by hook + gateway unit tests alone. Priority = business criticality × test fragility risk.

### Selected flows

#### `e2e/accounts/accounts.test.ts` — Account management

Justification: CRUD account is the entry point to the entire app. No existing E2E coverage.

| Test                                       | Spec rule |
| ------------------------------------------ | --------- |
| Create account → appears in account list   | ACC-001   |
| Edit account name → list reflects update   | ACC-002   |
| Delete account → removed from list         | ACC-003   |
| Create duplicate name → inline error shown | ACC-004   |

Selectors needed: form `id="add-account-form"`, field `id="add-account-name"`, `id="add-account-currency"`, submit `button[type="submit"][form="add-account-form"]`, error `[role="alert"]`.

#### `e2e/assets/assets.test.ts` — Asset management

Justification: asset lifecycle (create → archive → unarchive → delete) is critical path; no existing E2E coverage beyond web lookup.

| Test                                    | What it covers    |
| --------------------------------------- | ----------------- |
| Create asset → appears in asset table   | create happy path |
| Archive asset → moves to archived state | archive flow      |
| Unarchive asset → returns to active     | unarchive flow    |

#### `e2e/account_details/buy_sell.test.ts` — Buy + Sell transaction

Justification: core portfolio flow — filling a buy + a sell and verifying holding update is the primary user journey. Cannot be tested without real DB + real Tauri commands.

| Test                                             | Spec rule |
| ------------------------------------------------ | --------- |
| Buy holding → holding appears in account details | TRX-010   |
| Sell holding → quantity decremented              | TRX-020   |
| Sell more than held → error shown                | TRX-030   |

### Deferred

- **Category management** — simple CRUD, low business risk, unit tests sufficient.
- **Settings page** — language toggle; no Tauri command involved, no integration risk.
- **Update checker** — network-dependent, inherently flaky in CI.
- **Transaction list page** — read-only view; already exercised by buy/sell E2E above.
- **Price recording modal** — already covered by `asset_price_crud.rs` integration test; E2E adds little.

---

## Estimated effort

| Phase                                      | Work           | Est. hours |
| ------------------------------------------ | -------------- | ---------- |
| 1.1 Integration tests (2 new files)        | 17 tests       | 4h         |
| 1.2 Unit tests (2 existing files extended) | 17 tests       | 3h         |
| 2.1 Gateway tests (5 files)                | ~25 tests      | 3h         |
| 2.2 Hook tests (6 files)                   | ~35 tests      | 8h         |
| 2.3 Utility tests (3 files)                | ~8 tests       | 1h         |
| 3 E2E (3 new files)                        | ~10 tests      | 4h         |
| **Total**                                  | **~112 tests** | **~23h**   |

Expected coverage after completion: **~75% frontend · ~78% backend**.
