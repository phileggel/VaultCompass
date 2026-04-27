# Business Rules — Account Performance Dashboard

## Context

The user wants to visualize the performance of a specific account over time: total value evolution period by period, breakdown by asset class, gain/loss versus historical cost, and base-100 indices for pure performance. Time granularity (monthly or yearly) is determined by the account's `UpdateFrequency`. The feature builds on existing entities — `Account`, `Asset`, `AssetCategory`, `AssetPrice` — and on two entities to be created in dedicated specs: `Operation` (buy/sell history, spec `operations.md`) and `ExchangeRate` (manual exchange rates, spec `account-currency.md`). The performance command is a cross-context use case in `use_cases/`.

> **Dependencies**: this spec can only be implemented after the `operations.md` spec (operations history) and the `account-currency.md` spec (per-account currency + exchange rates).

---

## Business rules

**R1 — Asset value over a period (backend)**: The value of an asset held in an account for a given period is `reconstructed_quantity × latest AssetPrice.price`, where the quantity is reconstructed at the end of the period from the operations history (spec `operations.md`), and the price is the latest `AssetPrice.price` whose date is on or before the last day of the period. The last day of a monthly period is the last calendar day of the month (e.g. Jan 31, Feb 28/29); for a yearly period, it is December 31. If no price is available before this threshold, the asset is excluded from the period's calculation (never an implicit zero value). Values are converted into the account's reference currency (`Account.currency`) using the manually entered exchange rates (spec `account-currency.md`).

**R2 — Total account value over a period (backend)**: The total account value for a period is the sum of all its holdings' values computed per R1.

**R3 — Time granularity determined by UpdateFrequency (backend + frontend)**: Granularity is determined by the account's `UpdateFrequency`: `ManualMonth` or `Automatic` → monthly periods; `ManualYear` → yearly periods. The `ManualDay` and `ManualWeek` frequencies use monthly granularity by default.

**R4 — Full time range (frontend)**: The dashboard always displays the full available history for the account, with no range selector or temporal truncation.

**R5 — Period progression (backend)**: A period's progression is `value_n − value_{n−1}` (absolute) and `(value_n − value_{n−1}) / value_{n−1}` (relative). If the previous period has no computable value, both progressions are `null`.

**R6 — YTD base 100 (backend)**: The YTD reference is the total account value at the end of the last period of the previous calendar year (e.g. end of December N−1), set to 100 (not visible in the table). This reference value requires that **all** assets in the account have a price available at that date; if at least one asset is missing a price, the reference is absent and the YTD base 100 displays "—" for the entire year. Formula: `base_n = base_{n−1} × (1 + progression_n)`. The first month displayed may be ≠ 100. If no complete value is available at the end of the previous year, the reference is the last month where all assets have a price, before the first month of the year.

**R7 — Historical base 100 (backend)**: The historical reference is the account value at the end of the period preceding the first month available in the full history, set to 100 (not visible). Same formula as R6. If the first available month has no predecessor, this first month is itself the reference (= 100) and is displayed as 100. This computation does not account for capital contributions between periods (accepted simplification).

**R8 — Per-category value in the table (backend)**: For each period, the backend computes the value of each asset category (`AssetCategory`) present in the account per R1. These values are returned in `CategoryValue { category_id, category_name, value }`. A category with no available price for a period returns `null`.

**R9 — Category columns in the table view (frontend)**: Category columns are dynamic (one column per distinct category present in the results). The table is horizontally scrollable when the column count exceeds the visible width.

**R10 — Absolute and relative account performance (backend)**: Computed from the operations history (spec `operations.md`) using the VWAP method. For each asset: `gain = (current_price − VWAP_average_price) × current_quantity`. Total account performance is the sum of gains across all assets, converted to the account's reference currency. `relative_gain_pct = absolute_gain / total_VWAP_cost × 100`. If total cost is zero, `relative_gain_pct` is `null`.

**R11 — Period display order (frontend)**: Periods are displayed in ascending chronological order (oldest first, most recent at the bottom).

**R12 — Period without data (frontend)**: A period with no available price for any holding displays "—" in all its numeric cells — never zero, so as not to skew progressions and bases.

**R13 — Chart view (frontend)**: The chart view displays a vertical bar chart with one bar per period, whose height represents the total account value (R2). Periods without data (R12) are represented by a missing bar or a distinct visual indicator. Three indicator cards are displayed above the chart: current total value, total absolute gain, total relative gain (in %).

**R14 — Dedicated backend command in use_cases/ (backend)**: The Tauri command `get_account_performance(account_id)` is implemented in `use_cases/` because it requires data from multiple contexts: `AccountRepository` + `OperationRepository` (context/account), `AssetRepository` + `AssetCategoryRepository` + `PriceRepository` (context/asset), and `ExchangeRateRepository` (context/account-currency), injected as dependencies. It returns `AccountPerformanceResult { periods: Vec<AccountPeriod>, performance: AccountPerformance }` with `AccountPeriod { period_label, total_value, progression_abs, progression_pct, base100_ytd, base100_all, category_values: Vec<CategoryValue { category_id, category_name, value }> }`.

**R15 — Unknown account (backend)**: If the supplied `account_id` does not exist, the command returns an explicit error (not an empty result).

**R16 — Access from the account view (frontend)**: The dashboard is accessible from `AccountAssetDetailsView` via a "Performance" tab. It is not an item in the navigation drawer.

**R17 — No external benchmark in this version (frontend)**: Comparison with an external index (CAC40, etc.) is out of scope for this version.

---

## Workflow

```
[User navigates to an account]
  → Click on the "Performance" tab
          │
          ▼
[Call get_account_performance(account_id)]
  → Full history returned
  → Granularity read from Account.update_frequency
          │
          ├─→ Chart view : total-value bars per period
          │                 + cards : current value, gain €, gain %
          └─→ Table view : one row per period
                            Period | Prog (€) | Prog (%) | Total value
                            | Cat.1 | Cat.2 | … | YTD base 100 | Hist. base 100
```

---

## UX Mockup

### Entry point

From `AccountAssetDetailsView` — "Performance" tab at the top of the view.

### Main component

Panel embedded in the account view (no modal). Two sub-views accessible via tabs:

1. **Chart view** — bar chart (total value per period) + 3 indicator cards
2. **Table view** — grid with one row per period, horizontal scroll

### Table view — columns and example

| Period   | Prog (€) | Prog (%) | Total value | [Cat. A] | [Cat. B] | …   | YTD base 100 | Hist. base 100 |
| -------- | -------- | -------- | ----------- | -------- | -------- | --- | ------------ | -------------- |
| Jan 2025 | +370 €   | +3.1%    | 12,450 €    | 5,200 €  | 7,250 €  | …   | 103.1        | 134.2          |
| Feb 2025 | +320 €   | +2.6%    | 12,770 €    | 5,400 €  | 7,370 €  | …   | 105.8        | 137.7          |
| Mar 2025 | —        | —        | —           | —        | —        | …   | —            | —              |

> The January YTD base 100 is > 100 because the reference is the end of December of the previous year.

### States

- **Empty**: "No price data available. Start by entering prices for your assets."
- **Loading**: Skeletons over the chart and the table
- **Partial data**: Cells without data shown with "—" + tooltip "Price not entered for this period"
- **Error**: Error message with Retry button

### User flow

1. The user navigates to an account in `AccountAssetDetailsView`.
2. They click the "Performance" tab.
3. The chart view is displayed by default with the value bars and the 3 cards.
4. They switch to the table view to see detailed numbers row by row.
5. Hovering a "Base 100" cell shows a tooltip indicating the reference period (e.g. "Reference: Dec 2024 = 100").

---

## Dependencies

This spec can only be implemented after:

- **`docs/operations.md`** — operations history (buy/sell), VWAP computation, portfolio reconstruction at a past date. Replaces direct entry of `AssetAccount.average_price` and `AssetAccount.quantity`.
- **`docs/account-currency.md`** — per-account reference currency (`Account.currency`), manual exchange rates (`ExchangeRate`), conversion of values into the account currency.

## Open questions

None — all questions have been resolved.
