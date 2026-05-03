# Contract — Account

> Domain: account
> Last updated by: account spec, financial-asset-transaction spec, sell-transaction spec, transaction-list spec

> **Error model (account CRUD)**: commands return `Result<T, AccountCommandError>` — errors are typed Rust enums
> serialized as `{ code: "VariantName" }` (discriminated union, `#[serde(tag = "code")]`).

> **Error model (holding operations)**: commands return `Result<T, TransactionCommandError>` — errors are typed enums
> serialized as `{ code: "VariantName" }` (plus `available`/`requested` fields for `Oversell`).
> Variants: `TransactionNotFound`, `AccountNotFound`, `AssetNotFound`,
> `ArchivedAssetSell`, `ArchivedAsset`, `ClosedPosition`, `Oversell { available, requested }`, `CascadingOversell`,
> `InvalidDate`, `DateInFuture`, `DateTooOld`, `QuantityNotPositive`, `UnitPriceNegative`,
> `FeesNegative`, `ExchangeRateNotPositive`, `TotalAmountNotPositive`, `InvalidTotalCost`, `Unknown`.

## Commands

### Account CRUD

| Command                        | Args                                                                                                 | Return                   | Errors                                                                             |
| ------------------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------- |
| `get_accounts`                 | —                                                                                                    | `Vec<Account>`           | `DbError`                                                                          |
| `add_account`                  | `CreateAccountDTO { name: String, currency: String, update_frequency: UpdateFrequency }`             | `Account`                | `NameEmpty (ACC-002)`, `NameAlreadyExists (ACC-003)`, `InvalidCurrency (TRX-021)`  |
| `update_account`               | `UpdateAccountDTO { id: String, name: String, currency: String, update_frequency: UpdateFrequency }` | `Account`                | `NameEmpty (ACC-002)`, `NameAlreadyExists (ACC-003)`, `InvalidCurrency (TRX-021)`  |
| `delete_account`               | `id: String`                                                                                         | `()`                     | `DbError (ACC-005, ACC-006)` _(no NotFound — plain DELETE, silent on missing row)_ |
| `get_account_deletion_summary` | `account_id: String`                                                                                 | `AccountDeletionSummary` | `Unknown` _(read-only; counts are 0 if account has no data — no NotFound raised)_  |

### Holdings & Transactions

> All commands below live in `context/account/` except `open_holding`, which is implemented in
> `use_cases/open_holding/` — it primarily mutates the account aggregate but also checks asset
> state across the asset BC.

| Command                     | Args                                   | Return             | Errors                                                                                                                                                                                                                                                                                                          |
| --------------------------- | -------------------------------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_asset_ids_for_account` | `account_id: String`                   | `Vec<String>`      | `DbError (TXL-054)` — returns empty list for unknown or empty account, never NotFound (TXL-013)                                                                                                                                                                                                                 |
| `buy_holding`               | `BuyHoldingDTO`                        | `Transaction`      | `AccountNotFound (TRX-020)`, `AssetNotFound (TRX-020)`, `InvalidDate (TRX-020)`, `QuantityNotPositive (TRX-020)`, `ExchangeRateNotPositive (TRX-020)`, `FeesNegative (TRX-020)`, `TotalAmountNotPositive (TRX-020)`, `DbError`                                                                                  |
| `sell_holding`              | `SellHoldingDTO`                       | `Transaction`      | `AccountNotFound (TRX-020)`, `AssetNotFound (TRX-020)`, `InvalidDate (TRX-020)`, `QuantityNotPositive (TRX-020)`, `ExchangeRateNotPositive (TRX-020)`, `FeesNegative (SEL-020)`, `TotalAmountNotPositive (TRX-020)`, `ArchivedAssetSell (SEL-037)`, `ClosedPosition (SEL-012)`, `Oversell (SEL-021)`, `DbError` |
| `correct_transaction`       | `id: String, CorrectTransactionDTO`    | `Transaction`      | `TransactionNotFound (TRX-031)`, `InvalidDate (TRX-033)`, `QuantityNotPositive (TRX-033)`, `ExchangeRateNotPositive (TRX-033)`, `FeesNegative (TRX-033)`, `TotalAmountNotPositive (TRX-033)`, `ArchivedAssetSell (SEL-037)`, `CascadingOversell (SEL-032)`, `DbError`                                           |
| `cancel_transaction`        | `id: String`                           | `()`               | `TransactionNotFound (TRX-034)`, `DbError`                                                                                                                                                                                                                                                                      |
| `get_transactions`          | `account_id: String, asset_id: String` | `Vec<Transaction>` | `DbError (TXL-020)`                                                                                                                                                                                                                                                                                             |
| `open_holding`              | `OpenHoldingDTO`                       | `Transaction`      | `AccountNotFound (TRX-056)`, `AssetNotFound (TRX-056)`, `ArchivedAsset (TRX-050)`, `QuantityNotPositive (TRX-044)`, `InvalidTotalCost (TRX-045)`, `DateInFuture (TRX-046)`, `DateTooOld (TRX-046)`, `DbError`                                                                                                   |

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
```

```rust
enum TransactionType {
    Purchase,
    Sell,
    OpeningBalance,  // TRX-042
}

// Returned by buy_holding, sell_holding, correct_transaction, open_holding, and get_transactions
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
    realized_pnl: Option<i64>,      // micro-units; Some only for Sell (SEL-024); None for Purchase/OpeningBalance
    note: Option<String>,           // optional user comment; None when absent
    created_at: String,             // ISO 8601 timestamp; chronological tie-breaking (TRX-036, SEL-024)
}
```

## Events

| Event            | Payload | Rule    |
| ---------------- | ------- | ------- |
| `AccountUpdated` | —       | TRX-037 |

## Changelog

- 2026-04-26 — Added by `account` spec: get_accounts, add_account, update_account, delete_account, get_account_deletion_summary
- 2026-04-26 — Fixed: added InvalidCurrency error (TRX-021); removed phantom NotFound from delete_account and update_account; clarified error typing note
- 2026-04-26 — Typed errors: commands now return `AccountCommandError` discriminated union instead of `String`
- 2026-04-28 — Added `AccountUpdated` event (previously undeclared; owned by Account BC per migration plan)
- 2026-05-03 — Merged from `record_transaction-contract.md` and `transaction-contract.md`: get_asset_ids_for_account, buy_holding, sell_holding, correct_transaction, cancel_transaction, get_transactions, open_holding; Transaction struct reconciled (added created_at, added OpeningBalance variant)
