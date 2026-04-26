# Contract — Asset

> Domain: `asset`
> Last updated by: `market-price` spec

> **Error model note:** All Tauri commands currently return `Result<T, String>` — errors are
> `anyhow::Error` converted via `.map_err(|e| e.to_string())`. The Errors column documents the
> semantic content of those strings. When the structured error type refactor lands (see todo:
> "Replace string-matching error assertions with structured error types"), error variants here
> should be replaced with typed enum variants.

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

| Event               | Payload | Direction                                           |
| ------------------- | ------- | --------------------------------------------------- |
| `AssetPriceUpdated` | none    | published — fired after successful upsert (MKT-026) |

---

## Changelog

- 2026-04-26 — Added by `market-price` spec: `record_asset_price`
