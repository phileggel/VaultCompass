# Business Rules — Account Performance (PRF)

## Context

The Account Performance feature presents how a single account's value has evolved over time, period by period, together with the performance earned in each period. A user opens an account and sees a table whose rows are calendar periods (months or years); each row shows the account's value at the end of that period and three performance figures — period-over-period, year-to-date, and since-inception — each expressed both as a currency gain and as a percentage.

Performance is measured **net of the user's own cash flows**: a deposit or withdrawal moves the account's value but is not itself performance. Period values are **reconstructed on demand** from the account's transaction history (which determines the units held and the cash balance at any past date) and the asset price history (which determines each asset's value at any past date) — no period value is persisted. This mirrors the project's existing model, where holdings are always rebuilt by chronological transaction replay rather than cached.

This is a **feature spec** spanning two bounded contexts: the `account` context owns transactions, holdings, and cash; the `asset` context owns recorded prices. Because the two contexts must not import each other (B2), the cross-context read is orchestrated by a dedicated use case in `use_cases/`, injecting `AccountService` and `AssetService` per ADR-003 and ADR-004. It builds on [`docs/spec/account-details.md`](account-details.md) (Global Value, holdings) and [`docs/spec/market-price.md`](market-price.md) (AssetPrice history), and reuses the account `update_frequency` defined in [`docs/spec/account.md`](account.md) (ACC-004).

All monetary values are `i64` micro-units per [ADR-001](../adr/001-use-i64-for-monetary-amounts.md).

---

## Computed Concepts

> This feature persists no new entity. The concepts below are computed read-only views, defined here in business terms; their wire shape is `/contract`'s concern.

### Performance Period

A single calendar period (one month or one year) within the account's lifetime. Carries:

| Concept              | Business meaning                                                                                            |
| -------------------- | ----------------------------------------------------------------------------------------------------------- |
| `period`             | The calendar month or year the row represents.                                                              |
| `end_value`          | The account's Global Value at the last calendar day of the period, in the account's own currency (PRF-020). |
| `period_over_period` | Performance versus the immediately preceding period of the same granularity (PRF-033). Gain + percentage.   |
| `year_to_date`       | Performance from the start of the row's calendar year to the row's period end (PRF-034). Gain + percentage. |
| `since_inception`    | Performance from the account's inception to the row's period end (PRF-035). Gain + percentage.              |

### External Cash Flow

Money entering or leaving the account from outside the tracked world: a `Deposit` (inflow) or `Withdrawal` (outflow), plus an `OpeningBalance` seed (inflow at its recorded cost). `Purchase` and `Sell` are internal conversions between cash and assets and are **not** external flows (PRF-030).

---

## Business Rules

### View Modes and Eligibility (010–019)

**PRF-010 — Entry point (frontend)**: The Account Details view exposes a "Performance" action in its header that navigates to the account performance page at route `/accounts/:id/performance`. The page is scoped to the single account identified by the route.

**PRF-011 — Two view modes (frontend)**: The page offers two mutually exclusive view modes: **month view** (one row per calendar month) and **year view** (one row per calendar year). A control lets the user switch between the available modes. "Month view" and "year view" are the canonical names used throughout this spec.

**PRF-012 — Year view always available (frontend + backend)**: The backend always computes the yearly series for any account, and the frontend offers the year view for every account regardless of its `update_frequency`.

**PRF-013 — Month view eligibility by frequency (frontend + backend)**: The month view is available only when the account's `update_frequency` (ACC-004) is `Automatic`, `ManualDay`, or `ManualWeek` (cadences of at most one week, which produce enough observations for monthly rows). For `ManualMonth` and `ManualYear`, the month view is not offered and the page presents only the year view; the view-mode control is hidden or disabled.

**PRF-014 — Default view mode (frontend)**: When the month view is available (PRF-013), the page opens in month view showing the current calendar year. Otherwise it opens in year view.

**PRF-015 — Year selector in month view (frontend)**: The month view displays the twelve months of a single calendar year and exposes a year selector to change which year is shown. The selector defaults to the current calendar year and offers every year from the account's first data year (PRF-040) through the current calendar year inclusive — the current year is always selectable even when the latest transaction falls in an earlier year.

**PRF-016 — Invalid account guard (backend + frontend)**: If the `account_id` supplied to the backend does not correspond to an existing account, the read is rejected with a specific not-found error. The frontend transitions to the error state (PRF-052). This mirrors the Account Details guard (ACD-012).

### Period Value Computation (020–029)

**PRF-020 — Period value definition (backend)**: The value of a period is the account's Global Value (cash balance + Σ market value of non-cash active holdings, per CSH-094) evaluated as of the period's "period end", expressed in the account's own currency. The period end is the period's last calendar day for any completed period, and today's date for the in-progress current period (whose last calendar day is still in the future).

**PRF-021 — As-of-date holdings reconstruction (backend)**: The units held per asset and the cash balance at a period end are reconstructed by replaying all transactions dated on or before that period-end date, consistent with the chronological replay that derives current holdings (per the TRX/SEL/CSH replay model). Transactions dated after the period end do not contribute.

**PRF-022 — As-of-date price with carry-forward (backend)**: Each non-cash holding is valued using the most recent recorded `AssetPrice` whose `date` is on or before the period-end date. If the held asset has no recorded price on or before that date, it contributes `0` to that period's value (carry last-known, otherwise zero).

**PRF-023 — Cash component (backend)**: The cash balance at period end (reconstructed per PRF-021 from the cash effects of `Deposit`, `Withdrawal`, `Purchase`, and `Sell` transactions per the CSH model) is included at face value in the account currency.

**PRF-024 — Single-currency scope (backend)**: A non-cash holding whose asset currency differs from the account currency contributes `0` to the period value; no exchange-rate conversion is performed in this version, consistent with CSH-094 and MKT-034. Multi-currency valuation is deferred until the FX-rate feature ships.

**PRF-025 — Calculation precision (backend)**: All value and performance computations use `i128` intermediates before scaling back to `i64` micro-units, per ADR-001 and ACD-024.

**PRF-026 — Recompute on read (backend)**: Period values and performance figures are computed on demand when the page is read; no period value is persisted to the database, per ADR-013. The single sources of truth remain the transaction history and the recorded price history.

**PRF-027 — Computation failure (backend)**: If the read fails while loading the account's transactions or any held asset's price history, the command is rejected with a generic database error and the frontend shows the error state (PRF-052). This is distinct from PRF-016 (a known account with no data succeeds with an empty result per PRF-043; only an unknown account yields not-found).

### Performance Metrics (030–039)

**PRF-030 — External cash flow definition (backend)**: Over any period, the net external cash flow is `Σ Deposit − Σ Withdrawal` for transactions dated within the period, plus the recorded cost of any `OpeningBalance` seeded within the period (treated as an inflow). All three are taken in the account's own currency: `Deposit` and `Withdrawal` are already account-currency cash amounts, and an `OpeningBalance` seed's cost is its recorded total cost in account currency. `Purchase` and `Sell` are internal conversions and contribute nothing to net external flow.

> Known limitation (FX): a non-cash `OpeningBalance` in a currency other than the account's contributes its cost as an inflow here but values to `0` in `end_value` (PRF-024, no FX yet), producing a since-inception discrepancy until the FX-rate feature ships. Consistent with the FX deferral in PRF-024 and MKT-034.

**PRF-031 — Net-of-flows gain (backend)**: A metric's performance gain = `end_value − start_value − net_external_flow_over_the_span`, where the start value, end value, and span are those defined by the specific metric — PRF-033 (period-over-period), PRF-034 (year-to-date), PRF-035 (since-inception). A `start_value` of `0` represents inception (no value was held before the span began).

**PRF-032 — Simple Dietz percentage (backend)**: A period's performance percentage is `gain × 100_000_000 / denominator`, expressed as `i64` micro-percent (e.g. 8.00 % = 8 000 000), where `denominator = period_start_value + Σ_f (flow_f × days_from_flow_to_period_end_f) / days_in_period`. Each external flow (PRF-030) is thus weighted by the fraction of the period remaining after its transaction date. All arithmetic uses `i128` intermediates with the `× 100_000_000` scale applied to the numerator **before** the division and truncation toward zero, consistent with the scaled-numerator integer form of MKT-035 (`gain` and all values in micro-units; the unit micros cancel, `× 100` yields percent, `× 1_000_000` yields micro-percent). When `denominator` is `0`, the percentage is absent.

> Worked check: `gain` = €1 000 (1 000 000 000 micros), `denominator` = €12 500 (12 500 000 000 micros) → `1 000 000 000 × 100 000 000 / 12 500 000 000 = 8 000 000` = 8.00 %.

**PRF-033 — Period-over-period metric (backend)**: For each row, the period-over-period performance applies PRF-031/PRF-032 comparing the row's period against the immediately preceding period of the same granularity — the previous month in month view, the previous year in year view. The earliest row in the data span (PRF-040) has no preceding period, so its period-over-period metric is absent (PRF-042); that first period's performance is conveyed by its since-inception metric (PRF-035) instead.

**PRF-034 — Year-to-date metric (backend)**: For each row, the year-to-date performance applies PRF-031/PRF-032 over the span from the start of the row's calendar year (the prior 31 December end value as the start value) to the row's period end, netting all external flows dated within that span. For month rows in the account's first calendar year — where no prior 31 December exists within the account's life — the start value is inception (`0`), so the year-to-date metric equals the since-inception metric for that year and is present; a `0` gain there is a real zero (deposited, nothing moved yet), not an absent baseline. The year-to-date metric is therefore always present for month rows; it is omitted only for year rows (PRF-037).

**PRF-035 — Since-inception metric (backend)**: For each row, the since-inception performance compares the row's period-end value to the total net invested up to that period end (`Σ inflows − Σ outflows` since the account's inception). The inception start value is `0`; the gain is therefore `period_end_value − total_net_invested`, and the percentage uses the Simple Dietz denominator over the full lifetime span.

**PRF-036 — Value-and-percent pairing (frontend)**: Each of the three metrics (PRF-033, PRF-034, PRF-035) is displayed as a pair — the gain in the account's currency and the percentage. A gain is colour-coded by sign per existing P&L presentation; an absent percentage (PRF-032) renders as "—".

**PRF-037 — Year-to-date column omitted in year view (frontend)**: In year view the year-to-date metric (PRF-034) is exactly equal to the period-over-period metric (PRF-033) for every row, because both use the prior 31 December end value as their start value and net the same in-year flows. The year-to-date column is therefore omitted in year view as redundant; year view shows period-over-period and since-inception only. Month view shows all three.

### Period Range and Rows (040–049)

**PRF-040 — Data span (backend)**: Rows span from the period containing the account's earliest transaction date through the current period (the present month in month view, the present year in year view). No period precedes the first-transaction period — there is no leading zero-value row; the first-transaction period is the earliest row. A period in which the account holds nothing and has no cash has an end value of `0`.

**PRF-041 — Row ordering (frontend)**: Rows are ordered most-recent first (descending by period).

**PRF-042 — Absent baseline renders as "—" (backend + frontend)**: When a metric's comparison baseline does not exist — for example the first period has no preceding period for the period-over-period metric — that metric's gain and percentage are reported as absent and rendered "—".

**PRF-043 — No transactions (backend)**: An account with no transactions has no data span (PRF-040) and produces an empty result; the frontend shows the empty state (PRF-051).

### States (050–059)

**PRF-050 — Loading state (frontend)**: While the performance computation is in progress, the page displays a loading skeleton for the table.

**PRF-051 — Empty state (frontend)**: When the account has no transactions (PRF-043), the page shows an explicit empty state indicating there is no performance data yet, with an affordance to add a transaction (consistent with ACD-035).

**PRF-052 — Error state (frontend)**: If the computation fails, the page shows a generic error message and a Retry button that re-triggers the read, consistent with ACD-038.

**PRF-053 — Back navigation (frontend)**: The page provides navigation back to the Account Details view for the same account.

### Reactivity (060–069)

**PRF-060 — Reactivity to data changes (frontend)**: The page re-fetches its data when it receives a `TransactionUpdated`, `AssetPriceUpdated`, or `AccountUpdated` event, since all three can change reconstructed period values (transactions change holdings and flows, prices change valuations, account changes affect currency/frequency). This mirrors the Account Details subscription set (ACD-039, ACD-040, MKT-036).

---

## Workflow

```
[Account Details header → "Performance"]
  → Route: /accounts/:id/performance
          │
          ├─ [use_cases/account_performance/: load account (currency, update_frequency)]
          ├─ [decide available view modes by update_frequency (PRF-012, PRF-013)]
          ├─ [load all transactions for the account → derive data span (PRF-040)]
          ├─ [load price history for each held asset (AssetService)]
          │
          ├─ For each period in the span (month or year):
          │     ├─ replay transactions ≤ period end → units held + cash (PRF-021, PRF-023)
          │     ├─ value each holding at most-recent price ≤ period end, else 0 (PRF-022, PRF-024)
          │     ├─ end_value = cash + Σ holding values (PRF-020)
          │     ├─ net external flow in period = Σ Deposit − Σ Withdrawal (+OpeningBalance) (PRF-030)
          │     └─ gain + Simple Dietz % for period-over-period / YTD / since-inception (PRF-031–035)
          │
          └─ [Frontend: view-mode toggle + (month view) year selector]
             [Frontend: table rows, most-recent first (PRF-041)]
             [Frontend: loading / empty / error states (PRF-050–052)]
```

---

## UX Draft

### Entry Point

A "Performance" action in the Account Details header (alongside the existing "Refresh prices" action), per PRF-010.

### Main Component

A full-page table at `/accounts/:id/performance`.

- **Header**: account name, a view-mode toggle (Month / Year — Month hidden or disabled when ineligible per PRF-013), and, in month view, a year selector (PRF-015).
- **Table columns**:
  - Period (month label in month view, year in year view)
  - End value (account currency)
  - Period-over-period — gain + %
  - Year-to-date — gain + % _(month view only, PRF-037)_
  - Since inception — gain + %

### States

- **Loading**: table skeleton (PRF-050).
- **Empty**: "No performance data yet" + Add Transaction affordance (PRF-051).
- **Error**: generic message + Retry (PRF-052).
- **Absent metric**: "—" in the affected gain/% cell (PRF-042, PRF-036).

### User Flow

1. User opens an account and clicks "Performance" in the header.
2. The page opens in the default view (PRF-014): month view of the current year for sub-monthly accounts, else year view.
3. User reads the end value and the period-over-period / YTD / since-inception figures per row.
4. In month view, the user changes the year via the selector (PRF-015) or switches to year view (PRF-011).
5. Recording a transaction or price elsewhere refreshes the page automatically (PRF-060).

---

## Open Questions

None — all questions have been resolved.
