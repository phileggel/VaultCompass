# Implementation Plan — Price History CRUD (MKT-070+)

> **Spec**: `docs/spec/market-price.md` (rules MKT-070–MKT-096)
> **Contract**: `docs/contracts/asset-contract.md`
> **Domain**: `asset` bounded context (backend) + `account_details` feature (frontend)
> **ADRs in force**: ADR-001 (i64 micros), ADR-002 (Holding entity), ADR-003 (cross-context use cases), ADR-004 (use cases inject services)
> **Out of scope**: MKT-010–MKT-062 (already shipped)

---

## 1. Workflow TaskList

- [x] Review architecture & rules (`ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/frontend-rules.md`, `docs/spec/market-price.md` MKT-070+, `docs/contracts/asset-contract.md`)
- [x] No DB migration required — schema `asset_prices(asset_id, date, price)` already covers list/update/delete (verified in `src-tauri/migrations/202604260001_create_asset_prices.sql`)
- [x] Backend test stubs (`test-writer-backend` — all stubs written, red confirmed)
- [x] Backend implementation (minimal — make failing tests pass, green confirmed)
- [x] `just format` (rustfmt + clippy --fix)
- [x] Backend review (`reviewer-backend` → fix issues)
- [x] Type synchronization (`just generate-types`)
- [x] Compilation fixup (TypeScript errors from new bindings only — no UI work)
- [x] `just check` — TypeScript clean
- [x] Commit: backend layer — `feat(asset): add price history list, update, delete commands`
- [x] Frontend test stubs (`test-writer-frontend` — all stubs written, red confirmed)
- [ ] Frontend implementation (minimal — make failing tests pass, green confirmed)
- [ ] `just format`
- [ ] Frontend review (`reviewer-frontend` → fix issues)
- [ ] Commit: frontend layer — `feat(account-details): add price history modal with edit and delete`
- [ ] Cross-cutting review (`reviewer` always; `maintainer` only if root config files changed — none expected)
- [ ] i18n review (`i18n-checker` — new strings for the price history modal, edit form, delete dialog)
- [ ] Documentation update (`ARCHITECTURE.md` — add `get_asset_prices`/`update_asset_price`/`delete_asset_price` to the Asset api list and to the gateway list; `docs/todo.md` — log any deferred work in English)
- [ ] Spec check (`spec-checker`)
- [ ] Commit: tests & docs — `chore(market-price): document price history crud and update todo`

---

## 2. Detailed Implementation Plan

### 2.1 Migrations

**None.** The `asset_prices` table already has the columns and the `(asset_id, date)` primary key required for list, update, and delete (`src-tauri/migrations/202604260001_create_asset_prices.sql`). No `just migrate` / `just prepare-sqlx` step required for this plan.

---

### 2.2 Backend (`src-tauri/src/context/asset/`)

All work stays inside the `asset` bounded context (B5/B6). No cross-context imports. Per ADR-001, `AssetPrice.price` is `i64` micros; the `update_asset_price` write path accepts `f64` at the IPC boundary and converts to micros before validation, exactly like `record_asset_price` does today (MKT-024 — see `service.rs::record_price`). On the read path (`get_asset_prices`), `price` is returned as `i64` micros to the frontend (matches the contract's `AssetPrice` shared type).

#### 2.2.1 Domain — `domain/asset_price.rs`

Add three repository methods to the `AssetPriceRepository` trait (and to the `MockAssetPriceRepository` mockall block via `#[cfg_attr(test, mockall::automock)]`, already present):

| Method                  | Signature                                                                                                                                                                                 | Rule                                                       |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `get_all_for_asset`     | `async fn get_all_for_asset(&self, asset_id: &str) -> Result<Vec<AssetPrice>>` (date DESC)                                                                                                | MKT-072                                                    |
| `get_by_asset_and_date` | `async fn get_by_asset_and_date(&self, asset_id: &str, date: &str) -> Result<Option<AssetPrice>>`                                                                                         | MKT-083 / MKT-090 (existence check before update / delete) |
| `delete`                | `async fn delete(&self, asset_id: &str, date: &str) -> Result<u64>` (returns rows affected)                                                                                               | MKT-090                                                    |
| `replace_atomic`        | `async fn replace_atomic(&self, asset_id: &str, original_date: &str, new_price: AssetPrice) -> Result<()>` (single SQLite transaction: DELETE old, then INSERT … ON CONFLICT … DO UPDATE) | MKT-084                                                    |

Note: `upsert` already exists (MKT-025). For MKT-083 same-date in-place update we reuse `upsert` (a single-row primary-key write is atomic). For MKT-084 the `replace_atomic` method opens a `sqlx::Transaction` and runs delete+upsert before committing — this is single-aggregate (the `AssetPrice` value object owned by `asset`), so B22 UoW does not apply (B4 — one DB tx, one aggregate).

No new factory methods on the `AssetPrice` entity; `AssetPrice::new()` already validates MKT-021/MKT-022 and is reused by all writers.

#### 2.2.2 Domain — `domain/error.rs`

Extend `AssetPriceDomainError` with one new variant:

```rust
/// No AssetPrice record exists at (asset_id, date) for update/delete.
#[error("Asset price not found")]
NotFound,
/// Asset does not exist (already covered for record_asset_price by AssetDomainError::NotFound — kept distinct for explicit boundary mapping).
```

The existing `AssetDomainError::NotFound(String)` is reused for the asset-existence check (MKT-072 returns `AssetNotFound` — the existing variant maps cleanly). No new asset-domain error needed.

#### 2.2.3 Repository — `repository/asset_price.rs`

Implement the four new repository methods on `SqliteAssetPriceRepository`:

- `get_all_for_asset(asset_id)` — `SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ? ORDER BY date DESC`. Map rows via `AssetPrice::restore` (B1 — restore factory only in repos). Returns empty vec when no rows.
- `get_by_asset_and_date(asset_id, date)` — `SELECT … WHERE asset_id = ? AND date = ?`, `fetch_optional`.
- `delete(asset_id, date)` — `DELETE FROM asset_prices WHERE asset_id = ? AND date = ?` via `sqlx::query!`; return `result.rows_affected()`.
- `replace_atomic(asset_id, original_date, new_price)` — open `pool.begin()`, `DELETE … WHERE asset_id = ? AND date = ?`, then `INSERT … ON CONFLICT(asset_id, date) DO UPDATE SET price = excluded.price`, commit. On any sqlx error, the transaction auto-rolls back when the `Transaction` is dropped without `commit()` (MKT-084 atomicity).

All queries use sqlx macros (B15). No business logic in the repo (B0b).

#### 2.2.4 Service — `service.rs`

Add three methods on `AssetService`:

| Method                                                                                                       | Logic                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Rules                                                                                            |
| ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `get_prices(&self, asset_id: &str) -> Result<Vec<AssetPrice>>`                                               | 1. Reject unknown asset → `AssetDomainError::NotFound` (MKT-072). 2. Delegate to `price_repo.get_all_for_asset`. No event.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | MKT-072                                                                                          |
| `update_price(&self, asset_id: &str, original_date: &str, new_date: &str, new_price_f64: f64) -> Result<()>` | 1. f64 finiteness guard → `AssetPriceDomainError::NonFinite` (MKT-082). 2. Convert to `i64` micros via `(f * 1_000_000.0).round() as i64` (ADR-001 / MKT-024 boundary). 3. Build `AssetPrice::new(asset_id, new_date, micros)?` — validates `> 0` (MKT-021), well-formed ISO + not in future (MKT-022). 4. Existence check: `price_repo.get_by_asset_and_date(asset_id, original_date)` → `None` returns `AssetPriceDomainError::NotFound` (MKT-083). 5. Branch on `original_date == new_date`: same-date → `price_repo.upsert(price)`; date-changed → `price_repo.replace_atomic(asset_id, original_date, price)` (MKT-084). 6. On success publish `Event::AssetPriceUpdated` via the bus (MKT-085, B7). 7. `tracing::info!(target: BACKEND, …)` log line. | MKT-082, MKT-083, MKT-084, MKT-085, MKT-095 (asset_id is fixed across the call — never re-bound) |
| `delete_price(&self, asset_id: &str, date: &str) -> Result<()>`                                              | 1. `price_repo.delete(asset_id, date)`. 2. If `rows_affected == 0` → `AssetPriceDomainError::NotFound` (MKT-090). 3. Publish `Event::AssetPriceUpdated` (MKT-091, B7). 4. Tracing log.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | MKT-090, MKT-091                                                                                 |

Also extend the existing `record_price` method to surface the unknown-asset case as `AssetPriceCommandError::AssetNotFound` (already returns `AssetDomainError::NotFound`; the boundary mapper change is in 2.2.5).

The service stays a thin orchestrator (B21): no domain logic — validation lives in `AssetPrice::new`; atomicity lives in the repo's `replace_atomic`.

#### 2.2.5 API — `api.rs`

**Error enums** — add two new typed enums + extend the existing one:

```rust
// Extend AssetPriceCommandError (MKT-043 retroactive — currently missing)
pub enum AssetPriceCommandError {
    AssetNotFound,    // NEW — MKT-043 / MKT-072
    NotPositive,
    NonFinite,
    DateInFuture,
    Unknown,
}

// New
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum UpdateAssetPriceCommandError {
    NotFound,         // MKT-083 (record at original_date missing)
    NotPositive,      // MKT-082 / MKT-021
    NonFinite,        // MKT-082
    DateInFuture,     // MKT-082 / MKT-022
    Unknown,
}

// New
#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum DeleteAssetPriceCommandError {
    NotFound,         // MKT-090
    Unknown,
}
```

**Mappers** — add `to_update_asset_price_error` and `to_delete_asset_price_error`. Update `to_asset_price_error` to map `AssetDomainError::NotFound` → `AssetPriceCommandError::AssetNotFound` (currently falls through to `Unknown`, contract-side bug fix).

**Tauri commands** (B8 — thin: deserialize, delegate to `state.asset_service`, serialize):

```rust
#[tauri::command] #[specta::specta]
pub async fn get_asset_prices(state: State<'_, AppState>, asset_id: String)
    -> Result<Vec<AssetPrice>, AssetPriceCommandError>;

#[tauri::command] #[specta::specta]
pub async fn update_asset_price(
    state: State<'_, AppState>,
    asset_id: String,
    original_date: String,
    new_date: String,
    new_price: f64,
) -> Result<(), UpdateAssetPriceCommandError>;

#[tauri::command] #[specta::specta]
pub async fn delete_asset_price(
    state: State<'_, AppState>,
    asset_id: String,
    date: String,
) -> Result<(), DeleteAssetPriceCommandError>;
```

`AssetPrice` must be exposed as a Specta `Type`. It is currently a backend-internal struct in `domain/asset_price.rs` — add `#[derive(Serialize, specta::Type)]` (mirror `Asset` and `Holding`). Verify no `Deserialize` is needed (commands take primitives, not the struct).

#### 2.2.6 Module barrel — `mod.rs` and `domain/mod.rs`

The existing `pub use api::*; pub use domain::*; pub use repository::*; pub use service::*;` re-exports automatically expose the new types. No edit needed unless a sub-module is added.

#### 2.2.7 Specta builder — `core/specta_builder.rs`

Register the new types and commands:

```rust
.typ::<asset::AssetPrice>()                        // shared read type
.typ::<asset::UpdateAssetPriceCommandError>()
.typ::<asset::DeleteAssetPriceCommandError>()
// AssetPriceCommandError already registered — its new AssetNotFound variant flows through
.commands(tauri_specta::collect_commands![
    // … existing …
    asset::get_asset_prices,
    asset::update_asset_price,
    asset::delete_asset_price,
])
```

(B0c — only `core/specta_builder.rs` registers commands.)

#### 2.2.8 Backend tests (inline `#[cfg(test)] mod tests` per `docs/testing.md`)

To be authored by `test-writer-backend`. Stubs land in `src-tauri/src/context/asset/service.rs` (extend existing `mod tests`) and `src-tauri/src/context/asset/repository/asset_price.rs` (add a `mod tests` using `setup_pool` à la the existing service tests).

Service test stubs (one stub per behaviour, named after the rule):

- `get_prices_rejects_unknown_asset` — MKT-072
- `get_prices_returns_empty_when_no_records` — MKT-072 (asset exists, no prices)
- `get_prices_returns_records_sorted_date_descending` — MKT-072
- `update_price_rejects_when_record_not_found` — MKT-083 NotFound
- `update_price_rejects_non_positive` — MKT-082 / MKT-021
- `update_price_rejects_non_finite` — MKT-082
- `update_price_rejects_future_date` — MKT-082 / MKT-022
- `update_price_same_date_updates_in_place_and_publishes_event` — MKT-083 + MKT-085
- `update_price_date_changed_deletes_old_and_upserts_new_atomically` — MKT-084 (assert old date row gone, new date row present, single tx — verifiable by post-state)
- `update_price_date_changed_overwrites_existing_target_date` — MKT-084 collision
- `delete_price_rejects_when_record_not_found` — MKT-090 NotFound
- `delete_price_removes_record_and_publishes_event` — MKT-090 + MKT-091
- `record_price_unknown_asset_maps_to_asset_not_found_error` — MKT-043 (boundary mapper test, lives in `api.rs` or covered indirectly via service)

Repository test stubs (real SQLite, B27):

- `get_all_for_asset_returns_rows_sorted_date_descending`
- `get_by_asset_and_date_returns_none_when_missing`
- `delete_returns_zero_rows_affected_when_missing`
- `replace_atomic_rolls_back_on_insert_failure` (force a constraint or use a pre-condition; if not feasible, drop in favour of a positive test that asserts both old absent + new present)

Tests must be non-trivial (B25). Repository implementations use real SQLite (B27); service unit tests use the mockall-generated `MockAssetPriceRepository` (B26) for finer control where needed, but the existing pattern in `service.rs` uses real SQLite via `setup_pool` — keep that to stay consistent (preferred for atomicity assertions).

---

### 2.3 Frontend (`src/features/account_details/`)

Frontend lives entirely inside the `account_details` feature (use-case-centric layout per F1 and the kit memory note). No new feature module — the price history modal is opened from the holdings table, owned by the same use case as the `PriceModal` already in place.

#### 2.3.1 Gateway — `src/features/account_details/gateway.ts`

Add three methods (only file allowed to call `commands.*` for this feature, F3). Match `bindings.ts` positional signatures exactly.

```ts
async getAssetPrices(assetId: string): Promise<Result<AssetPrice[], AssetPriceCommandError>>
  → commands.getAssetPrices(assetId)

async updateAssetPrice(
  assetId: string,
  originalDate: string,
  newDate: string,
  newPrice: number,
): Promise<Result<null, UpdateAssetPriceCommandError>>
  → commands.updateAssetPrice(assetId, originalDate, newDate, newPrice)

async deleteAssetPrice(
  assetId: string,
  date: string,
): Promise<Result<null, DeleteAssetPriceCommandError>>
  → commands.deleteAssetPrice(assetId, date)
```

Imports come from `@/bindings`. No object-wrapping (Critical Pattern — bindings use positional args).

#### 2.3.2 Sub-feature — `src/features/account_details/price_history/`

New sub-feature directory. Files (gold layout, F1/F2):

```
price_history/
├── PriceHistoryModal.tsx           # the list modal (MKT-071, MKT-073, MKT-074, MKT-075, MKT-076, MKT-088, MKT-096)
├── usePriceHistory.ts              # fetch + mutate state (MKT-072, MKT-076, MKT-093)
├── usePriceHistory.test.ts         # tests
├── EditPriceForm.tsx               # inline edit form (MKT-080, MKT-081, MKT-087, MKT-094)
├── useEditPrice.ts                 # edit form state (MKT-082, MKT-086, MKT-087, MKT-094)
├── useEditPrice.test.ts
├── DeletePriceConfirmDialog.tsx    # wraps ConfirmationDialog (MKT-089)
└── PriceHistoryRow.tsx             # row presenter with Edit + Delete IconButtons (MKT-080, MKT-088)
```

Component responsibilities:

- **`PriceHistoryModal`** — receives `{ isOpen, onClose, assetId, assetName, assetCurrency }`. Hosts `usePriceHistory`, renders one of: spinner (MKT-074), inline error + retry (MKT-074), empty state with "Add price" CTA (MKT-073), populated list. Shows the existing `PriceModal` (re-used for "Add price" — MKT-075). Hosts the `EditPriceForm` and `DeletePriceConfirmDialog` modals when their target row is set. Renders the delete error banner at the top of the list when present (MKT-096).
- **`usePriceHistory`** — owns: `prices: AssetPrice[]`, `isLoading`, `fetchError`, `editTarget: AssetPrice | null`, `deleteTarget: AssetPrice | null`, `deleteError: string | null`, `deletingDate: string | null` (in-flight per row, MKT-093). Calls gateway on mount (MKT-072) and after each successful add/edit/delete (MKT-076). Returns `refetch`, `openEdit`, `openDelete`, `confirmDelete`, `closeEdit`, `closeDelete`. Subscribing to `AssetPriceUpdated` is **not** needed inside this hook — re-fetch is triggered explicitly after each mutation; the global `useAccountDetails` already subscribes (MKT-036) and refreshes the underlying view.
- **`EditPriceForm`** — pre-fills date + price from `editTarget`. Asset name + currency label read-only (MKT-081). Uses `useEditPrice` for state.
- **`useEditPrice`** — same validation predicates as `usePriceModal` (`validatePrice`, `validateDate`); both should be lifted to `shared/validatePrice.ts` for reuse (see 2.3.3). Calls `accountDetailsGateway.updateAssetPrice(assetId, originalDate, newDate, newPrice)` (MKT-082). On success: snackbar (MKT-086), close form, trigger parent `refetch`. On failure: keep open, show inline error keyed by `error.${result.error.code}` (MKT-087). In-flight: `isSubmitting` flag (MKT-094).
- **`DeletePriceConfirmDialog`** — wraps `ui/components/modal/ConfirmationDialog`. Identifies the target by date + formatted price (MKT-089). Calls back to `confirmDelete` from `usePriceHistory`.
- **`PriceHistoryRow`** — pure row presenter. Receives `{ row, currencyCode, isDeleting, onEdit, onDelete }`. Edit `IconButton` (`Pencil`), Delete `IconButton` (`Trash2`); delete disabled while `isDeleting === row.date` (MKT-093).

#### 2.3.3 Shared utilities — `src/features/account_details/shared/`

Reuse and extract:

- **`shared/validatePrice.ts`** — extract `validatePrice(price: string)` + `validateDate(date: string)` from `usePriceModal.ts`. `usePriceModal`, `useEditPrice` both import from here (DRY without changing behaviour). Pure functions, F5/F18 friendly.
- **`shared/presenter.ts`** — extend with `toPriceHistoryRow(price: AssetPrice, assetCurrency: string): PriceHistoryRowViewModel { date: string; formattedDate: string; formattedPrice: string }` so the UI never sees raw micros (F5).

#### 2.3.4 Entry point — `src/features/account_details/account_details_view/HoldingRow.tsx`

Add a fifth icon button on the active-holding row, alongside Buy / Sell / Enter price / Search (MKT-070). Suggested icon: `History` (lucide). Add a new `onPriceHistory: (assetId: string) => void` prop and wire it. Keep the button gated by `row.canEnterPrice` (active holdings only — MKT-070).

#### 2.3.5 Wiring — `src/features/account_details/account_details_view/AccountDetailsView.tsx`

Mirror the existing `priceTarget` pattern:

- New state: `historyTarget: { assetId: string; assetName: string; assetCurrency: string } | null`.
- New handlers: `handleOpenHistory`, `handleHistoryClose`.
- Pass `onPriceHistory={handleOpenHistory}` to `HoldingRow`.
- Render `<PriceHistoryModal isOpen … />` conditionally on `historyTarget`.
- Re-fetch of Account Details is automatic via `AssetPriceUpdated` (already subscribed by `useAccountDetails`, MKT-036). No extra subscription needed.

#### 2.3.6 i18n — `src/i18n/locales/{fr,en}/common.json`

New keys (under existing `account_details.*` and a new `price_history.*` namespace):

- `account_details.action_price_history` — tooltip on the new HoldingRow icon button (MKT-070)
- `price_history.title`, `price_history.empty_message`, `price_history.add_price`, `price_history.column_date`, `price_history.column_price`, `price_history.column_actions`, `price_history.fetch_error`, `price_history.retry`, `price_history.action_edit`, `price_history.action_delete`
- `price_history.edit_title`, `price_history.edit_submit`, `price_history.edit_success`, `price_history.delete_confirm_title`, `price_history.delete_confirm_message`, `price_history.delete_confirm_button`, `price_history.delete_success`, `price_history.delete_error_banner`
- `error.NotFound`, `error.AssetNotFound` (if not already present — verify and reuse)

Run `i18n-checker` at the docs phase to confirm no missing/extra keys.

#### 2.3.7 Frontend tests (Vitest + colocated, per `docs/testing.md`)

To be authored by `test-writer-frontend`. Module-level mock of `../gateway`. Stable references in `renderHook` (F19).

`usePriceHistory.test.ts`:

- `loads prices on mount and exposes them sorted as returned by the backend` (MKT-072)
- `surfaces fetch error code and exposes retry that re-calls gateway` (MKT-074)
- `refetches after a successful delete` (MKT-076)
- `tracks the deleting row so the row can show in-flight state` (MKT-093)
- `keeps the row and shows error banner when delete fails` (MKT-096)
- `clears delete error when a subsequent delete succeeds`

`useEditPrice.test.ts`:

- `pre-fills date and price from the target on mount` (MKT-081)
- `disables submit while either field is invalid` (MKT-082)
- `calls updateAssetPrice with original_date, new_date, new_price as numbers` (gateway argument verification)
- `shows snackbar, closes form and triggers refetch on success` (MKT-086)
- `keeps form open and surfaces error code on failure` (MKT-087)
- `disables submit while in-flight` (MKT-094)

No tests for pure DOM rendering (F18). No tests for the existing `usePriceModal` beyond what already exists; the validation helper extraction does not change behaviour.

---

### 2.4 Documentation Updates

- **`ARCHITECTURE.md`** — add the three new commands to the Asset Tauri commands list; add the three gateway methods to the Account Details gateway entry; mention the new `price_history/` sub-feature; note `AssetPrice` is now a frontend-visible Specta type. No event-bus table change (`AssetPriceUpdated` already registered).
- **`docs/todo.md`** — add resolved entries in English referencing the closed rules (MKT-070+ shipped). Log any deferred follow-up explicitly (e.g. "Consider extracting validatePrice/validateDate to a true shared/lib if a third caller emerges").
- **`docs/contracts/asset-contract.md`** — already reflects the new commands (last updated 2026-04-29). No edit needed unless `spec-checker` flags a drift.

---

### 2.5 Rules Coverage

| Rule            | Layer              | Task / File                                                                                                                                                                                                                                                             |
| --------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MKT-070         | Frontend           | New `History` IconButton in `account_details_view/HoldingRow.tsx`; wired to `onPriceHistory`                                                                                                                                                                            |
| MKT-071         | Frontend           | `PriceHistoryModal.tsx` renders the list; `presenter.toPriceHistoryRow` formats date + price in asset currency; backend already returns DESC order                                                                                                                      |
| MKT-072         | Backend            | `AssetService::get_prices` (asset existence guard → `AssetNotFound`); `SqliteAssetPriceRepository::get_all_for_asset` ordered DESC; `api::get_asset_prices`                                                                                                             |
| MKT-073         | Frontend           | Empty state branch in `PriceHistoryModal` with "Add price" CTA reusing the existing `PriceModal`                                                                                                                                                                        |
| MKT-074         | Frontend           | Loading + fetch-error branches in `PriceHistoryModal`; `usePriceHistory.refetch`                                                                                                                                                                                        |
| MKT-075         | Frontend           | `PriceHistoryModal` opens the existing `PriceModal` for the same asset; reuses MKT-020/029 path                                                                                                                                                                         |
| MKT-076         | Frontend           | `usePriceHistory.refetch` after successful add/edit/delete; `useAccountDetails` already refetches on `AssetPriceUpdated`                                                                                                                                                |
| MKT-080         | Frontend           | Edit `IconButton` in `PriceHistoryRow`                                                                                                                                                                                                                                  |
| MKT-081         | Frontend           | `EditPriceForm` — read-only asset name + currency, editable date + price, pre-filled                                                                                                                                                                                    |
| MKT-082         | Backend + Frontend | Frontend: `useEditPrice` reuses `validatePrice`/`validateDate` from `shared/validatePrice.ts`. Backend: `AssetService::update_price` builds `AssetPrice::new` (validates MKT-021/MKT-022); maps to `UpdateAssetPriceCommandError::{NotPositive,NonFinite,DateInFuture}` |
| MKT-083         | Backend            | `AssetService::update_price` — `get_by_asset_and_date(asset_id, original_date)` → `NotFound`; same-date branch reuses `upsert`                                                                                                                                          |
| MKT-084         | Backend            | `SqliteAssetPriceRepository::replace_atomic` (single sqlx transaction: delete + upsert) called when `original_date != new_date`                                                                                                                                         |
| MKT-085         | Backend            | `AssetService::update_price` publishes `Event::AssetPriceUpdated` after success                                                                                                                                                                                         |
| MKT-086         | Frontend           | `useEditPrice` — snackbar on success, close form, trigger refetch                                                                                                                                                                                                       |
| MKT-087         | Frontend           | `useEditPrice` — keep open, show inline error keyed by `result.error.code`                                                                                                                                                                                              |
| MKT-088         | Frontend           | Delete `IconButton` in `PriceHistoryRow`                                                                                                                                                                                                                                |
| MKT-089         | Frontend           | `DeletePriceConfirmDialog` (wraps `ui/components/modal/ConfirmationDialog`) shows date + formatted price                                                                                                                                                                |
| MKT-090         | Backend            | `AssetService::delete_price` — `rows_affected == 0` → `DeleteAssetPriceCommandError::NotFound`; `SqliteAssetPriceRepository::delete`                                                                                                                                    |
| MKT-091         | Backend            | `AssetService::delete_price` publishes `Event::AssetPriceUpdated` after success                                                                                                                                                                                         |
| MKT-092         | Frontend           | `usePriceHistory.confirmDelete` → snackbar + refetch                                                                                                                                                                                                                    |
| MKT-093         | Frontend           | `usePriceHistory.deletingDate` flag; `PriceHistoryRow` disables Delete while equal to its date                                                                                                                                                                          |
| MKT-094         | Frontend           | `useEditPrice.isSubmitting`; `EditPriceForm` submit button disabled + spinner                                                                                                                                                                                           |
| MKT-095         | Backend            | `update_asset_price` signature does not accept a new `asset_id`; `AssetPrice::new` is built with the original `asset_id` argument unchanged                                                                                                                             |
| MKT-096         | Frontend           | `usePriceHistory.deleteError` + banner at top of list in `PriceHistoryModal`; entry remains in list on failure                                                                                                                                                          |
| MKT-043 (retro) | Backend            | `to_asset_price_error` mapper now maps `AssetDomainError::NotFound` → `AssetPriceCommandError::AssetNotFound`                                                                                                                                                           |

---

### 2.6 Commit Checkpoints

1. **Backend** — once tests are green, types regenerated, FE compiles: `feat(asset): add price history list, update, delete commands`
2. **Frontend** — once history modal + edit + delete + tests pass: `feat(account-details): add price history modal with edit and delete`
3. **Tests & docs** — once `ARCHITECTURE.md` + `docs/todo.md` updated and `spec-checker` clean: `chore(market-price): document price history crud and update todo`

Each checkpoint runs `/smart-commit` (the assistant proposes the title, the user confirms; the kit handles the rest).

---

### 2.7 Implementation Discipline Reminder

The backend and frontend implementation tasks above must implement **only what is required to make the failing tests pass — no additional methods, no defensive code, no anticipation of future rules**. The `test-writer-backend` and `test-writer-frontend` agents define the scope; the implementation must not exceed it.
