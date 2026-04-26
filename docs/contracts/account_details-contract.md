# Contract — Account Details

> Domain: `account_details`
> Last updated by: `account-details` spec

> **Error model note:** All Tauri commands currently return `Result<T, String>` — errors are
> `anyhow::Error` converted via `.map_err(|e| e.to_string())`. The Errors column documents the
> semantic content of those strings. When the structured error type refactor lands (see todo:
> "Replace string-matching error assertions with structured error types"), error variants here
> should be replaced with typed enum variants.

---

## Commands

| Command               | Args                 | Return                   | Errors                                                                                                                                                             |
| --------------------- | -------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `get_account_details` | `account_id: String` | `AccountDetailsResponse` | `String` — `"account not found"` (ACD-012); any DB or service failure (ACD-038); price lookup failures are silently swallowed — field degrades to `None` (MKT-031) |

---

## Shared Types

```rust
// Active position — quantity > 0 (ACD-020)
struct HoldingDetail {
    asset_id: String,
    asset_name: String,
    asset_reference: String,
    quantity: i64,                      // micros, always > 0
    average_price: i64,                 // micros, VWAP
    cost_basis: i64,                    // micros, quantity × average_price (ACD-023)
    realized_pnl: i64,                  // micros, cumulative from partial sells; 0 if none (SEL-042)
    asset_currency: String,             // ISO 4217 code of the asset's native currency (MKT-023)
    current_price: Option<i64>,         // micros in asset currency; None when no price ever recorded (MKT-031)
    current_price_date: Option<String>, // ISO date of the price observation; None when current_price is None (MKT-031)
    unrealized_pnl: Option<i64>,        // micros in account currency; None on currency mismatch or no price; 0 (not None) when price == avg_price (MKT-033/034)
    performance_pct: Option<i64>,       // micros (5.25% = 5_250_000); None when unrealized_pnl is None or cost_basis = 0; 0 (not None) when unrealized_pnl is 0 (MKT-035)
}

// Closed position — quantity = 0 (ACD-044)
struct ClosedHoldingDetail {
    asset_id: String,
    asset_name: String,
    asset_reference: String,
    realized_pnl: i64,      // micros, total gain/loss for this position (ACD-045)
    last_sold_date: String, // ISO date "YYYY-MM-DD"; non-optional in this DTO (ACD-043)
}

// Top-level response for get_account_details
struct AccountDetailsResponse {
    account_name: String,
    holdings: Vec<HoldingDetail>,              // active (quantity > 0), includes archived assets (ACD-020, ACD-021), sorted by asset_name asc (ACD-033)
    closed_holdings: Vec<ClosedHoldingDetail>, // closed, sorted by asset_name asc (ACD-046); empty list when none
    total_holding_count: i64,                  // all holdings regardless of quantity (ACD-034)
    total_cost_basis: i64,                     // micros, sum of cost_basis across active holdings (ACD-031)
    total_realized_pnl: i64,                   // micros, sum of total_realized_pnl across all holdings (ACD-045)
    total_unrealized_pnl: Option<i64>,         // micros; sum across same-currency priced active holdings; None when none qualify (MKT-040)
}
```

---

## Events

| Event                | Payload | Direction                                     |
| -------------------- | ------- | --------------------------------------------- |
| `TransactionUpdated` | none    | subscribed — triggers full re-fetch (ACD-039) |
| `AssetUpdated`       | none    | subscribed — triggers full re-fetch (ACD-040) |
| `AssetPriceUpdated`  | none    | subscribed — triggers full re-fetch (MKT-036) |

---

## Changelog

- 2026-04-25 — Added by `account-details` spec: `get_account_details`
- 2026-04-26 — Extended by `market-price` spec: HoldingDetail +5 fields, AccountDetailsResponse +total_unrealized_pnl, AssetPriceUpdated event subscription
