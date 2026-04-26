# Business Rules — Market Price (MKT)

## Context

The Market Price feature allows users to record the current market value of a financial asset manually. It is the first phase of market price support; automated price feeds are a future feature that will slot into the same data model.

A price is recorded per asset (not per holding) and is timestamped: multiple entries can accumulate over time, one per date per asset. The Account Details view uses the most recently dated price to display the current value, unrealized gain/loss, and performance percentage for each active holding.

This spec is a **feature spec** spanning two domains: price recording belongs to the `asset` bounded context; display of current price and derived values extends the `use_cases/account_details/` use case. See `docs/spec/account-details.md` for the baseline Account Details behaviour that this spec extends.

Recording a buy or sell transaction does **not** automatically create a price record. `Transaction.unit_price` is the price transacted at (a cost-basis input); `AssetPrice.price` is the current market value of the asset. Conflating them would show cost as current price, making unrealized P&L meaningless.

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

---

## Open Questions

None — all questions have been resolved.
