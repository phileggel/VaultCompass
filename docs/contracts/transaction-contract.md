# Contract — Transaction

> Domain: transaction
> Last updated by: financial-asset-transaction spec, sell-transaction spec, transaction-list spec

## Commands

| Command                     | Args                 | Return        | Errors                                                                                          |
| --------------------------- | -------------------- | ------------- | ----------------------------------------------------------------------------------------------- |
| `get_asset_ids_for_account` | `account_id: String` | `Vec<String>` | `DbError (TXL-054)` — returns empty list for unknown or empty account, never NotFound (TXL-013) |

## Shared Types

```rust
struct Transaction {
    id: String,
    account_id: String,
    asset_id: String,
    transaction_type: TransactionType,
    date: String,                  // ISO date YYYY-MM-DD
    quantity: i64,                 // micro-units (TRX-024)
    unit_price: i64,               // micro-units, asset currency (TRX-021)
    exchange_rate: i64,            // micro-units, asset→account rate (TRX-021)
    fees: i64,                     // micro-units, account currency
    total_amount: i64,             // micro-units, computed by backend — never sent from frontend (TRX-026, SEL-023)
    realized_pnl: Option<i64>,     // micro-units; Some only for Sell (SEL-024); None for Purchase
    note: Option<String>,          // optional user comment; None when absent
    created_at: String,            // ISO 8601 timestamp; chronological tie-breaking (TRX-036, SEL-024)
}

enum TransactionType {
    Purchase,
    Sell,
}
```

## Events

| Event            | Payload | Owned by   | Rule    |
| ---------------- | ------- | ---------- | ------- |
| `AccountUpdated` | —       | Account BC | TRX-037 |

> `TransactionUpdated` renamed to `AccountUpdated` — Transaction merges into Account BC (migration plan Phase 1–4). The event is owned and emitted by `AccountService`.

## Changelog

- 2026-04-26 — Added by `financial-asset-transaction` + `sell-transaction` + `transaction-list` specs: get_asset_ids_for_account
- 2026-04-28 — `TransactionUpdated` → `AccountUpdated` (migration plan: Transaction moves into Account BC)
