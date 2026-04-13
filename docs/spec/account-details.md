# Business Rules — Account Details (ACD)

## Context

The `Account Details` feature provides a comprehensive view of a specific account's composition and financial performance. It allows users to drill down into an account to see their current positions (Holdings), the valuation of each position based on the latest market prices, and the overall health of the account.

This feature consumes data from multiple contexts: `account` (for Account and Holding data), `asset` (for asset metadata and latest prices), and indirectly `transaction` (which feeds the Holding data via the TRX use case).

---

## Entity Definition

### Holding (Position)

Represents the current state of a financial position within the account.

| Field           | Business meaning                                                         |
| --------------- | ------------------------------------------------------------------------ |
| `asset_id`      | The financial asset held.                                                |
| `quantity`      | Current number of units held (i64 micros).                               |
| `average_price` | Volume-weighted average purchase price in account currency (i64 micros). |

### HoldingPerformance (View Model)

A computed view of a holding including market valuation.

| Field                | Business meaning                                              |
| -------------------- | ------------------------------------------------------------- |
| `current_price`      | Latest recorded price for the asset (in account currency).    |
| `market_value`       | Total valuation of the position (`quantity * current_price`). |
| `cost_basis`         | Total cost of the position (`quantity * average_price`).      |
| `unrealized_gain`    | Absolute gain or loss (`market_value - cost_basis`).          |
| `unrealized_gain_pc` | Percentage gain or loss (`unrealized_gain / cost_basis`).     |

---

## Business Rules

### Navigation

**ACD-005 — Dependencies (frontend)**: This feature relies on the account management view defined in [Account Management](account.md) (specifically rule ACC-050) for its entry point.

**ACD-010 — View entry point (frontend)**: The Account Details view is accessed by clicking on an account row in the main Account Table.

**ACD-011 — Account selection persistence (frontend)**: The selected account ID is reflected in the URL/Routing to allow direct linking and browser "Back" navigation.

### Holding List and Valuation

**ACD-020 — Holding aggregation (backend)**: The view displays all holdings (where `quantity > 0`, including archived assets) for the selected account.

**ACD-021 — Latest price resolution (backend)**: For each holding, the system retrieves the most recent price record from the `AssetPrice` table.

**ACD-022 — Fallback price resolution (backend)**: If no market price is found for a holding, the `average_price` is used as a fallback for valuation.

**ACD-023 — Valuation calculation (backend)**: Market value for a holding is computed as `Holding.quantity * LatestPrice`.

**ACD-024 — Calculation precision (backend)**: Market value calculations use `i128` intermediates to prevent overflow before scaling back to `i64` micro-units.

**ACD-025 — Unrealized Gain/Loss (backend)**: Unrealized gain/loss is calculated as `market_value - cost_basis`.

**ACD-026 — Currency consistency (backend)**: All performance calculations are performed in the Account's base currency.

**ACD-027 — Exchange rate resolution (backend)**: If an asset is in a different currency, the latest price must be converted using the latest available market exchange rate. If no market rate is available, the exchange rate from the last transaction for that asset in the account is used as a fallback. If the transaction used for exchange rate fallback is deleted, the valuation defaults to 1.0.

### Account Performance

**ACD-030 — Total Account Value (backend)**: The total value of the account is the sum of the market values of all its holdings.

**ACD-031 — Total Account Performance (backend)**: The total performance is computed as the sum of all unrealized gains/losses divided by the sum of all cost bases.

**ACD-032 — Empty account state (frontend)**: If an account has no holdings, the view displays a clear "No holdings" message.

### Data Integrity and Reactivity

**ACD-040 — Reactivity to transactions (frontend + backend)**: The Account Details view refreshes its data when a `TransactionUpdated` event is received.

**ACD-041 — Reactivity to price updates (frontend + backend)**: The Account Details view refreshes its data when an `AssetUpdated` event is received.

**ACD-042 — Precision handling (backend)**: All financial calculations follow [ADR-001](../adr/001-use-i64-for-monetary-amounts.md). Percentage values are also stored as `i64`.

---

## Workflow

```
[User selects Account]
  → Route: /accounts/:id
          │
          ├─ [Backend: Fetch Account metadata]
          ├─ [Backend: Fetch Holdings for account]
          ├─ [Backend: Resolve latest prices for assets]
          ├─ [Backend: Compute individual performance]
          ├─ [Backend: Compute total performance]
          │
          └─ [Frontend: Display Header with Total Value & Perf]
             [Frontend: Display Table of HoldingPerformance rows]
```

---

## UX Draft

### Entry Point

- Clicking a row in the `AccountTable`.

### Main Component

**ManagerLayout** containing:

- **Header**: Account name + Total Value (big) + Total Performance badge (green/red).
- **Position Table**:
  - Asset (Name + Ticker)
  - Quantity
  - Avg. Price
  - Current Price
  - Market Value
  - Gain/Loss (absolute + percentage badge)

### States

- **Loading**: Skeleton screens for the header and table.
- **Empty**: Illustration "No assets yet" + "Add transaction" button.
- **Error**: Generic error message with "Retry" button.

### User Flow

1. User clicks on "Bank Account" in the sidebar/table.
2. The page loads the holdings.
3. User sees individual position valuations and total account performance.

---

## Open Questions

None — all questions have been resolved.
