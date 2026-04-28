# Contract — Holding Operations (Account BC)

> Domain: account (holding operations — commands move from `use_cases/record_transaction/` to `context/account/` in Phase 4 of the migration plan)
> Last updated by: financial-asset-transaction spec, sell-transaction spec, transaction-list spec, account-bc-migration-plan

> **Error model**: all commands return `Result<T, TransactionCommandError>` — errors are typed enums
> serialized as `{ code: "VariantName" }` (plus `available`/`requested` fields for `Oversell`).
> Variants: `TransactionNotFound`, `AccountNotFound`, `AssetNotFound`,
> `ArchivedAssetSell`, `ClosedPosition`, `Oversell { available, requested }`, `CascadingOversell`,
> `InvalidDate`, `DateInFuture`, `DateTooOld`, `QuantityNotPositive`, `UnitPriceNegative`,
> `FeesNegative`, `ExchangeRateNotPositive`, `TotalAmountNotPositive`, `Unknown`.

## Commands

| Command               | Args                               | Return             | Errors                                                                                                                                                                                          |
| --------------------- | ---------------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `buy_holding`         | `BuyHoldingDTO`                    | `Transaction`      | `AccountNotFound (TRX-020)`, `AssetNotFound (TRX-020)`, `InvalidDate (TRX-020)`, `QuantityNotPositive (TRX-020)`, `ExchangeRateNotPositive (TRX-020)`, `FeesNegative (TRX-020)`, `TotalAmountNotPositive (TRX-020)`, `DbError` |
| `sell_holding`        | `SellHoldingDTO`                   | `Transaction`      | `AccountNotFound (TRX-020)`, `AssetNotFound (TRX-020)`, `InvalidDate (TRX-020)`, `QuantityNotPositive (TRX-020)`, `ExchangeRateNotPositive (TRX-020)`, `FeesNegative (SEL-020)`, `TotalAmountNotPositive (TRX-020)`, `ArchivedAssetSell (SEL-037)`, `ClosedPosition (SEL-012)`, `Oversell (SEL-021)`, `DbError` |
| `correct_transaction` | `id: String, CorrectTransactionDTO` | `Transaction`      | `TransactionNotFound (TRX-031)`, `InvalidDate (TRX-033)`, `QuantityNotPositive (TRX-033)`, `ExchangeRateNotPositive (TRX-033)`, `FeesNegative (TRX-033)`, `TotalAmountNotPositive (TRX-033)`, `ArchivedAssetSell (SEL-037)`, `CascadingOversell (SEL-032)`, `DbError` |
| `cancel_transaction`  | `id: String`                       | `()`               | `TransactionNotFound (TRX-034)`, `DbError`                                                                                                                                                      |
| `get_transactions`    | `account_id: String, asset_id: String` | `Vec<Transaction>` | `DbError (TXL-020)`                                                                                                                                                                         |

## Shared Types

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

## Events

| Event            | Payload | Owned by   |
| ---------------- | ------- | ---------- |
| `AccountUpdated` | —       | Account BC |

## Changelog

- 2026-04-26 — Added by `financial-asset-transaction` + `sell-transaction` + `transaction-list` specs: add_transaction, update_transaction, delete_transaction, get_transactions
- 2026-04-26 — Fixed: added ExchangeRateNotPositive, FeesNegative; full TRX-020 validation errors; typed errors
- 2026-04-27 — Added `record_price: bool` field on `CreateTransactionDTO` (market-price spec MKT-050+)
- 2026-04-28 — Migration plan Phase 4: `add_transaction` split into `buy_holding` + `sell_holding`; `update_transaction` → `correct_transaction`; `delete_transaction` → `cancel_transaction`; `CreateTransactionDTO` retired (`transaction_type` and `record_price` removed); `TypeImmutable` error variant removed; event renamed to `AccountUpdated`
