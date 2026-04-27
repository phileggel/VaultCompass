# Implementation Plan -- Market Price Auto-Record from Transaction (MKT-050 to MKT-062)

Spec: `docs/spec/market-price.md` (rules MKT-050 through MKT-062, 13 rules)
Primary contract: `docs/contracts/record_transaction-contract.md` (CreateTransactionDTO gains `record_price: bool`; new "Side Effects" section; DbError annotated)
Secondary contract: `docs/contracts/asset-contract.md` (AssetPriceUpdated event row updated to list two triggers)
ADRs: ADR-001 (i64 micros), ADR-004 (use cases inject services not repositories)

---

## 1. Workflow TaskList

- [x] Spec (`/spec-writer`) -- `docs/spec/market-price.md` MKT-050..MKT-062 appended
- [x] Spec Review (`spec-reviewer`) -- 2 criticals raised, both resolved (atomicity/B8 seam + zero-price conflict)
- [x] Contract (`/contract`) -- `record_transaction-contract.md` + `asset-contract.md` updated
- [x] Contract Review (`contract-reviewer`) -- 0 critical, 1 doc-comment polish applied
- [x] Plan -- this document
- [x] Review Architecture & Rules (`ARCHITECTURE.md`, `backend-rules.md`, `frontend-rules.md`)
- [x] Backend test stubs (`test-writer-backend` -- all stubs written, red confirmed)
- [x] Backend Implementation (minimal -- make failing tests pass, green confirmed)
- [x] `just format` (rustfmt + clippy --fix)
- [x] Backend Review (`reviewer-backend` -- fix issues)
- [x] Type Synchronization (`just generate-types`)
- [x] Compilation fixup (TypeScript errors from new bindings only -- no UI work)
- [x] `just check` -- TypeScript clean
- [x] Commit: `feat(record-transaction): auto-record asset price from transaction` (117e618)
- [x] Frontend test stubs (`test-writer-frontend` -- all stubs written, red confirmed)
- [x] Frontend Implementation (minimal -- make failing tests pass, green confirmed)
- [x] `just format`
- [x] Frontend Review (`reviewer-frontend` -- fix issues)
- [x] Commit: `feat(transactions): add auto-record price checkbox and settings toggle` (50f21bd)
- [x] Cross-cutting Review (`reviewer` always) -- 0 issues across 14 files
- [x] i18n Review (`i18n-checker` -- UI text changed) -- 3 keys × 2 locales, all wired, no orphans
- [x] Documentation Update (`ARCHITECTURE.md` + `docs/todo.md` -- entries in English)
- [x] Spec check (`spec-checker`) -- 13/13 implemented, 11/13 fully tested (MKT-056/062 deferred — fault injection seam tracked as separate todo)
- [ ] Commit: `docs(market-price): update architecture for auto-record feature`

---

## 2. Detailed Implementation Plan

### 2.0 Migrations

None required. The `asset_prices` table already exists (`src-tauri/migrations/202604260001_create_asset_prices.sql`) with the composite PK `(asset_id, date)` and `INSERT OR REPLACE` / `ON CONFLICT ... DO UPDATE` upsert semantics. The auto-record path reuses the same table and upsert logic.

---

### 2.1 Backend -- Add `record_price` field to `CreateTransactionDTO`

**File**: `src-tauri/src/use_cases/record_transaction/orchestrator.rs`

Extend the `CreateTransactionDTO` struct with:

```rust
/// MKT-054 -- when true and unit_price > 0, the orchestrator also upserts
/// AssetPrice(asset_id, date, unit_price) inside the same DB tx (MKT-055/056)
/// and publishes AssetPriceUpdated after commit (MKT-057).
pub record_price: bool,
```

This field is Specta-derived (`#[derive(Type)]` already present), so it will flow through to `bindings.ts` on `just generate-types`.

**Rules covered**: MKT-054

---

### 2.2 Backend -- Inject `AssetService` into `RecordTransactionUseCase`

**File**: `src-tauri/src/use_cases/record_transaction/orchestrator.rs`

The orchestrator currently has no access to `AssetService`. It needs it to call `notify_asset_price_updated()` after commit (MKT-057). Add a new field:

```rust
asset_service: Arc<AssetService>,
```

Update `RecordTransactionUseCase::new()` to accept `asset_service: Arc<AssetService>` as a sixth parameter and store it.

**File**: `src-tauri/src/lib.rs`

Update the `RecordTransactionUseCase::new()` call (around line 151) to pass `Arc::clone(&asset_service)` as the sixth argument. Import is already present via `use crate::context::asset::AssetService`.

**Rules covered**: MKT-057 (dependency injection for event publication)

---

### 2.3 Backend -- Add `notify_asset_price_updated()` to `AssetService`

**File**: `src-tauri/src/context/asset/service.rs`

Add a new public method:

```rust
/// Publishes AssetPriceUpdated without performing any write.
/// Called by the record_transaction use case after an atomic DB commit (MKT-057, B8).
pub fn notify_asset_price_updated(&self) {
    if let Some(bus) = &self.event_bus {
        bus.publish(Event::AssetPriceUpdated);
    }
}
```

This mirrors the `TransactionService::notify_transaction_updated()` pattern (see `src-tauri/src/context/transaction/service.rs` line 92-94). The method ONLY publishes the event -- no DB write. The actual upsert happens inside the orchestrator's DB transaction via raw `sqlx::query!`.

**Rules covered**: MKT-057

---

### 2.4 Backend -- Auto-record logic in orchestrator's `create_purchase` and `create_sell`

**File**: `src-tauri/src/use_cases/record_transaction/orchestrator.rs`

#### 2.4.1 Inside `create_purchase` (around line 139-174)

After the `self.upsert_holding_in_tx(&mut db_tx, &holding).await?;` call and BEFORE the `db_tx.commit()`, add:

```rust
// MKT-055 -- auto-record AssetPrice inside the same DB transaction
let price_written = if dto.record_price && dto.unit_price > 0 {
    sqlx::query!(
        "INSERT INTO asset_prices (asset_id, date, price) VALUES (?, ?, ?)
         ON CONFLICT(asset_id, date) DO UPDATE SET price = excluded.price",
        dto.asset_id,
        dto.date,
        dto.unit_price,
    )
    .execute(&mut *db_tx)
    .await
    .context("Failed to upsert asset price (MKT-055)")?;
    true
} else {
    false
};
```

After the `db_tx.commit()`, and after `self.transaction_service.notify_transaction_updated()`, add:

```rust
// MKT-057 -- publish AssetPriceUpdated after commit, only if a price was written
if price_written {
    self.asset_service.notify_asset_price_updated();
}
```

#### 2.4.2 Inside `create_sell` (around line 259-277)

Same pattern as above: after `self.upsert_holding_in_tx` and before `db_tx.commit()`, add the conditional `sqlx::query!` upsert. After commit and `notify_transaction_updated()`, conditionally call `notify_asset_price_updated()`.

#### 2.4.3 Inside `update_transaction` (around line 430-477)

Same pattern: after the `self.upsert_holding_in_tx(&mut db_tx, &new_holding).await?;` (and the old-holding cleanup block), before `db_tx.commit()`, add the conditional upsert. After commit, conditionally publish.

The `dto.date` and `dto.unit_price` used are the CURRENT values from the DTO, consistent with MKT-059 (edit lifecycle -- price independence: only the current date/price is targeted).

**Important**: `delete_transaction` does NOT auto-record and does NOT touch `AssetPrice` rows (MKT-060). No changes to `delete_transaction`.

**Implementation constraint**: implement only what is required to make the failing tests pass -- no additional methods, no defensive code, no anticipation of future rules.

**Rules covered**: MKT-055, MKT-056, MKT-057, MKT-058, MKT-059, MKT-060, MKT-061, MKT-062

---

### 2.5 Backend -- Update test helpers

**File**: `src-tauri/src/use_cases/record_transaction/orchestrator.rs` (in `#[cfg(test)] mod tests`)

The existing `buy_dto()` and `sell_dto()` helper functions must be updated to include the `record_price: false` field (since the struct gains a new required field). Without this change, all existing tests will fail to compile.

The `setup_uc()` helper must be updated to pass `Arc::clone(&asset_service)` as the sixth argument to `RecordTransactionUseCase::new()`. This requires creating an `AssetService` instance in the test setup (following the same pattern as in `src-tauri/src/context/asset/service.rs` tests -- using `SqliteAssetRepository`, `SqliteAssetCategoryRepository`, `SqliteAssetPriceRepository`).

**Rules covered**: compilation correctness

---

### 2.6 Backend -- New tests for auto-record

**File**: `src-tauri/src/use_cases/record_transaction/orchestrator.rs` (in `#[cfg(test)] mod tests`)

New test cases (to be written by `test-writer-backend`):

1. **MKT-055 -- create_purchase with record_price=true writes AssetPrice**: Create a purchase with `record_price: true`. Assert that `asset_prices` table contains a row with `(asset_id, date, price == unit_price)`.

2. **MKT-055 -- create_sell with record_price=true writes AssetPrice**: Create a buy (to have holding), then a sell with `record_price: true`. Assert the price row exists at the sell's date/unit_price.

3. **MKT-055 -- update_transaction with record_price=true writes AssetPrice**: Create a buy with `record_price: false`, then update it with `record_price: true`. Assert the price row exists.

4. **MKT-055/054 -- record_price=false does NOT write AssetPrice**: Create a purchase with `record_price: false`. Assert `asset_prices` table has no rows for that asset.

5. **MKT-058 -- silent overwrite on same-date collision**: Pre-insert an `AssetPrice` row at `(asset_id, date)`, then create a transaction on the same date with `record_price: true` and a different `unit_price`. Assert the row is overwritten.

6. **MKT-061 -- zero unit_price skips the write**: Create a purchase with `unit_price: 0` and `record_price: true`. Assert no `AssetPrice` row is created.

7. **MKT-056 -- atomicity on failure**: Force a failure after the price write (e.g., by using invalid data that fails a later step). Assert that neither the transaction nor the price row is persisted. (This may be hard to test in isolation -- if the test-writer judges it untestable without mocking infrastructure, it can be skipped with a comment.)

8. **MKT-059 -- edit lifecycle**: Create a purchase at date "2024-01-01" with `record_price: true`. Update it to date "2024-06-01" with `record_price: true`. Assert the old date's price row still exists AND a new one at "2024-06-01" is created.

9. **MKT-060 -- delete does not cascade**: Create a purchase with `record_price: true`. Delete the transaction. Assert the `AssetPrice` row at that date is still present.

**Rules covered**: MKT-054, MKT-055, MKT-056, MKT-058, MKT-059, MKT-060, MKT-061

---

### 2.7 Backend -- `just generate-types`

After all Rust changes compile, run:

```bash
just generate-types
```

This regenerates `src/bindings.ts` with:

- Updated `CreateTransactionDTO` type gaining `record_price: boolean`

All existing frontend code that constructs `CreateTransactionDTO` will now fail TypeScript compilation because the new required field is missing. This is expected and is addressed in the compilation fixup step.

---

### 2.8 Frontend -- Compilation fixup (TypeScript errors from new bindings only)

All places constructing `CreateTransactionDTO` need the new `record_price` field. As a compilation fixup (not UI work), add `record_price: false` to every call site:

**Files to update** (each constructs the DTO object):

1. `src/features/account_details/buy_transaction/useBuyTransaction.ts` -- in `doSubmit()` around line 77-87, add `record_price: false` to the object passed to `addTransaction()`.

2. `src/features/account_details/sell_transaction/useSellTransaction.ts` -- in `handleSubmit()` around line 90-99, add `record_price: false` to the object passed to `addTransaction()`.

3. `src/features/transactions/add_transaction/useAddTransaction.ts` -- in `doSubmit()` around line 89-99, add `record_price: false` to the object passed to `addTransaction()`.

4. `src/features/transactions/edit_transaction_modal/useEditTransactionModal.ts` -- in `doSubmit()` around line 99-109, add `record_price: false` to the object passed to `updateTransaction()`.

These are all temporary `false` defaults to make `just check` pass. The frontend implementation phase (below) replaces them with the actual checkbox state.

**Rules covered**: compilation correctness only

---

### 2.9 Frontend -- Settings: global auto-record toggle

#### 2.9.1 localStorage helpers

**File**: `src/features/settings/useSettings.ts`

Add a new localStorage key constant and getter/setter pair, following the same pattern as `getLanguageOverride`/`setLanguageOverride` in `src/i18n/config.ts`:

```typescript
const AUTO_RECORD_PRICE_KEY = "auto_record_price";

export function getAutoRecordPrice(): boolean {
  return localStorage.getItem(AUTO_RECORD_PRICE_KEY) === "true";
}

export function setAutoRecordPrice(enabled: boolean): void {
  localStorage.setItem(AUTO_RECORD_PRICE_KEY, String(enabled));
}
```

Extend the `useSettings` hook return value with:

```typescript
autoRecordPrice: boolean;          // current state
toggleAutoRecordPrice: () => void; // toggle handler
```

State is initialized from `getAutoRecordPrice()` (default `false` when key is absent -- MKT-050 "defaults to OFF"). The toggle calls `setAutoRecordPrice(!current)` and updates local state.

#### 2.9.2 Settings page UI

**File**: `src/features/settings/SettingsPage.tsx`

Add a new `<section>` after the language section, containing a labeled toggle/checkbox:

- Label: `t("settings.auto_record_price_label")`
- Description: `t("settings.auto_record_price_description")`
- Control: a checkbox input bound to `autoRecordPrice` / `toggleAutoRecordPrice`

**Rules covered**: MKT-050

---

### 2.10 Frontend -- Per-transaction checkbox state in hooks

Each transaction form hook must gain:

- `recordPrice: boolean` state
- `setRecordPrice: (value: boolean) => void` callback
- Initialization logic: create mode reads from `getAutoRecordPrice()` at form open (MKT-052, MKT-053); edit mode always starts `false` (MKT-052)
- The `record_price` field in the DTO passed to the gateway uses the `recordPrice` state value

#### 2.10.1 `useBuyTransaction` (create mode, modal)

**File**: `src/features/account_details/buy_transaction/useBuyTransaction.ts`

- Add `useState<boolean>` initialized from `getAutoRecordPrice()` (MKT-052 create default).
- In `doSubmit()`, change `record_price: false` to `record_price: recordPrice`.
- Return `recordPrice` and `setRecordPrice` from the hook.

#### 2.10.2 `useSellTransaction` (create mode, modal)

**File**: `src/features/account_details/sell_transaction/useSellTransaction.ts`

Same changes as 2.10.1.

#### 2.10.3 `useAddTransaction` (create mode, standalone page)

**File**: `src/features/transactions/add_transaction/useAddTransaction.ts`

Same changes as 2.10.1. The standalone page uses this hook for both buy and sell creation flows.

#### 2.10.4 `useEditTransactionModal` (edit mode)

**File**: `src/features/transactions/edit_transaction_modal/useEditTransactionModal.ts`

- Add `useState<boolean>(false)` -- always OFF in edit mode (MKT-052).
- In `doSubmit()`, change `record_price: false` to `record_price: recordPrice`.
- Return `recordPrice` and `setRecordPrice` from the hook.

**Rules covered**: MKT-051, MKT-052, MKT-053, MKT-054

---

### 2.11 Frontend -- Per-transaction checkbox in form components

Each form component must render the checkbox between the last data field and the submit button.

#### 2.11.1 `BuyTransactionModal`

**File**: `src/features/account_details/buy_transaction/BuyTransactionModal.tsx`

- Destructure `recordPrice` and `setRecordPrice` from `useBuyTransaction`.
- Add a checkbox `<label>` after the note field and before the error `<p>`, containing:
  - `<input type="checkbox" checked={recordPrice} onChange={(e) => setRecordPrice(e.target.checked)} />`
  - Label text: `t("transaction.auto_record_price_label", { date: formData.date })` -- reflects MKT-051's live date update.

#### 2.11.2 `SellTransactionModal`

**File**: `src/features/account_details/sell_transaction/SellTransactionModal.tsx`

Same checkbox pattern as 2.11.1.

#### 2.11.3 `AddTransactionPage` (standalone)

**File**: `src/features/transactions/add_transaction_page/AddTransactionPage.tsx`

Same checkbox pattern as 2.11.1, placed after the note `<TextareaField>` and before the error message.

#### 2.11.4 `EditTransactionModal`

**File**: `src/features/transactions/edit_transaction_modal/EditTransactionModal.tsx`

Same checkbox pattern as 2.11.1.

**Rules covered**: MKT-051, MKT-052

---

### 2.12 Frontend -- i18n keys

**Files**: `src/i18n/locales/en/common.json`, `src/i18n/locales/fr/common.json`

New keys in the `settings` group:

```json
"auto_record_price_label": "Automatically record transaction price as market price",
"auto_record_price_description": "When enabled, new transaction forms will have the auto-record checkbox pre-checked."
```

New key in the `transaction` group:

```json
"auto_record_price_label": "Use this price as the market price for {{date}}"
```

French translations:

```json
// settings
"auto_record_price_label": "Enregistrer automatiquement le prix de transaction comme prix de marche",
"auto_record_price_description": "Si active, les nouveaux formulaires de transaction auront la case auto-enregistrement pre-cochee."

// transaction
"auto_record_price_label": "Utiliser ce prix comme prix de marche pour le {{date}}"
```

**Rules covered**: MKT-050, MKT-051, F16

---

### 2.13 Frontend -- Hook tests

Tests to be written by `test-writer-frontend`:

#### 2.13.1 `useBuyTransaction` tests

**File**: `src/features/account_details/buy_transaction/useBuyTransaction.test.ts` (new)

- **MKT-052** -- `recordPrice` defaults to `getAutoRecordPrice()` value on create
- **MKT-053** -- snapshot semantics: changing `localStorage` after hook mount does not change `recordPrice`
- **MKT-054** -- submitting with `recordPrice: true` passes `record_price: true` to `addTransaction`
- **MKT-054** -- submitting with `recordPrice: false` passes `record_price: false` to `addTransaction`

#### 2.13.2 `useSellTransaction` tests

**File**: `src/features/account_details/sell_transaction/useSellTransaction.test.ts` (extend existing)

Same test patterns as 2.13.1, adapted for sell.

#### 2.13.3 `useAddTransaction` tests

**File**: `src/features/transactions/add_transaction/useAddTransaction.test.ts` (extend existing)

Same test patterns as 2.13.1.

#### 2.13.4 `useEditTransactionModal` tests

**File**: `src/features/transactions/edit_transaction_modal/useEditTransactionModal.test.ts` (extend existing)

- **MKT-052** -- `recordPrice` defaults to `false` in edit mode regardless of `localStorage`

#### 2.13.5 `useSettings` tests

**File**: `src/features/settings/useSettings.test.ts` (new)

- **MKT-050** -- `autoRecordPrice` defaults to `false` when localStorage key is absent
- **MKT-050** -- `toggleAutoRecordPrice` toggles the value and persists it
- **MKT-050** -- `autoRecordPrice` reflects `true` when localStorage key is `"true"`

**Rules covered**: MKT-050, MKT-052, MKT-053, MKT-054

---

## 3. Rules Coverage

| Rule    | Scope    | Description                                     | Implementation Task                         | Test Phase        |
| ------- | -------- | ----------------------------------------------- | ------------------------------------------- | ----------------- |
| MKT-050 | frontend | Global auto-record toggle in Settings           | 2.9 (useSettings, SettingsPage)             | test-writer-fe    |
| MKT-051 | frontend | Per-transaction checkbox in buy/sell forms      | 2.10, 2.11 (hooks + components)             | test-writer-fe    |
| MKT-052 | frontend | Checkbox default: toggle on create, OFF on edit | 2.10 (hooks init logic)                     | test-writer-fe    |
| MKT-053 | frontend | Snapshot semantics at form open                 | 2.10 (hooks init logic)                     | test-writer-fe    |
| MKT-054 | both     | Submit payload: record_price bool in DTO        | 2.1 (DTO), 2.10 (hooks), 2.8 (fixup)        | test-writer-be/fe |
| MKT-055 | backend  | Auto-write inside orchestrator's DB transaction | 2.4 (orchestrator)                          | test-writer-be    |
| MKT-056 | backend  | Atomicity: full rollback if any step fails      | 2.4 (orchestrator)                          | test-writer-be    |
| MKT-057 | backend  | AssetPriceUpdated event after commit            | 2.3 (AssetService), 2.4 (orchestrator)      | test-writer-be    |
| MKT-058 | backend  | Silent overwrite on same-date collision         | 2.4 (orchestrator upsert)                   | test-writer-be    |
| MKT-059 | backend  | Edit lifecycle: price independence              | 2.4 (update_transaction)                    | test-writer-be    |
| MKT-060 | backend  | Delete lifecycle: price independence            | no code change (implicit)                   | test-writer-be    |
| MKT-061 | backend  | Zero unit_price skip                            | 2.4 (conditional check)                     | test-writer-be    |
| MKT-062 | both     | Auto-record failure surfaces as tx error        | 2.4 (error propagation + rollback, MKT-056) | test-writer-be    |

---

## 4. Commit Checkpoints

### Checkpoint 1: Backend layer

**Suggested title**: `feat(record-transaction): auto-record asset price from transaction`
**Scope**: `CreateTransactionDTO` gains `record_price: bool`, `RecordTransactionUseCase` gains `AssetService` injection, orchestrator methods gain auto-record logic, `AssetService::notify_asset_price_updated()`, `lib.rs` wiring update, backend tests, existing test helper updates, `just generate-types`, TypeScript compilation fixup (`record_price: false` defaults).

### Checkpoint 2: Frontend layer

**Suggested title**: `feat(transactions): add auto-record price checkbox and settings toggle`
**Scope**: `useSettings` gains auto-record state, `SettingsPage` gains toggle, four transaction hooks gain `recordPrice` state, four form components gain checkbox, i18n keys (en + fr), frontend tests.

### Checkpoint 3: Tests and docs

**Suggested title**: `docs(market-price): update architecture for auto-record feature`
**Scope**: `ARCHITECTURE.md` updates (RecordTransactionUseCase gains AssetService injection, CreateTransactionDTO gains record_price, orchestrator description extended with auto-record side-effect, AssetPriceUpdated event table gains second trigger source, Settings feature description extended), `docs/todo.md` update (close the "(market-price) -- Opt-in: use transaction unit_price as market price" entry).

---

## 5. Phase 4 Closure Tasks

- [ ] `reviewer` -- cross-cutting review (always)
- [ ] `i18n-checker` -- UI text changed (new i18n keys in en + fr)
- [ ] `ARCHITECTURE.md` update:
  - Record Transaction use case section: mention `AssetService` injection and auto-record side-effect
  - `CreateTransactionDTO` description: add `record_price: bool` field
  - Event bus table: update `AssetPriceUpdated` row to mention both triggers (manual `record_asset_price` and auto-record via `add_transaction`/`update_transaction`)
  - Settings feature section: mention auto-record toggle
- [ ] `docs/todo.md` update: mark "(market-price) -- Opt-in: use transaction unit_price as market price" as resolved
- [ ] `spec-checker`
