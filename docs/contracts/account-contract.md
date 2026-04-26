# Contract — Account

> Domain: account
> Last updated by: account spec

> **Error model**: commands return `Result<T, AccountCommandError>` — errors are typed Rust enums
> serialized as `{ code: "VariantName" }` (discriminated union, `#[serde(tag = "code")]`).

## Commands

| Command                        | Args                                                                                                 | Return                   | Errors                                                                         |
| ------------------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------ |
| `get_accounts`                 | —                                                                                                    | `Vec<Account>`           | `DbError`                                                                      |
| `add_account`                  | `CreateAccountDTO { name: String, currency: String, update_frequency: UpdateFrequency }`             | `Account`                | `NameEmpty (ACC-002)`, `NameAlreadyExists (ACC-003)`, `InvalidCurrency (TRX-021)` |
| `update_account`               | `UpdateAccountDTO { id: String, name: String, currency: String, update_frequency: UpdateFrequency }` | `Account`                | `NameEmpty (ACC-002)`, `NameAlreadyExists (ACC-003)`, `InvalidCurrency (TRX-021)` |
| `delete_account`               | `id: String`                                                                                         | `()`                     | `DbError (ACC-005, ACC-006)` *(no NotFound — plain DELETE, silent on missing row)* |
| `get_account_deletion_summary` | `account_id: String`                                                                                 | `AccountDeletionSummary` | `NotFound (ACC-020)` *(not yet implemented — ACC-020 is planned, see spec OQ-1; must live in `use_cases/` per ADR-003/004)* |

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

## Events

*No events — account mutations do not publish domain events.*

## Changelog

- 2026-04-26 — Added by `account` spec: get_accounts, add_account, update_account, delete_account, get_account_deletion_summary
- 2026-04-26 — Fixed: added InvalidCurrency error (TRX-021); removed phantom NotFound from delete_account and update_account; clarified error typing note
- 2026-04-26 — Typed errors: commands now return `AccountCommandError` discriminated union instead of `String`
