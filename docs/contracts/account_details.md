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

| Command               | Args                 | Return                   | Errors                                                                 |
| --------------------- | -------------------- | ------------------------ | ---------------------------------------------------------------------- |
| `get_account_details` | `account_id: String` | `AccountDetailsResponse` | `String` — `"account not found"` (ACD-012); any DB or service failure (ACD-038) |

---

## Shared Types

```rust
// Active position — quantity > 0 (ACD-020)
struct HoldingDetail {
    asset_id: String,
    asset_name: String,
    asset_reference: String,
    quantity: i64,        // micros, always > 0
    average_price: i64,   // micros, VWAP
    cost_basis: i64,      // micros, quantity × average_price (ACD-023)
    realized_pnl: i64,    // micros, cumulative from partial sells; 0 if none (SEL-042)
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
}
```

---

## Events

| Event                | Payload | Direction                                          |
| -------------------- | ------- | -------------------------------------------------- |
| `TransactionUpdated` | none    | subscribed — triggers full re-fetch (ACD-039)      |
| `AssetUpdated`       | none    | subscribed — triggers full re-fetch (ACD-040)      |

---

## Changelog

- 2026-04-25 — Added by `account-details` spec: `get_account_details`
