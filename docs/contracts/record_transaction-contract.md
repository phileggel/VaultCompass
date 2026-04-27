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
| `add_transaction`    | `CreateTransactionDTO`                 | `Transaction`      | `InvalidTransactionType`, `AccountNotFound (TRX-020)`, `AssetNotFound (TRX-020)`, `InvalidDate (TRX-020)`, `QuantityNotPositive (TRX-020)`, `ExchangeRateNotPositive (TRX-020)`, `NegativeFees (SEL-020)`, `TotalAmountNotPositive (TRX-020)`, `ArchivedAssetSell (SEL-037)`, `ClosedPosition (SEL-012)`, `Oversell (SEL-021)`, `DbError (TRX-027, MKT-055/056/062)`                                                                        |
| `update_transaction` | `id: String, CreateTransactionDTO`     | `Transaction`      | `TransactionNotFound (TRX-031)`, `TypeImmutable (SEL-035)`, `AccountNotFound (TRX-033)`, `AssetNotFound (TRX-033)`, `InvalidDate (TRX-033)`, `QuantityNotPositive (TRX-033)`, `ExchangeRateNotPositive (TRX-033)`, `NegativeFees (TRX-033)`, `TotalAmountNotPositive (TRX-033)`, `ArchivedAssetSell (SEL-037)` _(guard absent in impl — spec mandates it via TRX-033)_, `CascadingOversell (SEL-032)`, `DbError (TRX-031, MKT-055/056/062)` |
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
    record_price: bool,          // MKT-054 — when true and unit_price > 0, the orchestrator also upserts
                                 //           AssetPrice(asset_id, date, unit_price) inside the same DB tx
                                 //           (MKT-055/056) and publishes AssetPriceUpdated after commit
                                 //           (MKT-057). Existing same-date price is silently overwritten
                                 //           (MKT-058). Skipped silently when unit_price = 0 (MKT-061).
}
// total_amount intentionally absent — computed by backend (TRX-026, SEL-023)
// realized_pnl intentionally absent — computed by backend (SEL-024)
```

## Side Effects

When `add_transaction` or `update_transaction` is called with `record_price = true` AND
`tx.unit_price > 0`, the orchestrator additionally upserts an `AssetPrice` row at
`(asset_id = tx.asset_id, date = tx.date, price = tx.unit_price)` inside the same DB
transaction as the transaction insert/update + holding recompute (MKT-055, MKT-056). On
successful commit, `AssetPriceUpdated` is published by the asset bounded context (MKT-057,
see `asset-contract.md`). Validation rules MKT-021 (price > 0) and MKT-022 (date not in
future) hold by construction: TRX-020 enforces `tx.date` not in future; MKT-061 silently
skips the upsert when `tx.unit_price = 0` (gifted/inherited assets, OQ-1). Conflicts on
`(asset_id, date)` are silently overwritten per MKT-058 / MKT-025 upsert semantics.
`AssetPrice` records are independent of the transaction lifecycle: editing the transaction
upserts at the *current* `tx.date` and `tx.unit_price` only, leaving prior price records
untouched (MKT-059); deleting the transaction does not cascade to any `AssetPrice` row
(MKT-060).

## Events

_Events are owned by the `transaction` bounded context — see `transaction-contract.md`._
_The auto-record path (MKT-057) additionally triggers `AssetPriceUpdated`, owned by the
`asset` bounded context — see `asset-contract.md`._

## Changelog

- 2026-04-26 — Added by `financial-asset-transaction` + `sell-transaction` + `transaction-list` specs: add_transaction, update_transaction, delete_transaction, get_transactions
- 2026-04-26 — Fixed: added InvalidTransactionType, ExchangeRateNotPositive, NegativeFees to add_transaction; added full TRX-020 validation errors + ArchivedAssetSell to update_transaction; fixed note field to Option<String>
- 2026-04-26 — Typed errors: commands now return `TransactionCommandError` discriminated union instead of `String`
- 2026-04-27 — Added by `market-price` spec (MKT-050+): `record_price: bool` field on `CreateTransactionDTO`; auto-record `AssetPrice` side-effect on `add_transaction` / `update_transaction`; `DbError` annotated with new MKT references (no new error variants — MKT-061 + TRX-020 preconditions exclude MKT-021/022/043 paths)
