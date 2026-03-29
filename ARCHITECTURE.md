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
4. Bounded context services: `AssetService`, `AccountService`
5. Event forwarder spawned to bridge `SideEffectEventBus` → Tauri frontend events

Log file location: `{app_log_dir}/app.log` (use `just collect-logs` to retrieve).

### Command Registry (`core/specta_builder.rs`)

All Tauri commands are registered here via `tauri_specta::collect_commands![]`. **Never register commands elsewhere.**

### Core Modules (`core/`)

| Module | Role |
|--------|------|
| `core/db.rs` | SQLite connection pool + migrations |
| `core/logger.rs` | `FRONTEND`/`BACKEND` constants + `log_frontend` Tauri command |
| `core/specta_types.rs` | Specta/TypeScript serialization documentation |
| `core/specta_builder.rs` | Tauri command registry — all commands registered here |
| `core/event_bus/` | `SideEffectEventBus` + `Event` enum |

### Event Bus (`core/event_bus/`)

Published on every state change. Frontend listens via a single `events.event.listen()` subscription in the global store.

| Event | Published by |
|-------|-------------|
| `AssetUpdated` | `context/asset/` |
| `CategoryUpdated` | `context/asset/` |
| `AccountUpdated` | `context/account/` |

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
- `id`, `name`, `class: AssetClass`, `category: AssetCategory`, `currency` (ISO 4217), `risk_level` (1–5), `reference` (ticker/ISIN or auto-generated `INT-{class}-{short_id}`)
- Factory methods: `new()` (generates ID + validates), `update_from()` (uses provided ID + validates), `from_storage()` (no validation)
- `AssetClass` enum: `RealEstate`, `Cash`, `Stocks`, `Bonds`, `ETF`, `MutualFunds`, `DigitalAsset`

**Entity: `AssetCategory`**
- `id`, `name`
- `SYSTEM_CATEGORY_ID = "default-uncategorized"` — fixed ID of the system fallback category
- Factory methods: `new()`, `update_from()`, `from_storage()`

**Entity: `AssetPrice`**
- Price/valuation data for an asset at a point in time

**Repository traits: `AssetRepository`, `AssetCategoryRepository`, `PriceRepository`**
- `get_all`, `get_by_id`, `create`, `update`, `delete`
- `AssetCategoryRepository` extras: `find_by_name` (case-insensitive), `reassign_assets_and_delete` (atomic transaction)

**Service: `AssetService`**
- CRUD for assets, categories, and prices
- Publishes `AssetUpdated` and `CategoryUpdated` events

**Tauri commands (`api.rs`)**
- `get_assets() -> Vec<Asset>`
- `add_asset(name, class, categoryId, currency, riskLevel, reference?) -> Asset`
- `update_asset(...) -> Asset`
- `delete_asset(id)`
- `get_categories() -> Vec<AssetCategory>`
- `add_category(label) -> AssetCategory`
- `update_category(id, label) -> AssetCategory`
- `delete_category(id)`
- `create_asset_price(...) -> AssetPrice`

---

### Account (`context/account/`)

**Entity: `Account`**
- `id`, `name`, `update_frequency: UpdateFrequency`
- `UpdateFrequency` enum: `Automatic`, `ManualDay`, `ManualWeek`, `ManualMonth`, `ManualYear`
- Factory methods: `new()` (generates ID + validates), `from_storage()` (no validation)

**Entity: `AssetAccount`**
- Junction entity linking an account to an asset (holdings)

**Repository traits: `AccountRepository`, `AssetAccountRepository`**
- `get_all`, `get_by_id`, `create`, `update`, `delete`

**Service: `AccountService`**
- CRUD for accounts and account holdings (asset–account links)
- Publishes `AccountUpdated` events

**Tauri commands (`api.rs`)**
- `get_accounts() -> Vec<Account>`
- `add_account(name, updateFrequency) -> Account`
- `update_account(account) -> Account`
- `delete_account(id)`
- `get_account_holdings(accountId) -> Vec<AssetAccount>`
- `upsert_account_holding(accountId, assetId, ...) -> AssetAccount`
- `remove_account_holding(accountId, assetId)`

---

### Database

- SQLite, migrations in `src-tauri/migrations/`
- After schema changes: `just clean-db` → `cargo sqlx prepare`
- Never add `BEGIN`/`COMMIT` in migrations (sqlx wraps each in a transaction)
- `202603280001_categories_case_insensitive.sql` — replaces `categories` name index with `UNIQUE ON LOWER(name)` for case-insensitive enforcement

---

## Frontend (`src/`)

### Global Store (`lib/store.ts`)

**`useAppStore`** (Zustand) — shared data across features:

| Field | Type | Reloaded on event |
|-------|------|-------------------|
| `assets` | `Asset[]` | `AssetUpdated` |
| `categories` | `AssetCategory[]` | `CategoryUpdated` |
| `accounts` | `Account[]` | `AccountUpdated` |

Loading states: `isLoadingAssets`, `isLoadingCategories`, `isLoadingAccounts`, `isInitialized`

`init()` — parallelized initial fetch + sets up a single `events.event.listen()` subscription that dispatches to fetch handlers by event type.

### Infrastructure

| Path | Role |
|------|------|
| `bindings.ts` | Auto-generated Tauri bindings — **DO NOT EDIT** |
| `lib/store.ts` | Global Zustand store |
| `lib/logger.ts` | Frontend logger — thin wrapper over `log_frontend` Tauri command |
| `lib/useFuzzySearch.ts` | Generic Fuse.js fuzzy-search hook used by `ComboboxField` |
| `i18n/config.ts` | react-i18next setup — fr default, en fallback, `common` namespace |
| `i18n/locales/{fr,en}/common.json` | Translation files — `category.*`, `action.*`, `field.*` key groups |
| `ui/global.css` | Clinical Atelier design system — indigo M3 palette, dark mode (`.dark`), Inter+Manrope fonts, elevation shadows (`shadow-elevation-*`), header gradient tokens, animation utilities |
| `ui/components/index.ts` | UI barrel — re-exports all shared components |
| `ui/components/button/` | `Button` (6 variants, 3 sizes) + `IconButton` (5 variants, round/square) |
| `ui/components/fab/` | `FAB` — Floating Action Button |
| `ui/components/field/` | `TextField`, `SelectField`, `CompactSelectField`, `SearchField`, `AmountField`, `DateField`, `ComboboxField` |
| `ui/components/modal/` | `Dialog`, `ConfirmationDialog`, `FormModal`, `ListModal`, `TabModal`, `SelectionModal`, `ModalContainer` |
| `ui/components/layout/` | `ManagerLayout`, `ManagerHeader` |
| `ui/components/card/` | `StatCard` |
| `ui/components/SortIcon.tsx` | Generic sort direction indicator |

---

### Features (`src/features/`)

All features follow the **feature-first (gold)** layout. Reference: `features/assets/`.

#### Assets (`features/assets/`)
- Gateway: `get_assets`, `add_asset`, `update_asset`, `delete_asset`
- Sub-features: `asset_table/`, `add_asset/`, `edit_asset_modal/`
- Shared: `shared/presenter.ts`, `shared/validateAsset.ts`

#### Categories (`features/categories/`)
- Gateway: `get_categories`, `add_category`, `update_category`, `delete_category`
- Sub-features: `category_table/`, `add_category/`, `edit_category_modal/`
- Shared: `shared/presenter.ts` — `isSystemCategory(id)` predicate, `SYSTEM_CATEGORY_ID` constant
- UX: FAB triggers `AddCategoryModal`; table rows show Edit/Delete `IconButton`s; system category has "Défaut" badge, Edit disabled

#### Accounts (`features/accounts/`)
- Gateway: `get_accounts`, `add_account`, `update_account`, `delete_account`
- Sub-features: `account_table/`, `add_account/`, `edit_account_modal/`

#### Account Asset Details (`features/account_asset_details/`)
- Drill-down view: assets held within a specific account
- Gateway: `get_account_holdings`, `upsert_account_holding`, `remove_account_holding`
- Component: `AccountAssetDetailsView.tsx`

#### Shell (`features/shell/`)
- Layout wrapper: `MainLayout.tsx`, `Sidebar.tsx`, `Content.tsx`, `Footer.tsx`
- `Header.tsx` — indigo gradient header with `ThemeToggle`
- `useSidebar.ts` — `NAV_ITEMS` constant (base items + `"Design System"` entry added only when `import.meta.env.DEV`)
- `theme_toggle/useThemeToggle.ts` — day/night/auto cycle, localStorage persistence, OS media query listener
- `theme_toggle/ThemeToggle.tsx` — Sun/Moon/Monitor icon button

#### Design System (`features/design-system/`) — **dev only**
- `DesignSystemPage.tsx` — component showcase page (Button, IconButton variants, sizes, states)
- Gated by `import.meta.env.DEV` in both `useSidebar.ts` (nav item) and `App.tsx` (render)

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
