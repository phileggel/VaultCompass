# Contract — Account

> Domain: `account`
> Last updated by: `account` spec, `financial-asset-transaction` spec, `sell-transaction` spec, `transaction-list` spec, `account-details` spec, `cash-tracking` spec, `cash-dividend` spec

> **Error model on the wire**: each command's error serializes as a flat `{ code: "VariantName", ...payload }` object. The FE matches on `code`. Per-command reachable codes are listed in the "Errors" column of each table below. Infrastructure failures surface as `{ code: "DatabaseError" }` (no payload; diagnostic chain preserved server-side via `tracing::error!`).
>
> Rust-internal type organization (per-BC enums, use-case composites, serde tagging) is out of scope for this contract — it documents the BE↔FE frontier, not Rust internals.

---

## Commands

### Account CRUD

| Command                        | Args                                                                                                 | Return                   | Errors                                                                                                          |
| ------------------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------- |
| `get_accounts`                 | —                                                                                                    | `Vec<Account>`           | `DatabaseError`                                                                                                 |
| `add_account`                  | `CreateAccountDTO { name: String, currency: String, update_frequency: UpdateFrequency }`             | `Account`                | `NameEmpty` (ACC-002), `NameAlreadyExists` (ACC-003), `InvalidCurrency { currency }` (TRX-021), `DatabaseError` |
| `update_account`               | `UpdateAccountDTO { id: String, name: String, currency: String, update_frequency: UpdateFrequency }` | `Account`                | `NameEmpty` (ACC-002), `NameAlreadyExists` (ACC-003), `InvalidCurrency { currency }` (TRX-021), `DatabaseError` |
| `delete_account`               | `id: String`                                                                                         | `()`                     | `DatabaseError` _(ACC-005, ACC-006 — plain DELETE, silent on missing row)_                                      |
| `get_account_deletion_summary` | `account_id: String`                                                                                 | `AccountDeletionSummary` | `DatabaseError` _(read-only; counts are 0 if account has no data — no NotFound raised)_                         |

### Account Details

> `get_account_details` is implemented in `use_cases/account_details/` — it reads from both the
> account and asset BCs but mutates neither; owned here as the account aggregate is the primary subject.

| Command               | Args                 | Return                   | Errors                                                                                                                            |
| --------------------- | -------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `get_account_details` | `account_id: String` | `AccountDetailsResponse` | `AccountNotFound { account_id }` (ACD-012), `DatabaseError` (ACD-038); price lookup failures silently degrade to `None` (MKT-031) |

### Account Summaries

> `get_account_summaries` is implemented in `use_cases/account_summary/` — it reads from both the
> account and asset BCs (price lookups for each account's holdings) but mutates neither.

| Command                 | Args | Return                | Errors                                                                              |
| ----------------------- | ---- | --------------------- | ----------------------------------------------------------------------------------- |
| `get_account_summaries` | —    | `Vec<AccountSummary>` | `DatabaseError`; price lookup failures silently contribute 0 to the value (MKT-031) |

### Account Performance

> `get_account_performance` is implemented in `use_cases/account_performance/` — it reads from
> both the account BC (transaction replay → as-of-date holdings + cash) and the asset BC (price
> history) but mutates neither; owned here as the account aggregate is the primary subject. Period
> values and metrics are recomputed on read per ADR-013; nothing is persisted.

| Command                   | Args                 | Return                       | Errors                                                                                                                       |
| ------------------------- | -------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `get_account_performance` | `account_id: String` | `AccountPerformanceResponse` | `AccountNotFound { account_id }` (PRF-016), `DatabaseError` (PRF-027); price-lookup failures silently contribute 0 (PRF-022) |

### Holdings & Transactions

> Read paths (`get_asset_ids_for_account`, `get_transactions`) and mutation paths (`buy_holding`,
> `sell_holding`, `correct_transaction`, `cancel_transaction`, `open_holding`) live behind a
> single FE-visible surface. Mutation commands coordinate across the account and asset BCs
> (cash-asset seeding, archived-asset guards, etc.).

| Command                     | Args                                                    | Return             | Errors                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------- | ------------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_asset_ids_for_account` | `account_id: String`                                    | `Vec<String>`      | `DatabaseError` (TXL-054) — returns empty list for unknown or empty account, never NotFound (TXL-013)                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `get_transactions`          | `account_id: String, asset_id: String`                  | `Vec<Transaction>` | `DatabaseError` (TXL-020)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `buy_holding`               | `BuyHoldingDTO`                                         | `Transaction`      | `AccountNotFound { account_id }` (TRX-020), `InvalidDate` (TRX-020), `DateInFuture` (TRX-020), `DateTooOld` (TRX-020), `QuantityNotPositive` (TRX-020), `UnitPriceNegative` (TRX-020), `ExchangeRateNotPositive` (TRX-020), `FeesNegative` (TRX-020), `TotalAmountNotPositive` (TRX-020), `InsufficientCash { current_balance_micros, currency }` (CSH-041), `DatabaseError`                                                                                                                                          |
| `sell_holding`              | `SellHoldingDTO`                                        | `Transaction`      | `AccountNotFound { account_id }` (TRX-020), `InvalidDate` (TRX-020), `DateInFuture` (TRX-020), `DateTooOld` (TRX-020), `QuantityNotPositive` (TRX-020), `UnitPriceNegative` (TRX-020), `ExchangeRateNotPositive` (TRX-020), `FeesNegative` (SEL-020), `TotalAmountNotPositive` (TRX-020), `ClosedPosition` (SEL-012), `Oversell { available, requested }` (SEL-021), `DatabaseError`                                                                                                                                  |
| `correct_transaction`       | `id: String, account_id: String, CorrectTransactionDTO` | `Transaction`      | `TransactionNotFound` (TRX-031), `AccountNotFound { account_id }` (TRX-031), `InvalidDate` (TRX-033), `DateInFuture` (TRX-033), `DateTooOld` (TRX-033), `QuantityNotPositive` (TRX-033), `UnitPriceNegative` (TRX-033), `ExchangeRateNotPositive` (TRX-033), `FeesNegative` (TRX-033), `TotalAmountNotPositive` (TRX-033), `CascadingOversell` (SEL-032), `InsufficientCash { current_balance_micros, currency }` (CSH-042 / CSH-051 / DIV-040 — dividend edit re-applies the cash credit on replay), `DatabaseError` |
| `cancel_transaction`        | `id: String, account_id: String`                        | `()`               | `TransactionNotFound` (TRX-034), `AccountNotFound { account_id }` (TRX-034), `CascadingOversell` (SEL-033 — replay after cancel can leave a later sell oversold), `InsufficientCash { current_balance_micros, currency }` (CSH-024 / CSH-051 / DIV-041 — deleting a dividend removes a cash credit, which can underflow a later debit on replay), `DatabaseError`                                                                                                                                                     |
| `open_holding`              | `OpenHoldingDTO`                                        | `Transaction`      | `AccountNotFound { account_id }` (TRX-056), `AssetNotFound` (TRX-056), `ArchivedAsset` (TRX-050), `OpeningBalanceOnCashAsset` (CSH-061), `QuantityNotPositive` (TRX-044), `InvalidTotalCost` (TRX-045), `InvalidDate` (TRX-046), `DateInFuture` (TRX-046), `DateTooOld` (TRX-046), `DatabaseError`                                                                                                                                                                                                                    |

### Cash Transactions

> Both commands record cash-only movements (no asset selector, no unit price, no exchange rate)
> and return the persisted `Transaction` so the frontend can mirror the buy/sell flow's success
> path. Edit / delete of Deposit and Withdrawal reuse `correct_transaction` and
> `cancel_transaction` (CSH-023 / CSH-033 / CSH-024 / CSH-034) — those commands accept any
> `transaction_type` and run the chronological replay across all cash-affecting transactions for
> the account.

| Command             | Args                                                                                           | Return        | Errors                                                                                                                                                                                                                                   |
| ------------------- | ---------------------------------------------------------------------------------------------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `record_deposit`    | `DepositDTO { account_id: String, date: String, amount_micros: i64, note: Option<String> }`    | `Transaction` | `AccountNotFound { account_id }` (CSH-021), `AmountNotPositive` (CSH-021), `InvalidDate` (CSH-021), `DateInFuture` (CSH-021), `DateTooOld` (CSH-021), `DatabaseError`                                                                    |
| `record_withdrawal` | `WithdrawalDTO { account_id: String, date: String, amount_micros: i64, note: Option<String> }` | `Transaction` | `AccountNotFound { account_id }` (CSH-031), `AmountNotPositive` (CSH-031), `InvalidDate` (CSH-031), `DateInFuture` (CSH-031), `DateTooOld` (CSH-031), `InsufficientCash { current_balance_micros, currency }` (CSH-080), `DatabaseError` |

### Dividend

> `record_dividend` records a cash dividend attributed to the **paying asset** (`asset_id` is that
> asset, not the Cash Asset) and credits the account's Cash Holding by the account-currency total —
> mirroring how Sell proceeds re-link to cash (CSH-050/012), but leaving the paying asset's holding
> quantity and cost basis untouched (DIV-024). It only ever credits cash, so it carries **no**
> `InsufficientCash` variant. Edit / delete reuse `correct_transaction` / `cancel_transaction`
> (DIV-040/041) — a dividend **delete** can surface `InsufficientCash` on replay (removing the credit
> may underflow a later debit), via `cancel_transaction`'s existing variant.

| Command           | Args          | Return        | Errors                                                                                                                                                                                                                                                                                                                                                                     |
| ----------------- | ------------- | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `record_dividend` | `DividendDTO` | `Transaction` | `AccountNotFound { account_id }` (DIV-011), `AssetNotFound` (DIV-011), `AssetNotHeld` (DIV-011 — no active holding, `quantity = 0` or never held), `DividendOnCashAsset` (DIV-011 — asset is a Cash Asset), `AmountNotPositive` (DIV-021), `InvalidDate` (DIV-021), `DateInFuture` (DIV-021), `DateTooOld` (DIV-021), `ExchangeRateNotPositive` (DIV-022), `DatabaseError` |

---

## Shared Types

```rust
struct Account {
    id: String,                          // unique identifier
    name: String,                        // user-defined display name (normalised, unique)
    currency: String,                    // ISO 4217 currency code (TRX-021)
    update_frequency: UpdateFrequency,   // how often the user plans to update data
}

enum UpdateFrequency {
    Automatic,
    ManualDay,
    ManualWeek,
    ManualMonth,
    ManualYear,
}

struct AccountDeletionSummary {
    holding_count: u32,       // active holdings in the account
    transaction_count: u32,   // transactions associated with the account
}
```

```rust
// Purchase: type is implicit in the command — no transaction_type field
struct BuyHoldingDTO {
    account_id: String,
    asset_id: String,
    date: String,           // ISO date YYYY-MM-DD
    quantity: i64,          // micro-units; strictly positive (TRX-020)
    unit_price: i64,        // micro-units, asset currency; zero or positive (TRX-020)
    exchange_rate: i64,     // micro-units; strictly positive (TRX-020)
    fees: i64,              // micro-units, account currency; zero or positive (TRX-020)
    note: Option<String>,
}

// Sell: identical fields, separate type — may diverge as sell-specific rules grow
struct SellHoldingDTO {
    account_id: String,
    asset_id: String,
    date: String,
    quantity: i64,
    unit_price: i64,
    exchange_rate: i64,
    fees: i64,              // micro-units, account currency; zero or positive (SEL-020)
    note: Option<String>,
}

// Correction: no account_id / asset_id / type — those are immutable on an existing transaction
struct CorrectTransactionDTO {
    date: String,
    quantity: i64,
    unit_price: i64,
    exchange_rate: i64,
    fees: i64,
    note: Option<String>,
}
```

> `total_amount` intentionally absent from input DTOs — computed by backend (TRX-026, SEL-023).
> `realized_pnl` intentionally absent — computed by backend (SEL-024).

```rust
// Opening balance: total_cost set directly by user; no fees, no exchange_rate (TRX-047); no note (TRX-043)
struct OpenHoldingDTO {
    account_id: String,
    asset_id: String,
    date: String,       // ISO date YYYY-MM-DD; must not be future or before 1900-01-01 (TRX-046)
    quantity: i64,      // micro-units; strictly positive (TRX-044)
    total_cost: i64,    // micro-units, account currency; strictly positive (TRX-045)
}

// Cash inflow from outside the application (CSH-020/022). Backend resolves the Cash Asset
// for `account.currency`; user does not pick an asset.
struct DepositDTO {
    account_id: String,
    date: String,           // ISO date YYYY-MM-DD; same TRX-020 / CSH-021 bounds as buy/sell
    amount_micros: i64,     // micro-units, account currency; strictly positive (CSH-021)
    note: Option<String>,
}

// Cash outflow to outside the application (CSH-030/032). Same shape as Deposit; eligibility
// (CSH-080) checked against current Cash Holding balance.
struct WithdrawalDTO {
    account_id: String,
    date: String,
    amount_micros: i64,
    note: Option<String>,
}

// Cash dividend paid by a held asset (DIV-020/022). `asset_id` is the PAYING asset; the backend
// credits the account's Cash Holding by `amount_micros × exchange_rate` (account currency) and
// leaves the paying asset's holding untouched (DIV-023/024). No quantity, no unit price, no fees.
struct DividendDTO {
    account_id: String,
    asset_id: String,       // the paying asset (must be an active, non-cash holding — DIV-011)
    date: String,           // ISO date YYYY-MM-DD; same TRX-020 / DIV-021 bounds as buy/sell
    amount_micros: i64,     // micro-units, ASSET currency; net dividend received; strictly positive (DIV-021)
    exchange_rate: i64,     // micro-units, asset→account rate; strictly positive; 1_000_000 when currencies match (DIV-022)
    note: Option<String>,
}
```

```rust
enum TransactionType {
    Purchase,
    Sell,
    OpeningBalance,  // TRX-042
    Deposit,         // CSH-022 — cash inflow
    Withdrawal,      // CSH-032 — cash outflow
    Dividend,        // DIV-023 — cash income; attributed to the paying asset, credits cash
}

// Returned by buy_holding, sell_holding, correct_transaction, open_holding, record_deposit,
// record_withdrawal, and get_transactions.
//
// For Deposit/Withdrawal: asset_id is always the Cash Asset for account.currency;
// quantity == total_amount; unit_price == 1_000_000 (cash is its own unit); exchange_rate ==
// 1_000_000; fees == 0 (v1); realized_pnl is None.
// For Dividend (DIV-023): asset_id is the PAYING asset (not the Cash Asset); total_amount is the
// account-currency cash credited; exchange_rate is the asset→account rate (DIV-022); fees == 0;
// realized_pnl is None (income, not a capital gain — DIV-024). quantity/unit_price carry no
// business meaning (fixed convention).
struct Transaction {
    id: String,
    account_id: String,
    asset_id: String,
    transaction_type: TransactionType,
    date: String,                   // ISO date YYYY-MM-DD
    quantity: i64,                  // micro-units (TRX-024)
    unit_price: i64,                // micro-units, asset currency (TRX-021)
    exchange_rate: i64,             // micro-units, asset→account rate (TRX-021)
    fees: i64,                      // micro-units, account currency
    total_amount: i64,              // micro-units, account currency — computed by backend (TRX-026, SEL-023)
    realized_pnl: Option<i64>,      // micros; Some only for Sell (SEL-024); None for Purchase/OpeningBalance
    note: Option<String>,           // optional user comment; None when absent
    created_at: String,             // ISO 8601 timestamp; chronological tie-breaking (TRX-036, SEL-024)
}
```

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
    dividends_received: i64,            // micros, account currency; sum of dividend cash for this (account, asset); 0 when none; always computable (DIV-070)
    total_return_pct: Option<i64>,      // micros; (unrealized_pnl + dividends_received) × 100 / cost_basis; None under the same conditions as performance_pct (DIV-071)
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
    holdings: Vec<HoldingDetail>,              // active (quantity > 0), includes Cash Holding when present and qty > 0 (CSH-090, CSH-097); includes archived assets (ACD-020, ACD-021), sorted by asset_name asc (ACD-033)
    closed_holdings: Vec<ClosedHoldingDetail>, // closed, sorted by asset_name asc (ACD-046); empty list when none
    total_holding_count: i64,                  // all holdings regardless of quantity (ACD-034)
    total_cost_basis: i64,                     // micros, sum of cost_basis across active non-cash holdings (ACD-031, CSH-093)
    total_realized_pnl: i64,                   // micros, sum of total_realized_pnl across all holdings (ACD-045)
    total_unrealized_pnl: Option<i64>,         // micros; sum across same-currency priced active holdings; None when none qualify (MKT-040)
    total_global_value: i64,                   // micros, account currency: cash_holding.quantity + Σ_h (h.quantity × latest_price(h)) over non-cash active holdings; unpriced non-cash holdings contribute 0 (CSH-094)
    total_dividends_received: i64,             // micros, account currency: sum of dividend cash across all the account's dividend transactions; 0 when none (DIV-073)
}

// Row returned by get_account_summaries (ACC-021)
struct AccountSummary {
    id: String,
    name: String,
    currency: String,                          // ISO 4217 currency code; same as Account.currency
    update_frequency: UpdateFrequency,
    total_global_value: i64,                   // micros, account currency: same algorithm as AccountDetailsResponse.total_global_value (CSH-094)
}
```

```rust
// Net-of-flows performance for one period; Simple Dietz percentage (PRF-031, PRF-032)
struct PerformanceMetric {
    gain: i64,          // micros, account currency
    pct: Option<i64>,   // micro-percent (8.00% = 8_000_000); None when the Dietz denominator is 0 (PRF-032)
}

// One calendar period row — a month or a year (PRF-020, PRF-040)
struct PerformancePeriod {
    year: i32,
    month: Option<u8>,                              // Some(1..=12) for month rows; None for year rows (PRF-011)
    end_value: i64,                                 // micros, account currency; Global Value at period end (PRF-020)
    period_over_period: Option<PerformanceMetric>,  // None when no preceding period exists (PRF-033, PRF-042)
    year_to_date: Option<PerformanceMetric>,        // None for year rows (PRF-037); always Some for month rows — inception-year months use baseline 0, equal to since-inception (PRF-034)
    since_inception: Option<PerformanceMetric>,     // measured from net invested; inception baseline value 0 (PRF-035)
}

// Top-level response for get_account_performance — recomputed on read (ADR-013)
struct AccountPerformanceResponse {
    account_name: String,
    currency: String,                   // account's own ISO 4217 currency
    month_view_available: bool,         // true only for Automatic/ManualDay/ManualWeek (PRF-013)
    yearly: Vec<PerformancePeriod>,     // one per year, most-recent first (PRF-041); month is None
    monthly: Vec<PerformancePeriod>,    // one per month over the full span, most-recent first; empty when month_view_available is false (PRF-013, PRF-015)
}
```

---

## Events

### Published

| Event                | Payload | Rule             |
| -------------------- | ------- | ---------------- |
| `AccountUpdated`     | —       | ACC-022          |
| `TransactionUpdated` | —       | TRX-037, DIV-026 |

### Subscribed (frontend re-fetch triggers)

| Event                | Payload | Rule                      |
| -------------------- | ------- | ------------------------- |
| `AccountUpdated`     | —       | ACC-021, PRF-060          |
| `TransactionUpdated` | —       | ACD-039, ACC-021, PRF-060 |
| `AssetUpdated`       | —       | ACD-040                   |
| `AssetPriceUpdated`  | —       | MKT-036, PRF-060          |

---

## Changelog

- 2026-05-29 — Added by `account-performance` spec: `get_account_performance` (+ `AccountPerformanceResponse`, `PerformancePeriod`, `PerformanceMetric` types; PRF-060 re-uses existing subscribed events)
- 2026-05-31 — Added by `cash-dividend` spec: `record_dividend` (+ `DividendDTO`); `TransactionType::Dividend` variant; `HoldingDetail.dividends_received` + `.total_return_pct`; `AccountDetailsResponse.total_dividends_received`; edit/delete reuse `correct_transaction`/`cancel_transaction` (DIV-040/041)
