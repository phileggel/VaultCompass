# Contract — Asset

> Domain: `asset`
> Last updated by: `market-price` spec, `asset-web-lookup` spec

> **Error model**: commands return typed error enums serialized as `{ code: "VariantName" }`; `lookup_asset` returns `Result<Vec<AssetLookupResult>, WebLookupCommandError>` (single variant: `NetworkError`):
>
> - Asset CRUD: `AssetCommandError` — `NameEmpty`, `ReferenceEmpty`, `InvalidRiskLevel`, `InvalidCurrency`, `Archived`, `CashAssetNotEditable`, `NotFound`, `CategoryNotFound`, `Unknown`
> - Categories: `CategoryCommandError` — `LabelEmpty`, `DuplicateName`, `SystemReadonly`, `SystemProtected`, `Unknown`
> - `record_asset_price`: `AssetPriceCommandError` — `AssetNotFound`, `NotPositive`, `NonFinite`, `DateInFuture`, `Unknown`
> - `get_asset_prices`: `AssetPriceCommandError` — `AssetNotFound`, `Unknown`
> - `update_asset_price`: `UpdateAssetPriceCommandError` — `NotFound`, `NotPositive`, `NonFinite`, `DateInFuture`, `Unknown`
> - `delete_asset_price`: `DeleteAssetPriceCommandError` — `NotFound`, `Unknown`
> - `archive_asset`: `ArchiveAssetCommandError` — `ActiveHoldings`, `CashAssetNotEditable`, `NotFound`, `Unknown`
> - `delete_asset`: `DeleteAssetCommandError` — `ExistingTransactions`, `CashAssetNotEditable`, `NotFound`, `Unknown`

---

## Commands

### Web Lookup

> `lookup_asset` is implemented in `use_cases/asset_web_lookup/` — it reads from an external web
> API and returns transient value objects; it does not persist anything. Owned here as the asset
> aggregate is the primary subject.

| Command        | Args            | Return                   | Errors         |
| -------------- | --------------- | ------------------------ | -------------- |
| `lookup_asset` | `query: String` | `Vec<AssetLookupResult>` | `NetworkError` |

### Asset Prices

| Command              | Args                                                                        | Return            | Errors                                                                                                         |
| -------------------- | --------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------- |
| `record_asset_price` | `asset_id: String, date: String, price: f64`                                | `()`              | `AssetNotFound` (MKT-043), `NotPositive` (MKT-021), `NonFinite` (MKT-021), `DateInFuture` (MKT-022), `Unknown` |
| `get_asset_prices`   | `asset_id: String`                                                          | `Vec<AssetPrice>` | `AssetNotFound` (MKT-072), `Unknown`                                                                           |
| `update_asset_price` | `asset_id: String, original_date: String, new_date: String, new_price: f64` | `()`              | `NotFound` (MKT-083), `NotPositive` (MKT-082), `NonFinite` (MKT-082), `DateInFuture` (MKT-082), `Unknown`      |
| `delete_asset_price` | `asset_id: String, date: String`                                            | `()`              | `NotFound` (MKT-090), `Unknown`                                                                                |

---

## Shared Types

```rust
// Transient value object — not persisted (WEB-020)
// Fields marked "optional" may be absent per spec rules
struct AssetLookupResult {
    name: String,
    reference: Option<String>,       // absent for keyword results with no ticker (WEB-046)
    currency: Option<String>,        // absent when OpenFIGI returns no currency (WEB-024)
    asset_class: Option<AssetClass>, // absent when securityType unrecognised (WEB-023)
    exchange: Option<String>,        // human-readable market name from exchCode; absent when OpenFIGI returns none (WEB-049)
}
```

```rust
// AssetClass variants (AST-003) — Derivatives added
enum AssetClass { Cash, Bonds, RealEstate, MutualFunds, ETF, Stocks, DigitalAsset, Derivatives }
// Derivatives maps from securityType: "Warrant" | "Option" | "Future" | "Rights" (WEB-023)
// default_risk for Derivatives = 5 (AST-003)
```

```rust
// Input prices are transmitted as f64 decimal; backend converts to i64 micros at the IPC boundary (MKT-024).
// ADR-001 (i64 micros) applies to storage and read responses — f64 on write input is the intentional
// transport-layer exception; the f64 → i64 conversion inside the command handler is the ADR-001 compliance point.

struct AssetPrice {
    asset_id: String,  // asset this price belongs to
    date: String,      // ISO 8601 calendar date (e.g. "2026-04-29")
    price: i64,        // market price in asset's native currency, i64 micros (ADR-001)
}
```

---

## Events

| Event               | Payload | Direction                                                                                                                                                                                                                                                         |
| ------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `AssetPriceUpdated` | none    | published — fired after successful `record_asset_price` (MKT-026), `update_asset_price` (MKT-085), `delete_asset_price` (MKT-091), or auto-record on `buy_holding`/`sell_holding`/`correct_transaction` when `record_price = true` and `unit_price > 0` (MKT-057) |

---

## Changelog

- 2026-04-26 — Added by `market-price` spec: `record_asset_price`
- 2026-04-26 — Typed errors: all commands now return domain-specific typed error enums instead of `String`
- 2026-04-26 — Added `CategoryNotFound` to `AssetCommandError` (raised when asset create/update references a nonexistent category)
- 2026-04-27 — Updated by `market-price` spec (MKT-050+): `AssetPriceUpdated` event now also fires from the auto-record path on `add_transaction` / `update_transaction`; no new commands
- 2026-04-29 — Added by `market-price` spec (MKT-070+): `get_asset_prices`, `update_asset_price`, `delete_asset_price`; `AssetPrice` shared type; error model extended with `UpdateAssetPriceCommandError`, `DeleteAssetPriceCommandError`
- 2026-05-03 — Merged from `asset_web_lookup-contract.md`: `lookup_asset`; added `AssetLookupResult` shared type
- 2026-05-03 — WEB-048/049: added `exchange` field to `AssetLookupResult`; added `Derivatives` AssetClass variant (AST-003); WEB-023 extended to map Warrant/Option/Future/Rights → Derivatives
- 2026-05-06 — CSH-016: added `CashAssetNotEditable` variant to `AssetCommandError`, `ArchiveAssetCommandError`, `DeleteAssetCommandError`; `archive_asset` and `delete_asset` now also surface `NotFound` (commands now load the asset to enforce the cash guard)
