# Contract — asset_web_lookup

> Domain: asset_web_lookup
> Last updated by: asset-web-lookup

## Commands

| Command            | Args            | Return                   | Errors         |
| ------------------ | --------------- | ------------------------ | -------------- |
| `lookup_asset` | `query: String` | `Vec<AssetLookupResult>` | `NetworkError` |

## Shared Types

```rust
// Transient value object — not persisted (WEB-020)
// Fields marked "optional" may be absent per spec rules
struct AssetLookupResult {
    name: String,
    reference: Option<String>,       // absent for keyword results with no ticker (WEB-046)
    currency: Option<String>,        // absent when OpenFIGI returns no currency (WEB-024)
    asset_class: Option<AssetClass>, // absent when securityType unrecognised (WEB-023)
}
```

## Events

None — this use case produces no new events. The downstream `add_asset` command (WEB-045) publishes `AssetUpdated` per the existing asset contract.

## Changelog

- 2026-05-01 — Added by `asset-web-lookup` spec: `lookup_asset`
