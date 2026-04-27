# Business Rules — Market Price (MKT)

## Context

The Market Price feature allows users to record the current market value of a financial asset manually. It is the first phase of market price support; automated price feeds are a future feature that will slot into the same data model.

A price is recorded per asset (not per holding) and is timestamped: multiple entries can accumulate over time, one per date per asset. The Account Details view uses the most recently dated price to display the current value, unrealized gain/loss, and performance percentage for each active holding.

This spec is a **feature spec** spanning two domains: price recording belongs to the `asset` bounded context; display of current price and derived values extends the `use_cases/account_details/` use case. See `docs/spec/account-details.md` for the baseline Account Details behaviour that this spec extends.

By default, recording a buy or sell transaction does **not** automatically create a price record. `Transaction.unit_price` is the price transacted at (a cost-basis input); `AssetPrice.price` is the current market value of the asset. Conflating them by default would show cost as current price, making unrealized P&L meaningless. As an explicit opt-in (see MKT-050+), the user can choose — globally or per transaction — to also persist the transacted unit price as the asset's market price for the transaction date.

All financial values are stored as `i64` micro-units per [ADR-001](../adr/001-use-i64-for-monetary-amounts.md).

---

## Entity Definition

### AssetPrice

Represents a manually recorded market price for a financial asset on a specific date. Owned by the `asset` bounded context.

| Field      | Business meaning                                                                   |
| ---------- | ---------------------------------------------------------------------------------- |
| `asset_id` | The asset whose market price this record describes.                                |
| `date`     | The calendar date this price observation applies to (ISO 8601, e.g. `2026-04-26`). |
| `price`    | Market price per unit in the asset's native currency (i64 micros, ADR-001).        |

> The combination `(asset_id, date)` is unique: only one price per asset per day. Recording a second price for the same `(asset_id, date)` pair overwrites the first (MKT-025). No standalone edit or delete command is provided in this phase; correction is done by re-recording (MKT-042).

### HoldingDetail (extended)

The `HoldingDetail` DTO defined in the ACD spec gains five new fields populated by this feature.

| Field                | Business meaning                                                                                                                                                                                       |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `asset_currency`     | ISO 4217 currency code of the asset (e.g. `"USD"`). Required for the price label in the modal (MKT-023) and for the currency-mismatch check (MKT-034). Always present.                                 |
| `current_price`      | Most recently dated `AssetPrice.price` for this asset, in asset currency (i64 micros). `None` if no price has ever been recorded.                                                                      |
| `current_price_date` | ISO date string of the price observation used as `current_price`. `None` when `current_price` is `None`.                                                                                               |
| `unrealized_pnl`     | Unrealized gain or loss in account currency (i64 micros). `None` when no price exists or when asset and account currencies differ (MKT-034). `0` when current price equals average price (not `None`). |
| `performance_pct`    | `unrealized_pnl / cost_basis × 100`, expressed as i64 micros (e.g. 5.25 % = 5 250 000). `None` when `unrealized_pnl` is `None` or `cost_basis` is zero. `0` when `unrealized_pnl` is zero.             |

### AccountDetailsResponse (extended)

The `AccountDetailsResponse` DTO gains one new field.

| Field                  | Business meaning                                                                                                                                                                  |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `total_unrealized_pnl` | Sum of `unrealized_pnl` (i64 micros) across all active holdings where a value is computable (same-currency with a recorded price). `None` when no holding has a computable value. |

---

## Business Rules

### Eligibility and Initiation (010–019)

**MKT-010 — Entry point (frontend)**: An "Enter price" action is available on each **active** holding row in the Account Details view, alongside the existing Buy and Sell actions. It is not shown on closed holdings.

**MKT-011 — Modal pre-fill — asset and date (frontend)**: Opening the "Enter price" form pre-fills the asset name as a read-only label and the date field with today's date (editable by the user, e.g. to backdate to yesterday's closing price).

**MKT-012 — Modal pre-fill — price (frontend)**: When `HoldingDetail.current_price_date` equals today's ISO date at the time the modal is opened, the price field is pre-filled with `HoldingDetail.current_price`. When the latest recorded price is from a prior date, the price field opens empty.

**MKT-013 — No additional backend call for pre-fill (frontend)**: The pre-fill data (MKT-011, MKT-012) is sourced from the `HoldingDetail` already loaded by the Account Details view. Opening the modal requires no additional IPC request.

### Recording a Price (020–029)

**MKT-020 — Required fields (frontend)**: The price form requires a non-empty date and a non-empty price. The submit button is disabled while either field is empty. The asset is implicit from the entry point and is not modifiable by the user.

**MKT-021 — Price validation (frontend + backend)**: A valid price is strictly greater than zero. The backend rejects a submitted price of zero or below with a specific error. The frontend validates inline and disables the submit button until corrected.

**MKT-022 — Date validation (frontend + backend)**: A valid date is a well-formed ISO 8601 calendar date (`YYYY-MM-DD`) that is not in the future. Any past date is accepted; no lower bound applies (users may backdate historical prices). The backend rejects an invalid or future date with a specific error. The frontend validates inline and disables the submit button until corrected.

**MKT-023 — Price currency (frontend + backend)**: The price is stored in the asset's native currency. No currency conversion is applied at recording time. The asset's currency code is displayed as a read-only label next to the price input field so the user knows which currency they are entering.

**MKT-024 — i64 storage (backend)**: The price is stored as i64 micro-units per ADR-001. The frontend transmits the human-readable decimal; the backend converts to micros at the IPC boundary.

**MKT-025 — Upsert by (asset, date) (backend)**: If a price record already exists for the same `(asset_id, date)` pair, it is overwritten with the new value. Otherwise a new record is created. This is transparent to the user; the form behaves identically for new and existing entries.

**MKT-026 — AssetPriceUpdated event (backend)**: After a successful upsert, the backend publishes an `AssetPriceUpdated` event on the event bus. This event carries no payload; it is a bare signal consistent with `AssetUpdated`, `CategoryUpdated`, `AccountUpdated`, and `TransactionUpdated`. It is published by the `asset` bounded context per B4. The Tauri frontend event discriminant string is `"AssetPriceUpdated"` — the variant name is forwarded as-is by the event forwarder, matching the convention for all existing events.

**MKT-027 — In-flight state (frontend)**: While the upsert request is in progress, the submit button is disabled and displays a spinner to prevent double-submission.

**MKT-028 — Success feedback (frontend)**: On success, the modal closes and a snackbar confirms the price was recorded.

**MKT-029 — Error feedback (frontend)**: On validation failure or backend rejection, the modal remains open. An inline error message is shown adjacent to the invalid field. The user can correct and resubmit without reopening the form.

### Display in Account Details (030–039)

**MKT-030 — Current price column (frontend + backend)**: The Account Details active holdings table gains a "Current Price" column. For each holding row, it displays `HoldingDetail.current_price` formatted in the asset's native currency. When `current_price_date` is available, it is shown as a secondary label (e.g. "as of 2026-04-25") to indicate the age of the data.

**MKT-031 — Latest price resolution (backend)**: The `AccountDetailsUseCase` retrieves the most recently dated `AssetPrice` for each active holding's asset via `AssetService`, per ADR-004 (use cases inject services, not repositories). If no record exists for an asset, `current_price` and `current_price_date` are `None`. A failure in the price lookup does not abort the overall `get_account_details` response; it degrades gracefully by returning `None` for the affected holding's price fields.

**MKT-032 — No-price placeholder (frontend)**: When `current_price` is `None` for a holding, the "Current Price", "Unrealized P&L", and "Performance %" columns display "—" for that row.

**MKT-033 — Unrealized P&L — same currency (backend)**: When the asset's native currency equals the account's currency (the gate condition defined in MKT-034), the backend computes `unrealized_pnl = (current_price − average_price) × quantity` using i128 intermediates before scaling back to i64, consistent with ACD-024. Both `current_price` and `average_price` are expressed in the same currency under this condition, making the subtraction valid. The result is included in `HoldingDetail.unrealized_pnl`. A zero result is returned as `0`, not `None`.

**MKT-034 — Unrealized P&L — currency mismatch (frontend + backend)**: When the asset currency differs from the account currency, `HoldingDetail.unrealized_pnl` and `HoldingDetail.performance_pct` are `None`. The frontend displays "—" in those columns. No exchange-rate conversion is attempted in this phase; multi-currency unrealized P&L is deferred to a future iteration.

**MKT-035 — Performance % (backend)**: When `unrealized_pnl` is available and `cost_basis` is non-zero, the backend computes `performance_pct = unrealized_pnl × 100 / cost_basis` as i64 micros using i128 intermediates and Rust integer division (truncation toward zero). Example: 5.25 % = 5 250 000 micros; −3.7 % = −3 700 000 micros. A zero result is returned as `0`, not `None`. When `cost_basis` is zero, `performance_pct` is `None`.

**MKT-036 — Reactivity (frontend)**: The Account Details event subscription adds `AssetPriceUpdated` alongside the existing `TransactionUpdated` and `AssetUpdated` subscriptions (ACD-039). Upon receiving `AssetPriceUpdated`, the view re-fetches account details, ensuring that newly recorded prices and all derived values (unrealized P&L, performance %, totals) are reflected immediately without a manual page refresh.

**MKT-037 — AssetPriceUpdated event registration (backend + frontend)**: The `AssetPriceUpdated` event is added to the event bus enum alongside `AssetUpdated`, `CategoryUpdated`, `AccountUpdated`, and `TransactionUpdated`. It is published exclusively by the `asset` bounded context. The global store treats it as a locally-handled event (no global data re-fetch triggered). `ARCHITECTURE.md` must be updated to register `AssetPriceUpdated` in the event bus table and to document that `useAccountDetails` subscribes to it alongside `TransactionUpdated` and `AssetUpdated`.

### Account Summary (040–049)

**MKT-040 — Total unrealized P&L (backend)**: `AccountDetailsResponse.total_unrealized_pnl` is the sum of `unrealized_pnl` across all active holdings for which the value is computable (same-currency holdings with a recorded price). Holdings with a currency mismatch or no recorded price are excluded from the sum and contribute nothing. When no holding qualifies, the field is `None`.

**MKT-041 — Total unrealized P&L display (frontend)**: The Account Details summary row displays `total_unrealized_pnl`. When the value is `None`, the summary shows "—". When the value is a number (including zero), it is displayed as-is; per-row "—" placeholders already communicate which individual holdings were excluded from the sum.

**MKT-042 — No delete or standalone edit in this phase (backend)**: `AssetPrice` records cannot be deleted or updated individually via a dedicated command in this phase. Correction is done by re-recording a price for the same `(asset_id, date)`, which overwrites the existing entry (MKT-025).

**MKT-043 — Unknown asset rejection (backend)**: The backend rejects `record_asset_price` with a specific error if `asset_id` does not refer to a known asset. In normal use the asset is always selected from active holdings, making this case unreachable from the UI; the guard exists for API-level correctness.

### Auto-record from Transactions (050–069)

This section extends the buy/sell transaction flow defined in `docs/spec/financial-asset-transaction.md` (TRX) and `docs/spec/sell-transaction.md` (SEL). When the user opts in, recording a buy or sell transaction also writes an `AssetPrice` record so the transacted unit price becomes the asset's market price for the transaction date. Standalone edit and delete of `AssetPrice` records remain out of scope (see MKT-042 and project todo).

**MKT-050 — Global auto-record toggle (frontend)**: The Settings page exposes a toggle "Automatically record transaction price as market price". The toggle defaults to OFF. The user's choice persists across sessions on the current device. The toggle controls only the default state of the per-transaction checkbox (MKT-052); it never bypasses or replaces that checkbox.

**MKT-051 — Per-transaction checkbox (frontend)**: The buy and sell transaction forms (both creation forms — modal and the standalone /transactions/new page — and their edit variants) display a checkbox "Use this price as the market price for {transaction date}" placed immediately before the form's primary submit action, after all data fields. The label's date placeholder reflects the form's current `date` field value and updates live when the user changes it.

**MKT-052 — Checkbox default state (frontend)**: When the form opens to **create** a new transaction, the checkbox initial state equals the current value of the global toggle (MKT-050). When the form opens to **edit** an existing transaction, the checkbox initial state is always OFF, regardless of the global toggle.

**MKT-053 — Checkbox snapshot semantics (frontend)**: The checkbox initial state (MKT-052) is read from the global toggle once at form open. Subsequent changes to the global toggle do not propagate into already-open forms; the user keeps whatever state they have already set in the open form.

**MKT-054 — Submit payload (frontend + backend)**: The frontend forwards the checkbox state as a `record_price: bool` field added to the existing `CreateTransactionDTO` already used by `add_transaction` and `update_transaction` (per the project's Specta single-DTO convention). The backend never reads the global toggle directly — the per-call flag carried in the DTO is the only signal that determines whether a price is recorded.

**MKT-055 — Auto-write inside the orchestrator's DB transaction (backend)**: When `record_price` is `true` and `tx.unit_price > 0` (see MKT-061 for the zero-price exception), the `record_transaction` orchestrator writes an `AssetPrice` row directly inside the same DB transaction it has already opened for the transaction insert/update and the holding recomputation. The write targets `(asset_id = tx.asset_id, date = tx.date, price = tx.unit_price)` and uses the same upsert semantics as MKT-025 (insert on absence, replace on `(asset_id, date)` collision). The price is taken in the asset's native currency; `tx.unit_price` already excludes fees per the TRX domain definition, so no fee adjustment is applied. Validation rules MKT-021 (price > 0) and MKT-022 (date not in future) hold by construction here: TRX-020 enforces `tx.date` not in the future, and the `tx.unit_price > 0` precondition is enforced by MKT-061.

**MKT-056 — Atomicity (backend)**: The transaction insert/update, the holding recomputation (TRX-027 / SEL-025), and the auto-record `AssetPrice` upsert all commit in a single database transaction. If any step fails, the entire operation is rolled back; no partial state is persisted. This matches the pre-existing TRX-027 atomicity guarantee, extended to include the price write when `record_price = true`.

**MKT-057 — AssetPriceUpdated event on auto-record (backend)**: After the orchestrator's DB transaction commits successfully and `record_price` was `true` and a price was actually written (i.e. MKT-061 did not skip), the orchestrator invokes the `asset` bounded context's notification entry-point (mirroring how `TransactionService.notify_transaction_updated()` is invoked after commit per B8) to publish the `AssetPriceUpdated` event defined in MKT-026. This is in addition to the `TransactionUpdated` event published by the transaction context. The two events are independent signals; their relative publication order is unspecified and is irrelevant to consumers because each subscriber refetches idempotently. When `record_price` is `false` or MKT-061 skipped the write, no `AssetPriceUpdated` event is published; behaviour is identical to the pre-feature add/update transaction flow.

**MKT-058 — Conflict — silent overwrite (backend)**: If an `AssetPrice` record already exists at `(tx.asset_id, tx.date)` when `record_price` is `true`, it is silently overwritten with `tx.unit_price` via the same upsert semantics as MKT-025. No prompt or warning is shown to the user; the form behaves identically whether or not a same-day price already exists.

**MKT-059 — Edit lifecycle — price independence (backend)**: Editing a transaction does not modify or remove any `AssetPrice` record previously written by that transaction. When the user re-saves an edited transaction with `record_price = true`, the upsert (MKT-055) targets the transaction's *current* `tx.date` and *current* `tx.unit_price`. If the user changed the transaction date during the edit, the price record at the prior date is left untouched and remains in storage; the upsert lands at the new date as a separate `(asset_id, date)` row. The same applies if the user changed the unit price: only the row at the current date is overwritten.

**MKT-060 — Delete lifecycle — price independence (backend)**: Deleting a transaction does not remove any `AssetPrice` record previously written by that transaction. `AssetPrice` records are independent of the transaction lifecycle: once persisted, they are governed solely by MKT rules (currently only MKT-025 upsert; standalone delete is deferred).

**MKT-061 — Zero unit_price skip (backend)**: If `record_price` is `true` and `tx.unit_price` is `0` (a valid transaction per TRX-020 / SEL-020 — gifted or inherited assets), the orchestrator silently skips the `AssetPrice` write. The transaction itself proceeds normally and commits per its own validation rules; no `AssetPriceUpdated` event is published; no error is surfaced to the user. Rationale: a zero market price would conflict with MKT-021 (price > 0) and is not a meaningful signal of the asset's market value.

**MKT-062 — Auto-record failure surfaces as transaction error (backend + frontend)**: A persistence failure of the auto-record `AssetPrice` write triggers rollback of the entire orchestrator DB transaction (per MKT-056). The error is returned to the frontend through the existing `add_transaction` / `update_transaction` error contract (the same `RecordTransactionError` channel used for other backend errors in the form). The frontend displays it inline using the existing transaction-form error states from the TRX / SEL specs; no new dedicated error path or UI element is introduced. The user can correct the input or untick the auto-record checkbox and retry.

---

## Workflow

```
Account Details (active holding row)
    → "Enter price" button
    → PriceModal opens (no extra fetch — uses HoldingDetail data)
        date = today (editable)
        price = current_price if current_price_date == today, else empty
        → user enters price, adjusts date if needed
        → submit
            backend: validate price > 0 and date ≤ today
            backend: upsert AssetPrice(asset_id, date, price)
            backend: publish AssetPriceUpdated (bare signal)
        → modal closes + snackbar
        → Account Details re-fetches on AssetPriceUpdated
        → holding row: current price, unrealized P&L, performance % updated
```

### Workflow — Auto-record from a buy/sell transaction (MKT-050+)

```
Settings page
    → "Automatically record transaction price as market price" toggle
    → choice persisted to localStorage (default OFF)

Buy/Sell transaction form (create or edit)
    → checkbox "Use this price as the market price for {date}"
        create mode → default = global toggle snapshot at open (MKT-052)
        edit mode   → default = OFF (MKT-052)
    → user submits
        frontend: record_price: bool added to CreateTransactionDTO (MKT-054)
        backend orchestrator (single DB transaction, MKT-056):
            ├─ insert/update Transaction
            ├─ recompute Holding (TRX-027 / SEL-025)
            └─ if record_price && tx.unit_price > 0:                       (MKT-061 skip-on-zero)
                 upsert AssetPrice(tx.asset_id, tx.date, tx.unit_price)    (MKT-055, MKT-058)
        → commit; on any failure rollback the whole DB transaction        (MKT-056, MKT-062)
        → after commit:
            transaction context publishes TransactionUpdated              (B8)
            if a price was actually written:
                asset context publishes AssetPriceUpdated                 (MKT-057, B8)
        → Account Details re-fetches via AssetPriceUpdated                 (MKT-036)
```

---

## UX Draft

### Entry Point

"Enter price" icon button on each active holding row in Account Details, in the actions column alongside Buy and Sell. Not shown on closed holdings.

### Main Component

Small modal dialog. No navigation — stays within Account Details.

### Form Fields

| Field          | Default                                   | Editable |
| -------------- | ----------------------------------------- | -------- |
| Asset name     | Pre-filled from holding row               | No       |
| Date           | Today                                     | Yes      |
| Price          | Today's existing price if any, else empty | Yes      |
| Currency label | Asset's native currency code              | No       |

### States

- **Submit in-flight** (MKT-027): Submit button disabled + spinner while persisting.
- **Validation / backend error** (MKT-029): Inline error adjacent to the invalid field; modal stays open.
- **Success** (MKT-028): Modal closes; snackbar "Price recorded."
- **No price (holding row)**: "—" in Current Price, Unrealized P&L, Performance % columns.
- **Currency mismatch (holding row)**: Current Price shown in asset currency; Unrealized P&L and Performance % show "—".

### User Flow

1. User views Account Details for an account.
2. User clicks "Enter price" on a holding row.
3. Modal opens immediately (no fetch) with asset name, today's date, and price pre-filled if a same-day entry exists.
4. User types the current market price (in asset currency, shown as a label).
5. User optionally changes the date (e.g. to use yesterday's closing price).
6. User submits.
7. Backend validates, upserts the price, publishes `AssetPriceUpdated`.
8. Modal closes, snackbar confirms.
9. Account Details re-fetches: the holding row now shows current price, unrealized P&L, performance %.

### UX Draft — Auto-record from Transactions (MKT-050+)

#### Settings page

A new toggle row "Automatically record transaction price as market price" sits alongside the existing language preference. Default OFF. State persists across sessions on the current device.

#### Buy and sell transaction forms

A checkbox is added directly above the submit button in:

- the buy creation modal,
- the sell creation modal,
- the standalone /transactions/new form (whether the transaction is a buy or a sell),
- the edit variants of all the above.

| Field             | Default                                                    | Editable |
| ----------------- | ---------------------------------------------------------- | -------- |
| Auto-record price | Snapshot of global toggle on create; always OFF on edit    | Yes      |

The label updates live with the form's date field: "Use this price as the market price for 2026-04-27".

#### States

- **Checkbox unchecked**: no behaviour change; the form behaves exactly as before this feature.
- **Checkbox checked + submit success**: the standard transaction success path (snackbar, modal close, list refresh) is unchanged. The new `AssetPriceUpdated` event causes Account Details to refresh its market-price columns transparently.
- **Submit failure with checkbox checked**: the form remains open with the standard inline transaction error feedback (MKT-062). No price is written (atomicity, MKT-056). The user can untick the checkbox or correct the inputs and retry.
- **Same-day price already recorded**: no warning shown; the existing entry is silently overwritten on submit (MKT-058).
- **Zero `unit_price`**: when the buy/sell unit price is `0` (gifted asset, TRX-020), the auto-record step is silently skipped per MKT-061. The transaction itself succeeds normally; the checkbox state has no observable effect in this case.

#### User flow — global default

1. User opens Settings.
2. User flips "Automatically record transaction price as market price" ON.
3. User opens a buy form anywhere in the app — the auto-record checkbox is pre-checked.
4. User submits; the unit price is recorded as the day's market price in addition to the transaction.

#### User flow — per-transaction override

1. Global toggle is OFF (default).
2. User opens a sell form for an asset they want to also stamp a market price for today.
3. User ticks the auto-record checkbox manually before submitting.
4. User submits; the price is recorded for this transaction only. The next form opens with the box unchecked again.

---

## Open Questions

None — all questions have been resolved.
