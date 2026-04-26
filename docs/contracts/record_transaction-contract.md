# Contract — Record Transaction

> Domain: record_transaction (use case — cross-context: transaction + account + asset)
> Last updated by: financial-asset-transaction spec, sell-transaction spec, transaction-list spec

> **Error model**: all commands return `Result<T, TransactionCommandError>` — errors are typed enums
> serialized as `{ code: "VariantName" }` (plus `available`/`requested` fields for `Oversell`).
> Variants: `TransactionNotFound`, `AccountNotFound`, `AssetNotFound`, `InvalidType`, `TypeImmutable`,
> `ArchivedAssetSell`, `ClosedPosition`, `Oversell { available, requested }`, `CascadingOversell`,
> `InvalidDate`, `DateInFuture`, `DateTooOld`, `QuantityNotPositive`, `UnitPriceNegative`,
> `FeesNegative`, `ExchangeRateNotPositive`, `TotalAmountNotPositive`, `Unknown`.

## Commands

| Command              | Args                                   | Return             | Errors                                                                                                                                                                                                                                                                                                                                                                                                                     |
| -------------------- | -------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `add_transaction`    | `CreateTransactionDTO`                 | `Transaction`      | `InvalidTransactionType`, `AccountNotFound (TRX-020)`, `AssetNotFound (TRX-020)`, `InvalidDate (TRX-020)`, `QuantityNotPositive (TRX-020)`, `ExchangeRateNotPositive (TRX-020)`, `NegativeFees (SEL-020)`, `TotalAmountNotPositive (TRX-020)`, `ArchivedAssetSell (SEL-037)`, `ClosedPosition (SEL-012)`, `Oversell (SEL-021)`, `DbError (TRX-027)`                                                                        |
| `update_transaction` | `id: String, CreateTransactionDTO`     | `Transaction`      | `TransactionNotFound (TRX-031)`, `TypeImmutable (SEL-035)`, `AccountNotFound (TRX-033)`, `AssetNotFound (TRX-033)`, `InvalidDate (TRX-033)`, `QuantityNotPositive (TRX-033)`, `ExchangeRateNotPositive (TRX-033)`, `NegativeFees (TRX-033)`, `TotalAmountNotPositive (TRX-033)`, `ArchivedAssetSell (SEL-037)` _(guard absent in impl — spec mandates it via TRX-033)_, `CascadingOversell (SEL-032)`, `DbError (TRX-031)` |
| `delete_transaction` | `id: String`                           | `()`               | `TransactionNotFound (TRX-034)`, `DbError`                                                                                                                                                                                                                                                                                                                                                                                 |
| `get_transactions`   | `account_id: String, asset_id: String` | `Vec<Transaction>` | `DbError (TXL-020)`                                                                                                                                                                                                                                                                                                                                                                                                        |

## Shared Types

```rust
struct CreateTransactionDTO {
    account_id: String,
    asset_id: String,
    transaction_type: String,    // "Purchase" or "Sell"; immutable once saved (SEL-035)
    date: String,                // ISO date YYYY-MM-DD
    quantity: i64,               // micro-units; strictly positive (TRX-020)
    unit_price: i64,             // micro-units, asset currency; zero or positive (TRX-020)
    exchange_rate: i64,          // micro-units; strictly positive (TRX-020)
    fees: i64,                   // micro-units, account currency; zero or positive (SEL-020)
    note: Option<String>,        // optional user comment; None when absent
}
// total_amount intentionally absent — computed by backend (TRX-026, SEL-023)
// realized_pnl intentionally absent — computed by backend (SEL-024)
```

## Events

_Events are owned by the `transaction` bounded context — see `transaction-contract.md`._

## Changelog

- 2026-04-26 — Added by `financial-asset-transaction` + `sell-transaction` + `transaction-list` specs: add_transaction, update_transaction, delete_transaction, get_transactions
- 2026-04-26 — Fixed: added InvalidTransactionType, ExchangeRateNotPositive, NegativeFees to add_transaction; added full TRX-020 validation errors + ArchivedAssetSell to update_transaction; fixed note field to Option<String>
- 2026-04-26 — Typed errors: commands now return `TransactionCommandError` discriminated union instead of `String`
