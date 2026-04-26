# ARCHITECTURE.md

> **For Claude Code** — kept up to date after each implementation (workflow step 10).
> Rules: [docs/backend-rules.md](docs/backend-rules.md) | [docs/frontend-rules.md](docs/frontend-rules.md)
> Feature specs: [docs/](docs/)

---

## Stack

- **Desktop app**: Tauri 2 (single executable)
- **Frontend**: React 19 + TypeScript, Zustand
- **Backend**: Rust, SQLite via sqlx (compile-time query verification)
- **IPC**: Specta-generated bindings (`src/bindings.ts`) — run `just generate-types` to sync

---

## Backend (`src-tauri/src/`)

### App Wiring (`lib.rs`)

`run()` initializes and injects all services as Tauri state:

1. `create_app_dirs()` — resolves and creates `local_data_dir` + `log_dir`
2. `initialize_tracing()` — sets up dual-output subscriber: `app.log` (no ANSI) + stderr; `EnvFilter` defaults to `debug` (override with `RUST_LOG`)
3. `Arc<Database>`, `Arc<SideEffectEventBus>`
4. Bounded context services: `AssetService`, `AccountService`, `TransactionService`
5. Event forwarder spawned to bridge `SideEffectEventBus` → Tauri frontend events
6. `Arc<UpdateState>` — managed separately from `AppState` so it is accessible before the DB is ready

Log file location: `{app_log_dir}/app.log` (use `just collect-logs` to retrieve).

### Command Registry (`core/specta_builder.rs`)

All Tauri commands are registered here via `tauri_specta::collect_commands![]`. **Never register commands elsewhere.**

### Core Modules (`core/`)

| Module                   | Role                                                          |
| ------------------------ | ------------------------------------------------------------- |
| `core/db.rs`             | SQLite connection pool + migrations                           |
| `core/logger.rs`         | `FRONTEND`/`BACKEND` constants + `log_frontend` Tauri command |
| `core/specta_types.rs`   | Specta/TypeScript serialization documentation                 |
| `core/specta_builder.rs` | Tauri command registry — all commands registered here         |
| `core/event_bus/`        | `SideEffectEventBus` + `Event` enum                           |

### Event Bus (`core/event_bus/`)

Published on every state change. Frontend listens via a single `events.event.listen()` subscription in the global store.

| Event                | Published by                                                                 |
| -------------------- | ---------------------------------------------------------------------------- |
| `AssetUpdated`       | `context/asset/`                                                             |
| `CategoryUpdated`    | `context/asset/`                                                             |
| `AccountUpdated`     | `context/account/`                                                           |
| `TransactionUpdated` | `context/transaction/` via `TransactionService.notify_transaction_updated()` |
| `AssetPriceUpdated`  | `context/asset/` via `AssetService.record_price()` (MKT-026)                 |

### Use Cases (`use_cases/`)

Cross-cutting application use cases that span multiple bounded contexts or require app-level infrastructure.

#### Account Details (`use_cases/account_details/`)

Orchestrates a cross-context read of account + asset data for the Account Details view (spec: `docs/spec/account-details.md`, ADR-003, ADR-004).

- `orchestrator.rs` — `AccountDetailsUseCase` injects `Arc<AccountService>` + `Arc<AssetService>`; `get_account_details(account_id)` fetches account, all holdings, splits into active (qty > 0) and closed (qty = 0 with `last_sold_date` set), enriches both with asset metadata, computes per-holding `cost_basis` via i128 intermediates (ACD-023/024), fetches latest asset price per holding (MKT-031, degrades gracefully on failure), computes `unrealized_pnl` and `performance_pct` when currencies match (MKT-033/034/035), sorts both lists by asset_name (ACD-033, ACD-046), returns `AccountDetailsResponse`
- DTOs:
  - `HoldingDetail` — active position: asset_id, asset_name, asset_reference, asset_currency, quantity, average_price, cost_basis, realized_pnl (all i64 micros), current_price, current_price_date, unrealized_pnl, performance_pct (nullable MKT fields)
  - `ClosedHoldingDetail` — closed position (qty=0): asset_id, asset_name, asset_reference, realized_pnl, last_sold_date: String (ACD-044, ACD-045)
  - `AccountDetailsResponse` — account_name, holdings, closed_holdings, total_holding_count: i64, total_cost_basis, total_realized_pnl (ACD-047), total_unrealized_pnl (MKT-040: sum of qualifying holdings, None when empty)
- `total_realized_pnl` is sourced from `Holding.total_realized_pnl` (persisted by `recalculate_holding`), not from a live transaction query; supersedes SEL-038
- Frontend `useAccountDetails` subscribes to `TransactionUpdated`, `AssetUpdated`, and `AssetPriceUpdated` events to trigger re-fetch (MKT-036)
- `api.rs` — `get_account_details(account_id: String) -> Result<AccountDetailsResponse, String>` Tauri command

#### Record Transaction (`use_cases/record_transaction/`)

Orchestrates transaction creation, update, and deletion (Purchase + Sell) across `transaction/`, `account/`, and `asset/` bounded contexts (spec: `docs/spec/financial-asset-transaction.md`, `docs/spec/sell-transaction.md`).

- `orchestrator.rs` — `RecordTransactionUseCase` with `create_transaction`, `update_transaction`, `delete_transaction`, `get_transactions`
  - Atomicity (TRX-027): all DB writes within each operation use `pool.begin()` + `commit()` directly
  - Purchase: VWAP computation (TRX-030, TRX-036), auto-unarchive on purchase of archived asset (TRX-028)
  - Sell: `compute_sell_total` (SEL-023): `floor(floor(qty×price/MICRO)×rate/MICRO) − fees`; closed-position guard (SEL-012); oversell guard (SEL-021); archived-asset guard (SEL-037)
  - `recalculate_holding`: full chronological replay over all transactions for an (account, asset) pair; computes running VWAP, `realized_pnl` per sell (SEL-024), `total_realized_pnl` (cumulative sum), and `last_sold_date` (max sell date, ISO string); returns `(Holding, HashMap<tx_id, pnl>)`
  - Sell recalculation cascades on update (SEL-031) and delete (SEL-033) — all sibling sells get updated P&L
  - Event published via `TransactionService.notify_transaction_updated()` after commit (B8)
- `api.rs` — four Tauri commands: `add_transaction`, `update_transaction`, `delete_transaction`, `get_transactions`
- `CreateTransactionDTO` — serializable DTO struct passed as single command parameter (Specta limit workaround)

#### Update Checker (`use_cases/update_checker/`)

Implements the application auto-update lifecycle (spec: `docs/update.md`).

- `service.rs` — `check()`, `download()`, `install()` functions; `UpdateInfo` (version string) + `UpdateState` (concurrent download guard + downloaded bytes store)
- `api.rs` — three Tauri commands: `check_for_update`, `download_update`, `install_update`
- Raw Tauri events emitted by the backend (frontend listens via `listen()`):
  - `update:available` — emitted on check when a new version exists (carries `UpdateInfo`)
  - `update:progress` — emitted during download (carries `percent: u64`)
  - `update:complete` — emitted when download + checksum OK
  - `update:error` — emitted on download or checksum failure (carries error message)
  - `db:migration_error` — emitted by `core/db.rs` if a migration fails at startup
- Business invariants: concurrent downloads blocked via `AtomicBool`; downloaded bytes stored in `Mutex<Option<Vec<u8>>>` between download and install commands; no breaking schema changes allowed (R15)

---

## Bounded Contexts (`context/`)

No cross-context imports. Public API via `mod.rs` only.

Directory structure per context:

```
context/{domain}/
├── domain/       # Entities + repository traits
├── repository/   # SQLite repository implementations
├── service.rs    # Business logic
├── api.rs        # Tauri command handlers
└── mod.rs        # Public exports
```

### Asset (`context/asset/`)

**Entity: `Asset`**

- `id`, `name`, `class: AssetClass`, `category: AssetCategory`, `currency` (ISO 4217), `risk_level` (1–5), `reference` (mandatory — ticker/ISIN), `is_archived: bool`
- Factory methods: `new()` (generates ID + validates), `with_id()` (uses provided ID + validates), `restore()` (no validation — from storage)
- `AssetClass` enum: `RealEstate`, `Cash`, `Stocks`, `Bonds`, `ETF`, `MutualFunds`, `DigitalAsset`
- `AssetClass::default_risk()` — returns default risk level per class (Cash→1, Bonds/RE→2, MF/ETF→3, Stocks→4, Digital→5)
- Archive is reversible soft-flag (`is_archived`). Soft-delete (`is_deleted`) is permanent and separate.

**Entity: `AssetCategory`**

- `id`, `name`
- `SYSTEM_CATEGORY_ID = "default-uncategorized"` — fixed ID of the system fallback category
- Factory methods: `new()`, `update_from()`, `from_storage()`

**Repository traits: `AssetRepository`, `AssetCategoryRepository`**

- `get_all` (active only), `get_all_including_archived`, `get_by_id`, `create`, `update`, `delete`
- `archive(id)`, `unarchive(id)` — toggle `is_archived` flag
- `AssetCategoryRepository` extras: `find_by_name` (case-insensitive), `reassign_assets_and_delete` (atomic transaction)

**Service: `AssetService`**

- CRUD for assets and categories
- `update_asset` rejects archived assets with `error.asset.archived_readonly`
- Publishes `AssetUpdated` and `CategoryUpdated` events

**Tauri commands (`api.rs`)**

- `get_assets() -> Vec<Asset>` — active only
- `get_assets_with_archived() -> Vec<Asset>` — active + archived
- `add_asset(name, class, categoryId, currency, riskLevel, reference) -> Asset`
- `update_asset(...) -> Asset`
- `archive_asset(id)`, `unarchive_asset(id)`
- `delete_asset(id)`
- `get_categories() -> Vec<AssetCategory>`
- `add_category(label) -> AssetCategory`
- `update_category(id, label) -> AssetCategory`
- `delete_category(id)`

---

### Transaction (`context/transaction/`)

**Entity: `Transaction`**

- `id`, `account_id`, `asset_id`, `transaction_type: TransactionType`, `date`, `quantity: i64`, `unit_price: i64`, `exchange_rate: i64`, `fees: i64`, `total_amount: i64`, `note: Option<String>`, `realized_pnl: Option<i64>`, `created_at: String`
- `TransactionType` enum: `Purchase`, `Sell` — parsed via `FromStr`; unknown variants are a hard error (TryFrom)
- All financial fields in i64 micro-units (ADR-001)
- Validation (TRX-020, TRX-026): date in range, qty > 0, exchange_rate > 0, `total_amount` invariant checked for Purchase only (Sell uses subtraction formula)
- Factory methods: `new()`, `with_id()`, `restore()`; `created_at` is set once in `new()`, immutable on update

**Repository trait: `TransactionRepository`**

- `get_by_id`, `get_by_account_asset(account_id, asset_id) -> Vec<Transaction>` (chronological — TRX-036)
- `get_asset_ids_for_account(account_id) -> Vec<String>` — distinct asset IDs with transactions for an account (TXL-013)
- `get_realized_pnl_by_account(account_id) -> Vec<(String, i64)>` — (asset_id, sum_pnl) for all Sell transactions (SEL-038)
- `create`, `update`, `delete`

**Service: `TransactionService`**

- Read access: `get_by_id`, `get_by_account_asset`, `get_asset_ids_for_account`, `get_realized_pnl_by_account`
- `notify_transaction_updated()` — publishes `TransactionUpdated` event (B8); called by `RecordTransactionUseCase` after commit

**Tauri commands**

- Write commands (`add_transaction`, `update_transaction`, `delete_transaction`, `get_transactions`) defined in `use_cases/record_transaction/api.rs` (B9 — cross-context use case)
- `get_asset_ids_for_account` defined in `context/transaction/api.rs` (B5 — single-context read, no orchestration)

---

### Account (`context/account/`)

**Entity: `Account`**

- `id`, `name`, `update_frequency: UpdateFrequency`
- `UpdateFrequency` enum: `Automatic`, `ManualDay`, `ManualWeek`, `ManualMonth`, `ManualYear`
- Factory methods: `new()` (generates ID + trims + validates), `with_id()` (uses provided ID + trims + validates), `restore()` (no validation, DB reconstruction)
- Hard-delete: `DELETE FROM accounts WHERE id = ?`; holdings cascade via `ON DELETE CASCADE` on `holdings.account_id`

**Entity: `Holding`** (replaces `AssetAccount`, see [ADR-002](docs/adr/002-replace-asset-account-with-holding.md))

- Represents the current state of a financial position: an asset held within an account
- `id`, `account_id`, `asset_id`, `quantity: i64` (micros), `average_price: i64` (micros)
- Computed from `Transaction` records via VWAP by `RecordTransactionUseCase`
- Factory methods: `new()`, `with_id()`, `restore()`
- Hard-delete: removed when no transactions remain for the `(account_id, asset_id)` pair (TRX-034)

**Repository traits: `AccountRepository`, `HoldingRepository`**

- `AccountRepository`: `get_all`, `get_by_id`, `find_by_name`, `create`, `update`, `delete`
- `HoldingRepository`: `get_by_account`, `get_by_account_asset`, `upsert`, `delete`, `delete_by_account_asset`

**Service: `AccountService`**

- CRUD for accounts; exposes `get_holdings_for_account` (delegating to `HoldingRepository`) for use by `AccountDetailsUseCase` (ADR-004)
- `create`: validates uniqueness via `find_by_name` before insert (R3)
- `update`: calls `with_id()` for trim+validation; validates uniqueness excluding own ID (R3)
- Publishes `AccountUpdated` events

**Tauri commands (`api.rs`)**

- `get_accounts() -> Vec<Account>`
- `add_account(name, updateFrequency) -> Account`
- `update_account(account) -> Account`
- `delete_account(id)`

---

### Database

- SQLite, migrations in `src-tauri/migrations/`
- After schema changes: `just clean-db` → `cargo sqlx prepare`
- Never add `BEGIN`/`COMMIT` in migrations (sqlx wraps each in a transaction)
- `202603280001_categories_case_insensitive.sql` — replaces `categories` name index with `UNIQUE ON LOWER(name)` for case-insensitive enforcement
- `202603290001_asset_archiving.sql` — adds `is_archived INTEGER NOT NULL DEFAULT 0` column to `assets`, drops the old unique index on reference (duplicates now allowed)
- `202604120001_create_holdings.sql` — creates `holdings` table (account_id, asset_id, quantity, average_price — all i64 micro-units, ADR-001)
- `202604120002_create_transactions.sql` — creates `transactions` table with FK cascade on `accounts.id` and restrict on `assets.id`
- `202604190001_add_realized_pnl_and_created_at_to_transactions.sql` — adds `realized_pnl INTEGER` (nullable, SEL-024) and `created_at TEXT NOT NULL DEFAULT (datetime('now'))` to `transactions`
- `202604250001_add_currency_to_accounts.sql` — adds `currency TEXT NOT NULL DEFAULT 'EUR'` to `accounts` (TRX-021, SEL-036)

---

## Frontend (`src/`)

### Global Store (`lib/store.ts`)

**`useAppStore`** (Zustand) — shared data across features:

| Field        | Type                          | Reloaded on event |
| ------------ | ----------------------------- | ----------------- |
| `assets`     | `Asset[]` (active + archived) | `AssetUpdated`    |
| `categories` | `AssetCategory[]`             | `CategoryUpdated` |
| `accounts`   | `Account[]`                   | `AccountUpdated`  |

Loading states: `isLoadingAssets`, `isLoadingCategories`, `isLoadingAccounts`, `isInitialized`

`init()` — parallelized initial fetch + sets up a single `events.event.listen()` subscription that dispatches to fetch handlers by event type.

### Infrastructure

| Path                               | Role                                                                                                                                                                                |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bindings.ts`                      | Auto-generated Tauri bindings — **DO NOT EDIT**                                                                                                                                     |
| `lib/store.ts`                     | Global Zustand store                                                                                                                                                                |
| `lib/logger.ts`                    | Frontend logger — thin wrapper over `log_frontend` Tauri command                                                                                                                    |
| `lib/useFuzzySearch.ts`            | Generic Fuse.js fuzzy-search hook used by `ComboboxField`                                                                                                                           |
| `i18n/config.ts`                   | react-i18next setup — fr default, en fallback, `common` namespace                                                                                                                   |
| `i18n/locales/{fr,en}/common.json` | Translation files — `category.*`, `action.*`, `field.*` key groups                                                                                                                  |
| `ui/global.css`                    | Clinical Atelier design system — indigo M3 palette, dark mode (`.dark`), Inter+Manrope fonts, elevation shadows (`shadow-elevation-*`), header gradient tokens, animation utilities |
| `ui/components/index.ts`           | UI barrel — re-exports all shared components                                                                                                                                        |
| `ui/components/button/`            | `Button` (6 variants, 3 sizes) + `IconButton` (5 variants, round/square)                                                                                                            |
| `ui/components/fab/`               | `FAB` — Floating Action Button                                                                                                                                                      |
| `ui/components/field/`             | `TextField`, `SelectField`, `CompactSelectField`, `SearchField`, `AmountField`, `DateField`, `ComboboxField`                                                                        |
| `ui/components/modal/`             | `Dialog`, `ConfirmationDialog`, `FormModal`, `ListModal`, `TabModal`, `SelectionModal`, `ModalContainer`                                                                            |
| `ui/components/layout/`            | `ManagerLayout`, `ManagerHeader`                                                                                                                                                    |
| `ui/components/card/`              | `StatCard`                                                                                                                                                                          |
| `ui/components/SortIcon.tsx`       | Generic sort direction indicator                                                                                                                                                    |

---

### Features (`src/features/`)

All features follow the **feature-first (gold)** layout. Reference: `features/assets/`.

#### Assets (`features/assets/`)

- Gateway: `get_assets`, `get_assets_with_archived`, `add_asset`, `update_asset`, `archive_asset`, `unarchive_asset`, `delete_asset`
- `useAssets()` hook: exposes `assets` (all incl. archived), `activeCount` (computed), `addAsset`, `updateAsset`, `archiveAsset`, `unarchiveAsset`, `deleteAsset`
- Sub-features: `asset_table/`, `add_asset/`, `edit_asset_modal/`
- Shared: `shared/presenter.ts` (risk badge classes, default risk), `shared/validateAsset.ts` (duplicate reference check), `shared/constants.ts` (`SYSTEM_CATEGORY_ID`, `DEFAULT_RISK_BY_CLASS`)
- `AssetTable` filters display by `showArchived`; archived rows shown at 50% opacity
- Spec: `docs/asset.md`

#### Categories (`features/categories/`)

- Gateway: `get_categories`, `add_category`, `update_category`, `delete_category`
- Sub-features: `category_table/`, `add_category/`, `edit_category_modal/`
- Shared: `shared/presenter.ts` — `isSystemCategory(id)` predicate, `SYSTEM_CATEGORY_ID` constant
- UX: FAB triggers `AddCategoryModal`; table rows show Edit/Delete `IconButton`s; system category has "Défaut" badge, Edit disabled

#### Accounts (`features/accounts/`)

- Gateway: `get_accounts`, `add_account`, `update_account`, `delete_account`
- Sub-features: `account_table/` (`AccountTable`, `useAccountTable` — sort/filter/isEmpty/hasNoSearchResults), `add_account/` (`AddAccountModal`, `useAddAccount` — FAB modal pattern), `edit_account_modal/` (`EditAccountModal`, `useEditAccountModal`)
- Shared: `shared/presenter.ts` — `FREQUENCY_ORDER` (logical enum sort order), `FREQUENCY_I18N_KEYS`; `shared/validateAccount.ts` — `validateAccountName()`
- UX: FAB triggers `AddAccountModal`; table rows show Edit/Delete `IconButton`s; inline errors on backend rejection; loading/error/retry states; empty vs no-search-results states distinct
- Spec: `docs/account.md` (R1–R16; R17 deferred pending Holding feature; R6 deferred pending Transaction feature)

#### Transactions (`features/transactions/`)

- Gateway: `addTransaction(dto)`, `updateTransaction(id, dto)`, `deleteTransaction(id)`, `getTransactions(accountId, assetId)`, `getAssetIdsForAccount(accountId)`
- `useTransactions()` hook: wraps gateway calls with error normalization (`{ data, error }` return shape)
- Sub-features:
  - `add_transaction/` — `AddTransactionModal` + `useAddTransaction` hook (TRX-010, TRX-011, TRX-026, TRX-029)
  - `edit_transaction_modal/` — `EditTransactionModal` + `useEditTransactionModal` (TRX-031, TRX-033); uses `computeSellTotalMicro` when `transaction_type === "Sell"`
  - `transaction_list/` — `TransactionListPage` + `useTransactionList` hook (TXL spec): account/asset filter dropdowns, sortable date column, edit/delete/add row actions, realized P&L column (SEL-041, SEL-043), all UX states
- Shared:
  - `shared/types.ts` — `TransactionFormData` (decimal strings)
  - `lib/microUnits.ts` — `decimalToMicro`, `microToDecimal`, `computeTotalMicro` (TRX-026), `computeSellTotalMicro` (SEL-023)
  - `shared/presenter.ts` — `toTransactionRow()` → `TransactionRowViewModel` with `realizedPnl: string | null`, `realizedPnlRaw: number | null` for sign-based color rendering
  - `shared/validateTransaction.ts` — `validateTransactionForm()` (base) + `validateSellForm()` (adds oversell guard SEL-022)
- `store.ts` — `useTransactionStore`: `lastFetchedKey`, `refreshHoldings()` (TRX-038 stub)
- Barrel `index.ts` — shared infrastructure exports (`useTransactions`, `transactionGateway`, stores, modals); buy/sell modals live in `account_details/` (use-case boundary)
- Entry points: "Add Transaction" button navigates to `/transactions/new` (TRX-010); magnifier `IconButton` per holding row navigates to transaction list (TXL-010)
- Spec: `docs/spec/financial-asset-transaction.md`, `docs/spec/transaction-list.md`, `docs/spec/sell-transaction.md`

#### Account Details (`features/account_details/`)

- Gateway: `getAccountDetails(accountId)`, `subscribeToEvents(callback)` — only file that calls `commands.*` and `events.event.listen`
- Sub-features (use-case boundary: buy/sell modals live here, not in `transactions/`):
  - `account_details_view/AccountDetailsView.tsx` — renders header (total cost basis + realized P&L), holdings table, and all UX states (loading skeletons, empty/all-closed, error with retry, non-empty CTA)
  - `account_details_view/HoldingRow.tsx` — table row with Buy (+) / Sell (−) / magnifier action buttons; `buildTarget()` resolves account+asset metadata for modal props
  - `account_details_view/useAccountDetails.ts` — fetches via gateway on mount and on `accountId` change; re-fetches on `TransactionUpdated` and `AssetUpdated` events (ACD-039, ACD-040); exposes `holdings` and `summary` view-models via `useMemo`
  - `buy_transaction/BuyTransactionModal.tsx` + `useBuyTransaction.ts` — buy form opened from holding row (TRX-041); mirrors sell modal; includes archived-asset confirmation dialog (TRX-029)
  - `sell_transaction/SellTransactionModal.tsx` + `useSellTransaction.ts` — sell form opened from holding row (SEL-010 to SEL-037): asset read-only, max quantity hint, oversell guard, exchange rate conditional, `transaction_type: "Sell"` DTO
  - `shared/types.ts` — `ModalTarget` (accountName, assetId, assetName, assetCurrency, showExchangeRate) and `SellTarget` (extends ModalTarget with holdingQuantityMicro)
- `shared/presenter.ts` — `toHoldingRow()` and `toAccountSummary()` mapping `HoldingDetail` / `AccountDetailsResponse` to display strings; includes `realizedPnl: string`, `realizedPnlRaw: number` on `HoldingRowViewModel` and `totalRealizedPnl: string`, `totalRealizedPnlRaw: number` on `AccountSummaryViewModel` for sign-based color rendering (SEL-042, SEL-043)
- Navigation: clicking an `AccountTable` row calls `useNavigate` to `/accounts/$accountId`; `AccountDetailsView` is rendered by its own route, not conditionally by `AccountManager`
- Spec: `docs/spec/account-details.md` (ACD-010–ACD-041)

#### Update (`features/update/`)

- Gateway: `checkForUpdate`, `downloadUpdate`, `installUpdate` (via Tauri commands); event listeners for `update:available`, `update:progress`, `update:complete`, `update:error`
- `update_banner/useUpdateBanner.ts` — state machine: `idle → available → downloading → ready / error`; exposed as `UpdateBannerData` with handlers
- `update_banner/UpdateBanner.tsx` — renders banner in shell for states `available`, `downloading`, `ready`, `error`; returns `null` when `idle`
- Spec: `docs/update.md` (R1–R27)

#### About (`features/about/`)

- `about_page/useAboutPage.ts` — reads `import.meta.env.VITE_APP_VERSION`; manual check trigger with `CheckStatus: idle | checking | up_to_date | error`
- `about_page/AboutPage.tsx` — version display + manual update check button (R25–R27)

#### Shell (`features/shell/`)

- Layout wrapper: `MainLayout.tsx`, `Sidebar.tsx`, `Content.tsx`, `Footer.tsx`
- `Header.tsx` — indigo gradient header with `ThemeToggle`
- `navItems.ts` — `NAV_ITEMS` constant (base items + `"Design System"` entry added only when `import.meta.env.DEV`)
- `gateway.ts` — shell-level event listeners: `onMigrationError` (listens to `db:migration_error`)
- `theme_toggle/useThemeToggle.ts` — day/night/auto cycle, localStorage persistence, OS media query listener
- `theme_toggle/ThemeToggle.tsx` — Sun/Moon/Monitor icon button

#### Design System (`features/design-system/`) — **dev only**

- `DesignSystemPage.tsx` — component showcase page (Button, IconButton variants, sizes, states)
- Gated by `import.meta.env.DEV` in both `navItems.ts` (nav item) and `App.tsx` (render)

---

### Data Flow

```
Component
  └─ Hook (state, useMemo, callbacks)
       └─ Gateway (commands.* — positional args, matches bindings.ts exactly)
            └─ Tauri IPC
                 └─ Rust api.rs handler (Result<T, String>)
                      └─ Service (anyhow::Result<T>)
                           └─ Repository (sqlx, Arc<dyn Trait>)
                                └─ SQLite

Backend publishes {Domain}Updated event
  └─ Frontend events.event.listen() in store.init()
       └─ Store re-fetches domain data → UI re-renders
```

### Feature Layout Convention (Gold)

All new features MUST follow the feature-first layout. Reference: `features/assets/`.

```
features/{domain}/
├── gateway.ts                     # ONLY file that calls commands.* for this domain
├── {sub_feature}/
│   ├── {SubFeature}.tsx           # Component
│   ├── use{SubFeature}.ts         # Colocated hook
│   └── use{SubFeature}.test.ts    # Colocated test
├── shared/
│   ├── {Domain}Form.tsx           # Shared form component (used by add + edit)
│   ├── constants.ts               # Feature-scoped constants
│   ├── presenter.ts               # Domain → UI transformations (toRow, toFormData…)
│   └── validate{Domain}.ts        # Pure validation logic
└── index.ts                       # Public re-exports
```

**Key rules:**

- `gateway.ts` at the feature root — no `api/` wrapper folder
- Sub-features are directories grouped by **feature concern**, not by layer (no `components/`, `hooks/` folders)
- Hooks are colocated next to their component inside the sub-feature folder
- One `gateway.ts` per feature — sub-features import from it, never create their own
- `shared/presenter.ts` — pure object transforming domain types into UI shapes; keeps components free of mapping logic
