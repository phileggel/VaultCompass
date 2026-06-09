# Contract — Connection

> Domain: `connection` (bounded context — provider credential management)
> Last updated by: `api-key-management` spec

> **Error model on the wire**: each command's error serializes as a flat `{ code: "VariantName", ...payload }` object. The FE matches on `code`. Per-command reachable codes are listed in the "Errors" column below. Infrastructure failures (OS keychain / session-memory / plaintext-file I/O) surface as `{ code: "KeyStoreError" }` (no payload; diagnostic chain preserved server-side via `tracing::error!`). This is the keychain-world analog of the SQLite contexts' `DatabaseError` — `connection` is the project's first non-SQLite bounded context, so its infrastructure variant is named for its substrate.
>
> Rust-internal type organization (per-BC enums, serde tagging) is out of scope for this contract — it documents the BE↔FE frontier, not Rust internals.

---

## Commands

| Command                    | Args                                                     | Return                    | Errors                      |
| -------------------------- | -------------------------------------------------------- | ------------------------- | --------------------------- |
| `get_provider_connections` | —                                                        | `Vec<ProviderConnection>` | `KeyStoreError`             |
| `save_provider_key`        | `SaveProviderKeyArgs { provider, key, allow_plaintext }` | `ProviderConnection`      | `EmptyKey`, `KeyStoreError` |
| `test_provider_key`        | `TestProviderKeyArgs { provider, key }`                  | `ProviderKeyTestOutcome`  | `EmptyKey`                  |
| `remove_provider_key`      | `RemoveProviderKeyArgs { provider }`                     | `()`                      | `KeyStoreError`             |

**Command notes (traceability to spec rules):**

- `get_provider_connections` — KEY-016, KEY-031. Lists every supported provider with its `has_key` and `active_tier` so the Connections dialog can render rows (KEY-032) and the refresh gate can check whether a key exists (KEY-040). Never returns the secret value (KEY-018).
- `save_provider_key` — KEY-010, KEY-011, KEY-012. Persists the key via the tier ladder (keychain → session-memory), returning the resulting `ProviderConnection` so the FE learns the `active_tier`. `allow_plaintext = true` enables the tier-3 plaintext fallback (the explicit opt-in of KEY-012); when `false`, a keyring-less host lands in session-memory (tier 2). `EmptyKey` when the value is blank/whitespace (KEY-010).
- `test_provider_key` — KEY-020, KEY-021, KEY-022. Probes the provider with the supplied (not-necessarily-saved) value using a fixed well-known symbol; read-only wrt stored state (KEY-022). The three outcomes (`Accepted` / `Rejected` / `Unreachable`, KEY-023) are **successful returns**, not errors. `EmptyKey` when there is no value to test.
- `remove_provider_key` — KEY-013, KEY-034. Clears the key from **every** tier (keychain + session-memory + plaintext file), not just the active one. Idempotent: removing when no key exists succeeds.

---

## Shared Types

```rust
// Which external provider a connection authenticates. Extensible: Finnhub, OpenFigi
// arrive as further variants in later slices (KEY-031).
enum Provider {
    Stooq,
}

// Where a stored key currently lives, per the ADR-011 ladder (KEY-011, KEY-015).
enum StorageTier {
    OsKeychain,     // tier 1 — default, persists, OS-encrypted
    SessionMemory,  // tier 2 — fallback, RAM-only, cleared on exit (KEY-017)
    PlaintextFile,  // tier 3 — explicit opt-in only (KEY-012)
}

// Result of probing a provider with a candidate key (KEY-023).
enum ProviderKeyTestOutcome {
    Accepted,    // provider accepted the key
    Rejected,    // provider reachable but rejected the key
    Unreachable, // provider could not be contacted (network failure)
}

// Non-secret state of one provider's connection, surfaced to the UI.
struct ProviderConnection {
    provider: Provider,
    has_key: bool,                  // whether a key is stored (KEY-016); the value is never exposed (KEY-018)
    active_tier: Option<StorageTier>, // where the key lives (KEY-015); None when has_key is false
}

struct SaveProviderKeyArgs {
    provider: Provider,
    key: String,           // the pasted secret; write-only from the UI's perspective (KEY-018)
    allow_plaintext: bool, // tier-3 opt-in (KEY-012); false keeps the key off disk on a keyring-less host
}

struct TestProviderKeyArgs {
    provider: Provider,
    key: String,
}

struct RemoveProviderKeyArgs {
    provider: Provider,
}
```

---

## Events

_(none — the Connections dialog refreshes its own state after each command; no cross-view event is published. KEY-043/044 reuse the existing `asset_price_fetch` path and its `AssetPriceUpdated` / `AssetPriceFetchCompleted` events from the MKT contract surface; they add no command or event here.)_

---

## Changelog

- 2026-06-08 — Added by `api-key-management` spec: `get_provider_connections`, `save_provider_key`, `test_provider_key`, `remove_provider_key` (new `connection` bounded context).
