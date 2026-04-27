# Contract — Asset

> Domain: `asset`
> Last updated by: `market-price` spec

> **Error model**: commands return typed error enums serialized as `{ code: "VariantName" }`:
>
> - Asset CRUD: `AssetCommandError` — `NameEmpty`, `ReferenceEmpty`, `InvalidRiskLevel`, `InvalidCurrency`, `Archived`, `NotFound`, `CategoryNotFound`, `Unknown`
> - Categories: `CategoryCommandError` — `LabelEmpty`, `DuplicateName`, `SystemReadonly`, `SystemProtected`, `Unknown`
> - `record_asset_price`: `AssetPriceCommandError` — `NotPositive`, `NonFinite`, `DateInFuture`, `Unknown`
> - `archive_asset`: `ArchiveAssetCommandError` — `ActiveHoldings`, `Unknown`
> - `delete_asset`: `DeleteAssetCommandError` — `ExistingTransactions`, `Unknown`

---

## Commands

| Command              | Args                                         | Return | Errors                                                                                                              |
| -------------------- | -------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| `record_asset_price` | `asset_id: String, date: String, price: f64` | `()`   | `PriceNotPositive` (MKT-021), `InvalidDate` (MKT-022), `FutureDate` (MKT-022), `AssetNotFound` (MKT-043), `DbError` |

---

## Shared Types

```rust
// No shared types — all args are primitives.
// price is transmitted as f64 decimal; backend converts to i64 micros at the IPC boundary (MKT-024).
// ADR-001 (i64 micros) applies to storage — f64 on the wire is the intentional transport-layer exception;
// the f64 → i64 conversion inside the command handler is the ADR-001 compliance point.
```

---

## Events

| Event               | Payload | Direction                                                                                                                                                                                                            |
| ------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `AssetPriceUpdated` | none    | published — fired after successful upsert via `record_asset_price` (MKT-026) **or** via the auto-record path on `add_transaction` / `update_transaction` when `record_price = true` and `tx.unit_price > 0` (MKT-057) |

---

## Changelog

- 2026-04-26 — Added by `market-price` spec: `record_asset_price`
- 2026-04-26 — Typed errors: all commands now return domain-specific typed error enums instead of `String`
- 2026-04-26 — Added `CategoryNotFound` to `AssetCommandError` (raised when asset create/update references a nonexistent category)
- 2026-04-27 — Updated by `market-price` spec (MKT-050+): `AssetPriceUpdated` event now also fires from the auto-record path on `add_transaction` / `update_transaction`; no new commands
