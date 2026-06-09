# Implementation Plan — API Key Management (KEY)

> Spec: `docs/spec/api-key-management.md` · Contract: `docs/contracts/connection-contract.md` (domain `connection`)
> New bounded context: `connection`. No database migration (keys live in OS keychain / session memory / opt-in plaintext file, never SQLite).
> Constraining ADRs: **ADR-011** (BYOK key storage ladder) · **ADR-015** (all providers BYOK-keyed, no key-less default) · ADR-004 (use cases inject services).

---

## 1. Workflow TaskList

### Setup

- [ ] 📖 Read spec: `docs/spec/api-key-management.md`
- [ ] 📖 Read contract: `docs/contracts/connection-contract.md`
- [ ] 📖 Read constraining ADRs: `docs/adr/011-byok-api-keys-os-keychain.md`, `docs/adr/015-byok-keyed-price-providers.md`, `docs/adr/004-use-cases-inject-services-not-repositories.md`
- [ ] 📖 Read conventions: `ARCHITECTURE.md`, `docs/backend-rules.md` (esp. B0/B37–B43 gold layout), `docs/ddd-reference.md`, `docs/error-model.md`, `docs/frontend-rules.md` (F0/F26–F28), `docs/i18n-rules.md`, `docs/frontend-visual-proof.md`, `docs/test_convention.md`

### Backend phase — **PR #1**

- [ ] 🗄️ No migration (keys are not persisted to SQLite)
- [ ] 📦 Add `keyring` crate to `src-tauri/Cargo.toml` (OS keychain backend; default features cover macOS/Windows/Linux Secret Service + portal per ADR-011)
- [ ] ✍️ Backend test stubs (`test-writer-backend` from `connection-contract.md`; + KEY-043/044 integration stubs in `asset_price_fetch` tests — confirm red)
- [ ] 🏗️ Backend Implementation (minimal — implement only what makes the failing tests pass; no defensive code, no anticipation of future rules; green confirmed)
- [ ] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` + **`reviewer-security`** [key storage / new commands / secret handling] + **`reviewer-infra`** [`keyring` dependency add in `Cargo.toml`] in parallel → `/review-triage` → apply Follow-ups; halt on (b)/(c)) — _no `reviewer-sql` (no migration)_
- [ ] 🔗 Type Synchronization (`just generate-types` → `src/bindings.ts`)
- [ ] 🔧 `npx tsc --noEmit` → fix TS errors from new bindings only (no UI work)
- [ ] 🧹 `just format`
- [ ] 💾 Commit: `feat(connection): provider API-key storage + Stooq keyed fetch` via `/smart-commit` [HARD GATE]
- [ ] 🔀 `/create-pr` (PR #1 — BE). After merge, branch FE off updated `main`.

### Frontend phase — **PR #2**

- [ ] ✍️ Frontend test stubs (`test-writer-frontend` from contract; pass `modified_functions` list — see §Rules Coverage; confirm red)
- [ ] 💻 Frontend Implementation (minimal — implement only what makes the failing tests pass; no defensive code, no anticipation of future rules; green confirmed)
- [ ] 📸 Visual proof (`/visual-proof` — Connections dialog: no-key / key-entered / testing / test-success / test-invalid / test-unreachable / key-set+tier / remove-confirm; light + dark)
- [ ] 🔍 Frontend Review (`reviewer-frontend` → `/review-triage` → apply Follow-ups; halt on (b)/(c))
- [ ] 🧹 `just format`
- [ ] 💾 Commit: `feat(connection): Connections dialog + price-refresh key gating` via `/smart-commit` [HARD GATE]
- [ ] 🔀 `/create-pr` (PR #2 — FE). After merge, branch E2E off updated `main`.

### Closure — **PR #3**

- [ ] ✍️ E2E scenarios (`test-writer-e2e`; `/setup-e2e` first if needed) — Connections save/remove + refresh-gate-opens-dialog. **Exclusions:** the live Stooq probe (KEY-021) stays in BE tests (no external network in E2E); keychain is unavailable headless → flows land in tier-2 session memory.
- [ ] ▶️ Run E2E suite (`just test-e2e-headless` → green; main agent triages failures)
- [ ] 🔍 Cross-cutting Review (`reviewer-e2e` [E2E files] + **`reviewer-security`** [final pass on the key surface + provider rewire] in parallel → `/review-triage`) — _`reviewer-infra` only if `Cargo.toml`/CI/capabilities changed_
- [ ] 📚 Documentation Update — `docs/todo.md` (close the 🔴 KEY entry), `docs/lessons.md` (L-006 resolution pointer to ADR-015/KEY), `ARCHITECTURE.md` (register the new `connection` BC + the `connection` event-less command surface; note first non-SQLite context), update memory `project_price_fetch_blocked_on_key`
- [ ] ✅ Spec check (`spec-checker`) [HARD GATE — every KEY-NNN + the 4 contract commands covered]
- [ ] 🧹 `just format`
- [ ] 💾 Commit: `test(connection): E2E + closure for API key management` via `/smart-commit` [HARD GATE]
- [ ] 🔀 `/create-pr` (PR #3 — E2E + closure)

---

## 2. Detailed Implementation Plan

### Migrations

None — no schema change.

### Backend (new `connection` BC, mirrors `context/currency/` gold layout)

`src-tauri/src/context/connection/`

- **`mod.rs`** — `pub mod api/application/domain/error/infrastructure` + glob re-export `api::*` (so `collect_commands!` resolves `connection::<cmd>`) + named re-exports of wire types.
- **`error.rs`** — `ConnectionError` enum (`#[derive(thiserror::Error, serde::Serialize, specta::Type, Clone)]`, `#[serde(tag="code")]`): variants `EmptyKey`, `KeyStoreError` (opaque on wire; full chain via `tracing::error!` per error-model). Mirror currency's `each_variant_emits_a_code` test. **KEY-014**: no variant or log carries the secret.
- **`domain/`**
  - `provider.rs` — `Provider` enum (`Stooq`), `StorageTier` enum (`OsKeychain | SessionMemory | PlaintextFile`), `ProviderKeyTestOutcome` enum (`Accepted | Rejected | Unreachable`), `ProviderConnection { provider, has_key, active_tier: Option<StorageTier> }`. All `specta::Type`.
  - `key_store.rs` — `KeyStore` trait (port, `#[cfg_attr(test, mockall::automock)]`): `clear(provider)` (KEY-013, all tiers), `store(provider, &key, allow_plaintext) -> StorageTier` (KEY-010/011/012), `locate(provider) -> Option<StorageTier>` (KEY-016, no value), `read(provider) -> Option<String>` (backend-internal, KEY-018 — never reaches the wire).
  - `probe.rs` — `ConnectionProbe` trait (port, mockable): `probe(provider, &key) -> ProviderKeyTestOutcome` (KEY-021/022, read-only).
  - `mod.rs` — re-exports.
- **`application/`**
  - `service.rs` — `ConnectionService` (injects `Box<dyn KeyStore>` + `Box<dyn ConnectionProbe>`): `connections() -> Result<Vec<ProviderConnection>, ConnectionError>` (KEY-016); `save_key(provider, key, allow_plaintext) -> Result<ProviderConnection,_>` (KEY-010: reject blank → `EmptyKey`; clear-all-tiers-then-store per KEY-013 overwrite); `test_key(provider, key) -> Result<ProviderKeyTestOutcome,_>` (KEY-021/022, `EmptyKey` on blank); `remove_key(provider) -> Result<(),_>` (KEY-013); **`resolve_key(provider) -> Result<Option<String>,_>`** (backend-internal, consumed only by the fetch use case — never a command).
  - `mod.rs` — `pub use ConnectionService`.
- **`api.rs`** — 4 `#[tauri::command] #[specta::specta]` handlers delegating to `ConnectionService`, returning typed `Result<_, ConnectionError>`: `get_provider_connections`, `save_provider_key(SaveProviderKeyArgs)`, `test_provider_key(TestProviderKeyArgs)`, `remove_provider_key(RemoveProviderKeyArgs)`. Args structs (`specta::Type`) live here or in `domain/provider.rs`.
- **`infrastructure/`**
  - `keyring_store.rs` — `LayeredKeyStore` implementing `KeyStore`: **tier 1** `keyring::Entry` (service = app id, account = provider name); **tier 2** a process-global `Mutex<HashMap<Provider,String>>` session store (KEY-017, cleared on exit by virtue of being in-process); **tier 3** a plaintext file under the app data dir, written only when `allow_plaintext` (KEY-012). `store` follows the ladder + KEY-011 floor semantics; `clear` wipes all three (KEY-013); `locate` probes tiers in order; write failure on the selected tier → `KeyStoreError` (KEY-011), never silent loss.
  - `stooq_probe.rs` — `StooqProbe` implementing `ConnectionProbe`: builds the keyed `q/d/l/` URL for a **fixed well-known symbol** (KEY-021, `spy.us`) and runs it through the shared `StooqGate` (PoW + cookie), so the probe clears the same browser-verification gate the fetch does. A daily-CSV body → `Accepted`; a non-CSV body (challenge page / rejection) → `Rejected`; a transport error → `Unreachable`. Uses `shared/infrastructure/stooq.rs`, not the `asset` BC's client (BC isolation).
  - `mod.rs` — re-exports.

**Provider-seam rewire (existing `asset` BC + `asset_price_fetch` use case) — KEY-043/044:**

- `src-tauri/src/context/asset/domain/asset_price.rs` — extend `PriceProvider::fetch_price` to accept the api key: `fetch_price(&self, symbol: &str, api_key: &str) -> anyhow::Result<Option<Quote>>`. Regenerate `MockPriceProvider` expectations in dependent tests.
- `src-tauri/src/context/asset/repository/stooq_client.rs` — switch the URL to the keyed `q/d/l/?s=SYM&i=d&apikey=KEY` daily-download endpoint (the light `q/l/` endpoint 404s, even with a key). **The proof-of-work machinery is RETAINED, not deleted** — a live probe (2026-06-08) proved the apikey does not bypass Stooq's PoW browser-verification gate, so the path must solve the challenge AND present the key (ADR-015). To avoid duplicating the PoW solver in the `connection` probe, extract the PoW + cookie + retry flow into `shared/infrastructure/stooq.rs` (a `StooqGate`); `stooq_client.rs` keeps only its URL build + `parse_quote`. The download returns the full daily history, so `parse_quote` switches to the daily format (`Date,Open,High,Low,Close,Volume`) and takes the **last** row (latest close, col 4; date col 0), read under a raised body cap (the shared 64 KB cap is too small for a full history). Observation-date handling (MKT-117/118) unchanged. _(Date-range trimming via `&d1=` to fetch only recent rows is a deferred optimization — it returned "No data" under a rate-limited probe key, unconfirmed.)_
- `src-tauri/src/shared/infrastructure/stooq.rs` (NEW) — `StooqGate`: shared Stooq HTTP plumbing (cookie store + proof-of-work solve/verify/retry per L-005) returning the response body. Consumed by both `stooq_client.rs` (asset fetch) and `stooq_probe.rs` (connection key test), so the PoW solver lives in one place and neither BC imports the other. `shared/infrastructure/http.rs` gains a `read_capped_text_with_limit` variant for the larger Stooq history payload.
- `src-tauri/src/use_cases/asset_price_fetch/orchestrator.rs` — inject `Arc<ConnectionService>` (ADR-004). At task start, after the scope is built, `resolve_key(Provider::Stooq)` and pass the resulting `Option<String>` into `dispatcher.spawn(scope, fx_pairs, lease, stooq_key)`. The orchestrator does **not** publish events (it owns no `event_bus`); the no-key decision is handed to the dispatcher so the existing event owner emits the completion signal.
- `src-tauri/src/use_cases/asset_price_fetch/dispatcher.rs` — `spawn` gains a `stooq_key: Option<String>` parameter. **`None` → KEY-044**: skip the whole scope without any per-asset provider call and publish `AssetPriceFetchCompleted { ok: 0, skipped: <scope_len> }` (MKT-119) — the dispatcher already owns the event bus, so the all-skipped completion is emitted here, not in the orchestrator. **`Some(key)`**: thread it into the per-asset `provider.fetch_price(&symbol, &key)` loop as today.

**DI / registry:**

- `src-tauri/src/lib.rs` — construct `LayeredKeyStore` + `StooqProbe` → `ConnectionService` (`Arc`); `app_handle.manage(connection_service.clone())`; inject the same `Arc<ConnectionService>` into `AssetPriceFetchUseCase`. (The `ReqwestStooqClient` provider stays as-is; the key now flows as a fetch parameter, so no key capture at construction — supports runtime key changes.)
- `src-tauri/src/core/specta_builder.rs` — add `// ----- connection BC -----` banner; `.typ::<connection::Provider>()`, `StorageTier`, `ProviderKeyTestOutcome`, `ProviderConnection`, `SaveProviderKeyArgs`, `TestProviderKeyArgs`, `RemoveProviderKeyArgs`, `ConnectionError`; add the 4 commands to `collect_commands!`.

### Frontend (new `features/connections/` + URL-driven modal mount)

`src/features/connections/`

- `gateway.ts` (+ `gateway.test.ts`) — only file calling `commands.*`: `getProviderConnections`, `saveProviderKey(provider, key, allowPlaintext)`, `testProviderKey(provider, key)`, `removeProviderKey(provider)`. Positional args matching `bindings.ts` exactly. Typed `Result` pass-through (F27).
- `index.ts`.
- `ConnectionsModal.tsx` — the dialog (FormModal chrome); lists provider rows (KEY-030/031); accepts an optional `focusProvider` (KEY-040 focuses Stooq).
- `useConnections.ts` — loads connections, exposes save/test/remove with per-row in-flight state (KEY-035).
- `provider_row/ProviderRow.tsx` (+ `useProviderRow.ts`) — KEY-032 (name, status, tier label, key field, Test, Remove, signup link), KEY-015/016 status+tier, KEY-020 test-enabled-only-on-non-empty, KEY-023 three test outcomes, KEY-033 save feedback, KEY-034 remove-confirm + hidden-when-no-key, KEY-012 plaintext opt-in confirm (only when tier-1 unavailable). Key input via `TextField` (password-style/masked).
- `shared/presenter.ts` (+ test) — `ConnectionError.code → i18n key` (F27 pipeline); `ProviderKeyTestOutcome → UI state`; `StorageTier → label`.

**Shell wiring (URL-driven modal, mirrors `AssetEditModalMount`):**

- `src/features/shell/navItems.ts` — add a "Connections" entry (`nav.connection`) that opens `?modal=connections` (KEY-030).
- `src/features/shell/ConnectionsModalMount.tsx` (new) — watches the URL `?modal=connections[&provider=…]`, renders `ConnectionsModal`. Mounted in the shell (no sibling-feature import).

**Gating + launch skip (modify existing):**

- `src/features/accounts/refresh_prices/useRefreshGlobalPrices.ts` — **KEY-040**: before dispatch, check Stooq `has_key` (via `connectionGateway.getProviderConnections()`); if absent, navigate `?modal=connections&provider=stooq` instead of dispatching. `[unit-test-needed]`
- `src/features/account_details/refresh_prices/useRefreshAccountPrices.ts` — same gate. `[unit-test-needed]`
- `src/App.tsx` — **KEY-041**: extract the launch-fetch gate into a testable helper (`shouldLaunchFetch(connections)`); skip `fetchAllAssetPrices()` silently when no key. `[unit-test-needed]`
- **KEY-042** — verify manual "Enter price" / price-history remain ungated (no change; covered by E2E + a guard test).

**i18n:**

- `src/i18n/locales/en/common.json` + `fr/common.json` — add `connection.*` block (statuses, tier labels, test outcomes, save/remove confirmations, signup link label, plaintext-warning copy) and `nav.connection`. i18n-aware a11y labels (F24).

### Rules Coverage

| Rule    | Layer              | Task                                                                                          | Notes                                             |
| ------- | ------------------ | --------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| KEY-010 | backend            | `ConnectionService::save_key` + `LayeredKeyStore::store`/`clear`                              | `EmptyKey`; clear-then-store overwrite (KEY-013)  |
| KEY-011 | backend            | `LayeredKeyStore::store` (tier ladder + session floor + write-error→`KeyStoreError`)          | ADR-011                                           |
| KEY-012 | frontend + backend | `LayeredKeyStore` tier-3 gate + `ProviderRow` opt-in confirm                                  | only when tier-1 unavailable                      |
| KEY-013 | frontend + backend | `LayeredKeyStore::clear` (all tiers) + `ProviderRow` remove                                   |                                                   |
| KEY-014 | backend            | `ConnectionError` + tracing audit (no secret in logs)                                         | reviewer-security check                           |
| KEY-015 | frontend           | `ProviderRow` tier label                                                                      |                                                   |
| KEY-016 | frontend + backend | `ConnectionService::connections` / `LayeredKeyStore::locate`                                  | read fault → `KeyStoreError`, not `has_key=false` |
| KEY-017 | backend            | tier-2 in-process session store                                                               |                                                   |
| KEY-018 | backend            | `KeyStore::read` is internal; no command returns the value                                    | reviewer-security check                           |
| KEY-020 | frontend           | `ProviderRow` Test action (enabled iff non-empty)                                             |                                                   |
| KEY-021 | backend            | `StooqProbe::probe` (fixed symbol)                                                            | ADR-015 keyed endpoint                            |
| KEY-022 | backend            | `ConnectionService::test_key` (read-only)                                                     |                                                   |
| KEY-023 | frontend           | `presenter.ts` outcome → 3 UI states                                                          |                                                   |
| KEY-030 | frontend           | `navItems.ts` + `ConnectionsModalMount`                                                       |                                                   |
| KEY-031 | frontend           | `ConnectionsModal` provider list                                                              |                                                   |
| KEY-032 | frontend           | `ProviderRow` contents                                                                        |                                                   |
| KEY-033 | frontend           | `ProviderRow` save feedback + snackbar                                                        |                                                   |
| KEY-034 | frontend           | `ProviderRow` remove confirm + hidden-when-no-key                                             |                                                   |
| KEY-035 | frontend           | `useProviderRow` in-flight state                                                              |                                                   |
| KEY-040 | frontend           | `useRefreshGlobalPrices` + `useRefreshAccountPrices` gate                                     | `[unit-test-needed]`                              |
| KEY-041 | frontend           | `App.tsx` launch-gate helper                                                                  | `[unit-test-needed]`                              |
| KEY-042 | frontend           | manual-entry-ungated guard                                                                    | E2E + guard test                                  |
| KEY-043 | backend            | `stooq_client.rs` keyed fetch + `PriceProvider` sig                                           | ADR-015; integration test in `asset_price_fetch`  |
| KEY-044 | backend            | `dispatcher.rs::spawn` no-key short-circuit (orchestrator resolves key, dispatcher publishes) | integration test; `ok=0`, all skipped (MKT-119)   |

**`modified_functions` for `test-writer-frontend`:** `[useRefreshGlobalPrices.ts:useRefreshGlobalPrices, useRefreshAccountPrices.ts:useRefreshAccountPrices, App.tsx:shouldLaunchFetch]`
**Backend modified-behavior tests for `test-writer-backend`:** KEY-043/044 in `src-tauri/tests/asset_price_fetch_crud.rs` (no-key short-circuit reports all-skipped; keyed fetch passes the key through — assert via `MockPriceProvider`/mocked `ConnectionService`).

---

## 3. PR Plan

- **Strategy**: `3 PRs`
- **Estimate**: BE ~15 files / ~600+ LOC (over the per-layer split threshold) · FE ~14 files / ~350 LOC · E2E light (keychain unavailable headless → tier-2 session memory; live probe excluded from E2E).

**PR #1 — `feat(connection): provider API-key storage + Stooq keyed fetch`**

- Scope: `Cargo.toml` (keyring) + `context/connection/` (full trio) + provider-seam rewire (`asset/domain/asset_price.rs`, `asset/repository/stooq_client.rs`, `use_cases/asset_price_fetch/{orchestrator,dispatcher}.rs`) + `lib.rs` DI + `specta_builder.rs` + `just generate-types` + backend tests. Terminates at the **Backend phase** `/create-pr`.
- Dependency: none (branch off `main`). Mergeable alone — bindings present, FE not yet consuming.
- Branch: ships from the current `feat/api-key-management` branch (serves as the BE branch; no rename needed)

**PR #2 — `feat(connection): Connections dialog + price-refresh key gating`**

- Scope: `features/connections/` + `shell/{navItems,ConnectionsModalMount}` + the 2 refresh-gate hooks + `App.tsx` launch skip + i18n + FE tests + `/visual-proof`. Terminates at the **Frontend phase** `/create-pr`.
- Dependency: rebase off `main` after PR #1 merges (consumes new bindings).
- Branch: `feat/api-key-management-fe`

**PR #3 — `test(connection): E2E + closure for API key management`**

- Scope: E2E scenarios + cross-cutting review + docs (`todo.md`, `lessons.md`, `ARCHITECTURE.md`, memory) + `spec-checker`. Terminates at the **Closure** `/create-pr`.
- Dependency: rebase off `main` after PR #2 merges.
- Branch: `feat/api-key-management-e2e`
