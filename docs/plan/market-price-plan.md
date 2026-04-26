# Implementation Plan -- Market Price (MKT)

Spec: `docs/spec/market-price.md` (MKT-010 through MKT-043, 22 rules)
Contracts: `docs/contracts/asset-contract.md` (new: `record_asset_price`), `docs/contracts/account_details-contract.md` (extended: HoldingDetail +5 fields, AccountDetailsResponse +1 field, AssetPriceUpdated subscription)
ADRs: ADR-001 (i64 micros), ADR-004 (use cases inject services not repositories)

---

## 1. Workflow TaskList

- [ ] Review Architecture & Rules (`ARCHITECTURE.md`, `backend-rules.md`, `frontend-rules.md`)
- [ ] Database Migration (`just migrate` + `just prepare-sqlx`)
- [x] Contract (`/contract` -- human approves shape)
- [x] Contract Review (`contract-reviewer` -- fix issues)
- [ ] Backend test stubs (`test-writer-backend` -- all stubs written, red confirmed)
- [ ] Backend Implementation (minimal -- make failing tests pass, green confirmed)
- [ ] `just format` (rustfmt + clippy --fix)
- [ ] Backend Review (`reviewer-backend` -- fix issues)
- [ ] Type Synchronization (`just generate-types`)
- [ ] Compilation fixup (TypeScript errors from new bindings only -- no UI work)
- [ ] `just check` -- TypeScript clean
- [ ] Commit: `feat(asset): implement market price recording backend`
- [ ] Frontend test stubs (`test-writer-frontend` -- all stubs written, red confirmed)
- [ ] Frontend Implementation (minimal -- make failing tests pass, green confirmed)
- [ ] `just format`
- [ ] Frontend Review (`reviewer-frontend` -- fix issues)
- [ ] Commit: `feat(account-details): display market price and unrealized P&L`
- [ ] Cross-cutting Review (`reviewer` always + `reviewer-sql` for migration)
- [ ] i18n Review (`i18n-checker` -- UI text changed)
- [ ] Documentation Update (`ARCHITECTURE.md` + `docs/todo.md` -- entries in English)
- [ ] Spec check (`spec-checker`)
- [ ] Commit: `docs(market-price): update architecture and close spec items`

---

## 2. Detailed Implementation Plan

### 2.0 Migration -- `asset_prices` table

A previous `asset_prices` table was dropped in `202604160001_drop_asset_prices.sql`. This migration re-creates it with the correct schema (i64 micros, unique composite key).

**File**: `src-tauri/migrations/202604260001_create_asset_prices.sql`

```sql
CREATE TABLE asset_prices (
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    date     TEXT NOT NULL,
    price    INTEGER NOT NULL,
    PRIMARY KEY (asset_id, date)
);
```

Notes:

- `price` is `INTEGER` (i64 micros, ADR-001). No separate `id` column -- the composite PK `(asset_id, date)` is the identity.
- `ON DELETE CASCADE` ensures prices are cleaned up when an asset is hard-deleted.
- After creating this file, run `just migrate` then `just prepare-sqlx` before writing any Rust code that queries this table.
- No `BEGIN`/`COMMIT` in the migration (sqlx wraps each in a transaction).

**Rules covered**: MKT-024, MKT-025, MKT-042

---

### 2.1 Backend -- `AssetPrice` domain entity

**File**: `src-tauri/src/context/asset/domain/asset_price.rs` (new)

Define the `AssetPrice` entity:

```
AssetPrice {
    asset_id: String,
    date: String,       // ISO 8601 "YYYY-MM-DD"
    price: i64,         // micro-units (ADR-001)
}
```

- Derive: `Debug, Serialize, Deserialize, Clone`
- No Specta `Type` derive needed -- this entity is internal to the asset context and never crosses the IPC boundary directly. The `HoldingDetail` DTO exposes derived values (`current_price`, etc.) instead.

Factory methods (per B1):

- `new(asset_id: String, date: String, price: i64) -> Result<Self>` -- validates `price > 0` (MKT-021), validates `date` is a well-formed ISO date not in the future (MKT-022). Date validation: parse with `chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")`, reject `Err`; compare against `chrono::Local::now().date_naive()`, reject if `parsed > today`.
- `restore(asset_id: String, date: String, price: i64) -> Self` -- no validation (B1 `restore` convention).

Error messages (matching contract error names):

- `price > 0` fails: `"Price must be strictly positive"` (maps to `PriceNotPositive`)
- date parse fails: `"Invalid date format — expected YYYY-MM-DD"` (maps to `InvalidDate`)
- date in future: `"Date cannot be in the future"` (maps to `FutureDate`)

**File**: `src-tauri/src/context/asset/domain/mod.rs`

Add `mod asset_price;` and `pub use asset_price::*;`.

**File**: `src-tauri/src/context/asset/mod.rs`

No change needed -- already re-exports `domain::*`.

**Dependencies**: `chrono` crate. Check if already in `Cargo.toml`; add `chrono = "0.4"` if missing.

**Rules covered**: MKT-021, MKT-022, MKT-024

---

### 2.2 Backend -- `AssetPriceRepository` trait + SQLite implementation

**Trait file**: `src-tauri/src/context/asset/domain/asset_price.rs` (same file as entity, following the `holding.rs` pattern)

```rust
#[async_trait]
pub trait AssetPriceRepository: Send + Sync {
    /// Upserts a price for (asset_id, date). Overwrites if exists (MKT-025).
    async fn upsert(&self, price: AssetPrice) -> Result<()>;
    /// Returns the most recently dated price for the given asset, if any (MKT-031).
    async fn get_latest(&self, asset_id: &str) -> Result<Option<AssetPrice>>;
}
```

**Implementation file**: `src-tauri/src/context/asset/repository/asset_price.rs` (new)

`SqliteAssetPriceRepository`:

- `new(pool: SqlitePool) -> Self`
- `upsert`: `INSERT INTO asset_prices (asset_id, date, price) VALUES (?, ?, ?) ON CONFLICT(asset_id, date) DO UPDATE SET price = excluded.price` (MKT-025). Uses `sqlx::query!`.
- `get_latest`: `SELECT asset_id, date, price FROM asset_prices WHERE asset_id = ? ORDER BY date DESC LIMIT 1`. Returns `Option<AssetPrice>` via `restore`.

**File**: `src-tauri/src/context/asset/repository/mod.rs`

Add `mod asset_price;` and `pub use asset_price::SqliteAssetPriceRepository;`.

**Rules covered**: MKT-025, MKT-031

---

### 2.3 Backend -- `AssetService` gains price methods

**File**: `src-tauri/src/context/asset/service.rs`

Extend `AssetService`:

- Add field: `price_repo: Box<dyn AssetPriceRepository>` (trait from domain)
- Update `AssetService::new()` signature: add `price_repo: Box<dyn AssetPriceRepository>` as third parameter
- Add method: `record_price(&self, asset_id: &str, date: &str, price_f64: f64) -> Result<()>`
  1. Validate asset exists: `self.asset_repo.get_by_id(asset_id)` -- if `None`, bail with `"Asset not found"` (MKT-043)
  2. Convert f64 to i64 micros: `let price_micros = (price_f64 * 1_000_000.0).round() as i64;` (MKT-024)
  3. Construct entity: `AssetPrice::new(asset_id.to_string(), date.to_string(), price_micros)?` -- validation runs here (MKT-021, MKT-022)
  4. Upsert: `self.price_repo.upsert(price).await?` (MKT-025)
  5. Publish event: `Event::AssetPriceUpdated` via `self.event_bus` (MKT-026)
- Add method: `get_latest_price(&self, asset_id: &str) -> Result<Option<AssetPrice>>`
  - Delegates to `self.price_repo.get_latest(asset_id)`
  - Used by `AccountDetailsUseCase` (ADR-004)

**File**: `src-tauri/src/lib.rs`

Update `AssetService::new()` call to pass a third argument:

```
Box::new(SqliteAssetPriceRepository::new(db.pool.clone()))
```

Import `SqliteAssetPriceRepository` from `crate::context::asset`.

**Rules covered**: MKT-021, MKT-022, MKT-024, MKT-025, MKT-026, MKT-031, MKT-043

---

### 2.4 Backend -- `AssetPriceUpdated` event variant

**File**: `src-tauri/src/core/event_bus/event.rs`

Add variant to the `Event` enum:

```rust
/// An asset price was recorded or updated
AssetPriceUpdated,
```

This is a bare signal event with no payload, consistent with all existing variants (MKT-026, MKT-037).

**Rules covered**: MKT-026, MKT-037

---

### 2.5 Backend -- `record_asset_price` Tauri command

**File**: `src-tauri/src/context/asset/api.rs`

Add a new Tauri command:

```rust
/// Records (or overwrites) a market price for an asset on a given date (MKT-024/025).
#[tauri::command]
#[specta::specta]
pub async fn record_asset_price(
    state: State<'_, AppState>,
    asset_id: String,
    date: String,
    price: f64,
) -> Result<(), String> {
    state
        .asset_service
        .record_price(&asset_id, &date, price)
        .await
        .map_err(|e| e.to_string())
}
```

Note: `price` arrives as `f64` on the IPC boundary. The f64-to-i64 conversion happens inside `AssetService::record_price` (MKT-024). The three args are positional primitives matching the asset contract: `asset_id: String, date: String, price: f64`.

**File**: `src-tauri/src/core/specta_builder.rs`

Register the command in `collect_commands![]`:

```
asset::record_asset_price,
```

Place it after the existing `asset::delete_asset` line.

**Rules covered**: MKT-024, MKT-025, MKT-043, B0c, B5

---

### 2.6 Backend -- Extend `HoldingDetail` and `AccountDetailsResponse`

**File**: `src-tauri/src/use_cases/account_details/orchestrator.rs`

Extend `HoldingDetail` struct with five new fields (per contract):

```rust
/// ISO 4217 currency code of the asset's native currency (MKT-023).
pub asset_currency: String,
/// Most recently dated price for this asset in asset currency (i64 micros). None if no price recorded (MKT-031).
pub current_price: Option<i64>,
/// ISO date string of the price observation. None when current_price is None (MKT-031).
pub current_price_date: Option<String>,
/// Unrealized gain/loss in account currency (i64 micros). None on currency mismatch or no price (MKT-033/034).
pub unrealized_pnl: Option<i64>,
/// Performance percentage as i64 micros (5.25% = 5_250_000). None when unrealized_pnl is None or cost_basis = 0 (MKT-035).
pub performance_pct: Option<i64>,
```

Extend `AccountDetailsResponse` struct with:

```rust
/// Sum of unrealized_pnl across all same-currency priced active holdings. None when none qualify (MKT-040).
pub total_unrealized_pnl: Option<i64>,
```

**Rules covered**: MKT-023, MKT-030, MKT-031, MKT-033, MKT-034, MKT-035, MKT-040

---

### 2.7 Backend -- Extend `AccountDetailsUseCase::get_account_details`

**File**: `src-tauri/src/use_cases/account_details/orchestrator.rs`

In the `get_account_details` method, after the existing active-holding enrichment loop, add price-fetching and P&L computation logic:

For each active holding in the enrichment loop:

1. Fetch `asset.currency` (already available from the `get_asset_by_id` call).
2. Fetch latest price: `self.asset_service.get_latest_price(&holding.asset_id).await`. Wrap in a match/ok to swallow errors gracefully (MKT-031: "A failure in the price lookup does not abort the overall response; it degrades gracefully by returning None").
3. Populate `asset_currency` from `asset.currency`.
4. If latest price exists, set `current_price = Some(price.price)` and `current_price_date = Some(price.date)`.
5. If latest price exists AND `asset.currency == account.currency` (MKT-033/034):
   - Compute `unrealized_pnl = ((current_price - average_price) as i128 * quantity as i128 / 1_000_000) as i64` using i128 intermediates (MKT-033, consistent with ACD-024).
   - If `cost_basis != 0`: `performance_pct = (unrealized_pnl as i128 * 100_000_000 / cost_basis as i128) as i64` -- this yields i64 micros (MKT-035). Note: `100 * 1_000_000 = 100_000_000`.
   - If `cost_basis == 0`: `performance_pct = None`.
6. If currency mismatch or no price: `unrealized_pnl = None`, `performance_pct = None`.

The `AccountDetailsUseCase` needs access to the account's currency. The `account` variable already exists from the earlier fetch and `Account.currency` is available.

After the loop, compute `total_unrealized_pnl` (MKT-040):

- Collect all `unrealized_pnl` values that are `Some(value)`.
- If any exist, sum them to produce `Some(sum)`. Otherwise, `None`.

Include the new field in the returned `AccountDetailsResponse`.

**Implementation note**: implement only what is required to make the failing tests pass -- no additional methods, no defensive code, no anticipation of future rules.

**Rules covered**: MKT-031, MKT-033, MKT-034, MKT-035, MKT-040

---

### 2.8 Backend -- `just generate-types`

After all Rust changes compile, run:

```bash
just generate-types
```

This regenerates `src/bindings.ts` with:

- New command: `recordAssetPrice(assetId: string, date: string, price: number)`
- Updated `HoldingDetail` type with five new fields
- Updated `AccountDetailsResponse` type with `total_unrealized_pnl`
- Updated `Event` type with `AssetPriceUpdated` variant

---

### 2.9 Frontend -- Gateway: add `recordAssetPrice`

**File**: `src/features/account_details/gateway.ts`

Add a new gateway method for recording a price. The `record_asset_price` command belongs to the `asset` bounded context, but is called from the Account Details feature's PriceModal. Per F3, the gateway at the feature root is the only file allowed to call `commands.*`. Since the PriceModal lives in `account_details/`, its gateway method goes here:

```typescript
async recordAssetPrice(assetId: string, date: string, price: number): Promise<Result<null, string>> {
  return commands.recordAssetPrice(assetId, date, price);
}
```

**Rules covered**: MKT-024 (f64 on wire), F3

---

### 2.10 Frontend -- Event subscription: `AssetPriceUpdated`

**File**: `src/features/account_details/account_details_view/useAccountDetails.ts`

In the `subscribeToEvents` effect callback, add `"AssetPriceUpdated"` to the event type check:

```typescript
if (
  type === "TransactionUpdated" ||
  type === "AssetUpdated" ||
  type === "AssetPriceUpdated"
) {
  fetchDetails();
}
```

**File**: `src/lib/store.ts`

Add `"AssetPriceUpdated"` to the `locallyHandledEvents` set so the global store does not trigger a full data re-fetch for this event:

```typescript
const locallyHandledEvents = new Set([
  "TransactionUpdated",
  "AssetPriceUpdated",
]);
```

**Rules covered**: MKT-036, MKT-037

---

### 2.11 Frontend -- Presenter: extend view models

**File**: `src/features/account_details/shared/presenter.ts`

Extend `HoldingRowViewModel` with:

```typescript
assetCurrency: string;
currentPrice: string | null; // formatted decimal or null
currentPriceDate: string | null; // ISO date or null
unrealizedPnl: string | null; // formatted decimal or null
unrealizedPnlRaw: number | null; // micro-units for sign-based color, or null
performancePct: string | null; // formatted percentage or null
performancePctRaw: number | null; // micro-units for sign-based color, or null
```

Extend `AccountSummaryViewModel` with:

```typescript
totalUnrealizedPnl: string | null; // formatted decimal or null
totalUnrealizedPnlRaw: number | null; // micro-units for sign-based color, or null
```

Update `toHoldingRow()`:

- `assetCurrency: detail.asset_currency`
- `currentPrice: detail.current_price != null ? microToDecimal(detail.current_price, 2) : null`
- `currentPriceDate: detail.current_price_date ?? null`
- `unrealizedPnl: detail.unrealized_pnl != null ? microToDecimal(detail.unrealized_pnl, 2) : null`
- `unrealizedPnlRaw: detail.unrealized_pnl ?? null`
- `performancePct: detail.performance_pct != null ? microToDecimal(detail.performance_pct, 2) : null`
- `performancePctRaw: detail.performance_pct ?? null`

Update `toAccountSummary()`:

- `totalUnrealizedPnl: response.total_unrealized_pnl != null ? microToDecimal(response.total_unrealized_pnl, 2) : null`
- `totalUnrealizedPnlRaw: response.total_unrealized_pnl ?? null`

**Rules covered**: MKT-030, MKT-032, MKT-034, MKT-041

---

### 2.12 Frontend -- Presenter tests: extend

**File**: `src/features/account_details/shared/presenter.test.ts`

Update `makeHolding()` helper to include the five new fields with defaults:

```typescript
asset_currency: "USD",
current_price: null,
current_price_date: null,
unrealized_pnl: null,
performance_pct: null,
```

Update `makeResponse()` helper:

```typescript
total_unrealized_pnl: null,
```

Add test cases:

- `toHoldingRow` formats `currentPrice` as 2-decimal string when present (MKT-030)
- `toHoldingRow` returns null for `currentPrice` when `current_price` is null (MKT-032)
- `toHoldingRow` formats `unrealizedPnl` when present (MKT-033)
- `toHoldingRow` returns null for `unrealizedPnl` on currency mismatch (MKT-034)
- `toHoldingRow` formats `performancePct` when present (MKT-035)
- `toHoldingRow` passes `assetCurrency` through (MKT-023)
- `toAccountSummary` formats `totalUnrealizedPnl` when present (MKT-040)
- `toAccountSummary` returns null for `totalUnrealizedPnl` when none qualify (MKT-040)

**Rules covered**: MKT-023, MKT-030, MKT-032, MKT-033, MKT-034, MKT-035, MKT-040, MKT-041

---

### 2.13 Frontend -- `PriceModal` component + hook

**New sub-feature directory**: `src/features/account_details/enter_price/`

#### 2.13.1 Hook: `usePriceModal.ts`

**File**: `src/features/account_details/enter_price/usePriceModal.ts` (new)

Props interface:

```typescript
interface UsePriceModalProps {
  assetId: string;
  assetName: string;
  assetCurrency: string;
  currentPrice: string | null; // already formatted decimal, or null
  currentPriceDate: string | null; // ISO date, or null
  onSubmitSuccess?: () => void;
}
```

State and logic:

- `date`: initialized to today's ISO date (editable, MKT-011)
- `price`: initialized to `currentPrice` if `currentPriceDate === today`, else `""` (MKT-012, MKT-013)
- `error: string | null`
- `isSubmitting: boolean`
- `isFormValid: boolean` -- computed: `date` is non-empty AND `price` is non-empty AND parsed price > 0 AND date is valid ISO and not in future (MKT-020, MKT-021, MKT-022)
- `handleDateChange(value: string)`: updates date state
- `handlePriceChange(value: string)`: updates price state
- `handleSubmit`: validates, calls `accountDetailsGateway.recordAssetPrice(assetId, date, parseFloat(price))`, handles success (snackbar + `onSubmitSuccess`) and error (inline error), manages `isSubmitting` (MKT-027, MKT-028, MKT-029)

The hook calls `accountDetailsGateway.recordAssetPrice` (gateway from section 2.9).

**Rules covered**: MKT-011, MKT-012, MKT-013, MKT-020, MKT-021, MKT-022, MKT-027, MKT-028, MKT-029

#### 2.13.2 Component: `PriceModal.tsx`

**File**: `src/features/account_details/enter_price/PriceModal.tsx` (new)

Uses `FormModal` from `@/ui/components/modal/`.

Props: `isOpen`, `onClose`, and the same props as `UsePriceModalProps`.

Layout:

- Title: i18n key `market_price.modal_title`
- Asset name: read-only label (MKT-011)
- Date field: `DateField`, default today, editable (MKT-011)
- Price field: `AmountField` or `TextField`, with currency code label next to it (MKT-023)
- Submit button: disabled when `!isFormValid || isSubmitting`, shows spinner when `isSubmitting` (MKT-020, MKT-027)
- Inline error message below the relevant field (MKT-029)

**Rules covered**: MKT-010, MKT-011, MKT-020, MKT-023, MKT-027, MKT-028, MKT-029

#### 2.13.3 Hook tests: `usePriceModal.test.ts`

**File**: `src/features/account_details/enter_price/usePriceModal.test.ts` (new)

Test cases (to be written by `test-writer-frontend`):

- Pre-fills date with today (MKT-011)
- Pre-fills price when `currentPriceDate === today` (MKT-012)
- Leaves price empty when `currentPriceDate !== today` (MKT-012)
- Submit disabled when price is empty (MKT-020)
- Submit disabled when date is empty (MKT-020)
- Calls gateway with correct args on valid submit (MKT-024)
- Shows snackbar and calls onSubmitSuccess on success (MKT-028)
- Shows inline error on backend rejection (MKT-029)
- Disables submit during in-flight request (MKT-027)

**Rules covered**: MKT-011, MKT-012, MKT-013, MKT-020, MKT-024, MKT-027, MKT-028, MKT-029

---

### 2.14 Frontend -- Integrate PriceModal into AccountDetailsView

**File**: `src/features/account_details/shared/types.ts`

Add a new target type for the price modal:

```typescript
export type PriceTarget = {
  assetId: string;
  assetName: string;
  assetCurrency: string;
  currentPrice: string | null;
  currentPriceDate: string | null;
};
```

**File**: `src/features/account_details/account_details_view/AccountDetailsView.tsx`

- Add state: `const [priceTarget, setPriceTarget] = useState<PriceTarget | null>(null);`
- Add handlers: `handlePriceClose`, `handlePriceSuccess` (close modal + retry, same pattern as buy/sell)
- Render `PriceModal` when `priceTarget` is set (same conditional pattern as `BuyTransactionModal` / `SellTransactionModal`)
- Pass `onEnterPrice={setPriceTarget}` to `HoldingRow`

**File**: `src/features/account_details/account_details_view/HoldingRow.tsx`

- Add prop: `onEnterPrice: (target: PriceTarget) => void`
- Add "Enter price" `IconButton` in the actions column (MKT-010). Icon: `DollarSign` or `TrendingUp` from lucide-react. Place before or after existing Buy/Sell/Search buttons.
- The button calls `onEnterPrice` with data sourced from the `HoldingRowViewModel` (MKT-013: no additional fetch).
- Add aria-label using i18n key `market_price.action_enter_price`

**File**: `src/features/account_details/account_details_view/AccountDetailsView.tsx` -- table header

Add three new `<th>` columns in the active holdings table header:

- `market_price.column_current_price` -- "Current Price"
- `market_price.column_unrealized_pnl` -- "Unrealized P&L"
- `market_price.column_performance_pct` -- "Performance %"

**File**: `src/features/account_details/account_details_view/HoldingRow.tsx` -- table cells

Add three new `<td>` cells rendering:

- Current Price: `row.currentPrice ?? "---"` with secondary label `row.currentPriceDate` if present (MKT-030, MKT-032)
- Unrealized P&L: use `PnlCell`-like rendering with sign-based color; `"---"` when null (MKT-032, MKT-034)
- Performance %: formatted percentage with sign-based color; `"---"` when null (MKT-032, MKT-034)

Note: the "---" placeholder should use the i18n key `account_details.pnl_placeholder` (already exists as "--").

**File**: `src/features/account_details/account_details_view/AccountDetailsView.tsx` -- summary header

Add `totalUnrealizedPnl` display in the summary header next to total cost basis and total realized P&L:

- When `summary.totalUnrealizedPnlRaw != null`: display with sign-based color (MKT-041)
- When null: display "---" (MKT-041)

**Rules covered**: MKT-010, MKT-013, MKT-030, MKT-032, MKT-034, MKT-041

---

### 2.15 Frontend -- i18n keys

**Files**: `src/i18n/locales/en/common.json`, `src/i18n/locales/fr/common.json`

Add new key group `market_price`:

English:

```json
"market_price": {
  "modal_title": "Enter Market Price",
  "action_enter_price": "Enter price",
  "form_date_label": "Date",
  "form_price_label": "Price",
  "column_current_price": "Current Price",
  "column_unrealized_pnl": "Unrealized P&L",
  "column_performance_pct": "Perf. %",
  "price_as_of": "as of {{date}}",
  "success_recorded": "Price recorded.",
  "error_price_positive": "Price must be greater than zero.",
  "error_date_required": "Date is required.",
  "error_date_future": "Date cannot be in the future.",
  "error_generic": "An error occurred. Please try again."
}
```

Add to `account_details` group:

```json
"total_unrealized_pnl": "Total Unrealized P&L"
```

French translations follow the same structure with appropriate translations.

**Rules covered**: MKT-028, MKT-029, F16

---

### 2.16 Frontend -- `useAccountDetails.test.ts` updates

**File**: `src/features/account_details/account_details_view/useAccountDetails.test.ts`

Extend existing tests or add new tests:

- Verify that `AssetPriceUpdated` event triggers re-fetch (MKT-036)

**Rules covered**: MKT-036

---

## 3. Rules Coverage

| Rule    | Scope    | Description                               | Implementation Task                                      |
| ------- | -------- | ----------------------------------------- | -------------------------------------------------------- |
| MKT-010 | frontend | Enter price button on active holding rows | 2.14 (HoldingRow)                                        |
| MKT-011 | frontend | Modal pre-fill: asset name + today's date | 2.13 (usePriceModal, PriceModal)                         |
| MKT-012 | frontend | Pre-fill price when same-day entry exists | 2.13 (usePriceModal)                                     |
| MKT-013 | frontend | No additional backend call for pre-fill   | 2.13 (usePriceModal), 2.14 (HoldingRow)                  |
| MKT-020 | frontend | Required fields: date + price             | 2.13 (usePriceModal)                                     |
| MKT-021 | both     | Price > 0 validation                      | 2.1 (AssetPrice::new), 2.13 (usePriceModal)              |
| MKT-022 | both     | Date validation: ISO, not future          | 2.1 (AssetPrice::new), 2.13 (usePriceModal)              |
| MKT-023 | both     | Currency label on price field             | 2.13 (PriceModal), 2.11 (presenter)                      |
| MKT-024 | backend  | i64 micro storage, f64 on wire            | 2.3 (AssetService::record_price), 2.5 (api)              |
| MKT-025 | backend  | Upsert by (asset_id, date)                | 2.0 (migration), 2.2 (repo upsert)                       |
| MKT-026 | backend  | AssetPriceUpdated event                   | 2.3 (AssetService), 2.4 (Event enum)                     |
| MKT-027 | frontend | In-flight submit state                    | 2.13 (usePriceModal, PriceModal)                         |
| MKT-028 | frontend | Success: close modal + snackbar           | 2.13 (usePriceModal)                                     |
| MKT-029 | frontend | Error: inline message, modal stays open   | 2.13 (usePriceModal, PriceModal)                         |
| MKT-030 | both     | Current Price column                      | 2.6 (HoldingDetail), 2.11 (presenter), 2.14 (HoldingRow) |
| MKT-031 | backend  | Latest price resolution via AssetService  | 2.3 (get_latest_price), 2.7 (orchestrator)               |
| MKT-032 | frontend | No-price placeholder "---"                | 2.14 (HoldingRow)                                        |
| MKT-033 | backend  | Unrealized P&L same-currency computation  | 2.7 (orchestrator)                                       |
| MKT-034 | both     | Currency mismatch: None/---               | 2.7 (orchestrator), 2.14 (HoldingRow)                    |
| MKT-035 | backend  | Performance % computation                 | 2.7 (orchestrator)                                       |
| MKT-036 | frontend | Reactivity: re-fetch on AssetPriceUpdated | 2.10 (useAccountDetails)                                 |
| MKT-037 | both     | Event registration + locallyHandledEvents | 2.4 (Event enum), 2.10 (store.ts)                        |
| MKT-040 | backend  | Total unrealized P&L                      | 2.6 (AccountDetailsResponse), 2.7 (orchestrator)         |
| MKT-041 | frontend | Total unrealized P&L display              | 2.11 (presenter), 2.14 (AccountDetailsView)              |
| MKT-042 | backend  | No delete/edit -- correction by re-record | 2.0 (migration), 2.2 (upsert only)                       |
| MKT-043 | backend  | Unknown asset rejection                   | 2.3 (AssetService::record_price)                         |

---

## 4. Commit Checkpoints

### Checkpoint 1: Backend layer

**Suggested title**: `feat(asset): implement market price recording backend`
**Scope**: Migration, AssetPrice entity, repository, service methods, event variant, Tauri command, orchestrator extension, all backend tests.

### Checkpoint 2: Frontend layer

**Suggested title**: `feat(account-details): display market price and unrealized P&L`
**Scope**: Gateway, presenter, PriceModal component + hook, HoldingRow extension, AccountDetailsView columns, event subscription, store update, i18n, all frontend tests.

### Checkpoint 3: Tests and docs

**Suggested title**: `docs(market-price): update architecture and close spec items`
**Scope**: ARCHITECTURE.md updates (event bus table, AssetPrice entity, new command, extended DTOs), `docs/todo.md` updates.
