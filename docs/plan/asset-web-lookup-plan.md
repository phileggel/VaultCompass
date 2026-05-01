# Implementation Plan — Asset Web Lookup (WEB)

> Spec: `docs/spec/asset-web-lookup.md` · Contract: `docs/contracts/asset_web_lookup-contract.md`
> Trigram: `WEB` (registered in `docs/spec-index.md`, status `active`)
> Profile: Tauri 2 (full pipeline including `just generate-types` + E2E)

---

## 1. Workflow TaskList

- [ ] Review architecture and rules: `ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/frontend-rules.md`, `docs/e2e-rules.md`, `docs/ubiquitous-language.md`
- [ ] Database migration — **N/A** (no schema change; `AssetLookupResult` is a transient value object per WEB-020 / contract)
- [ ] Backend test stubs (`test-writer-backend` — all stubs written, red confirmed)
- [ ] Backend implementation (minimal — make failing tests pass, green confirmed; **implement only what is required to make the failing tests pass — no additional methods, no defensive code, no anticipation of future rules**)
- [ ] `just format` (rustfmt + clippy --fix)
- [ ] Backend review (`reviewer-backend` then fix issues)
- [ ] Type synchronization: `just generate-types` (Tauri profile)
- [ ] Compilation fixup — TypeScript errors from the new bindings only (no UI work yet)
- [ ] `just check` — TypeScript clean
- [ ] Commit (backend layer): `feat(asset-web-lookup): add search_asset_web command and OpenFIGI client`
- [ ] Frontend test stubs (`test-writer-frontend` — all stubs written, red confirmed)
- [ ] Frontend implementation (minimal — make failing tests pass, green confirmed; **implement only what is required to make the failing tests pass — no additional methods, no defensive code, no anticipation of future rules**)
- [ ] `just format`
- [ ] Frontend review (`reviewer-frontend` then fix issues)
- [ ] Commit (frontend layer): `feat(asset-web-lookup): add web lookup dialog before Add Asset form`
- [ ] E2E tests (`test-writer-e2e`; `/setup-e2e` first if not initialized; tests written, red confirmed; iterate selectors per `docs/e2e-rules.md` until green)
- [ ] Commit (E2E): `test(asset-web-lookup): add E2E coverage for web lookup flow`
- [ ] Cross-cutting review (`reviewer-arch` always; `reviewer-sql` skipped — no migrations; `reviewer-infra` if any config / script / hook / workflow file changed — likely no, unless `Cargo.toml` is touched to add an HTTP client)
- [ ] i18n review (`i18n-checker` — UI text added)
- [ ] Documentation update: `ARCHITECTURE.md` (new use case `asset_web_lookup`), `docs/todo.md` if new tech debt or resolved items (entries in English)
- [ ] Spec check (`spec-checker`)
- [ ] Commit (tests & docs): `docs(asset-web-lookup): record use case and update architecture`

---

## 2. Detailed Implementation Plan

### 2.1 Architectural placement

- **Backend**: new use case folder `src-tauri/src/use_cases/asset_web_lookup/` (mirrors `update_checker/` — also an outbound HTTP use case with no DB dependency).
  - Justification: this is **not** a mutation of the `Asset` aggregate; it is a transient lookup that happens to feed `add_asset`. It does **not** belong to `context/asset/` because that would force a non-domain HTTP dependency into a bounded context (B0a, B0b).
  - The use case has **no service / no repository** because there is no aggregate to load and no DB write. The orchestrator directly drives the OpenFIGI HTTP client (analogous to `update_checker::service`).
  - No event published (B12 — orchestrators do not publish; and there is no state change). The downstream `add_asset` already publishes `AssetUpdated`.
- **Frontend**: extension of the existing asset-creation flow under `src/features/assets/`. The web lookup is the new entry point that wraps `AddAssetModal`. New sub-feature directory `web_lookup/` colocated next to `add_asset/`. No new top-level feature module — per F1/F2, the web lookup is a sub-feature concern of the assets feature (the lookup result feeds the Add Asset form, no other consumer).

### 2.2 Migrations

**None.** `AssetLookupResult` is a transient value object (contract §Shared Types). No persisted state introduced. `just prepare-sqlx` is **not** required.

### 2.3 Backend tasks (`src-tauri/src/use_cases/asset_web_lookup/`)

#### File: `src-tauri/src/use_cases/asset_web_lookup/mod.rs`

- Re-export public surface: `pub use api::*;` and `pub use orchestrator::{AssetLookupResult, AssetWebLookupUseCase};` and `pub use api::WebLookupCommandError;` (B11).

#### File: `src-tauri/src/use_cases/asset_web_lookup/orchestrator.rs`

- **Value object** `AssetLookupResult` (B1 exception — no identity, no factory; struct-literal constructed inside the orchestrator):
  - `name: String`
  - `reference: Option<String>`
  - `currency: Option<String>`
  - `asset_class: Option<crate::context::asset::AssetClass>` (imported through context `mod.rs` per B6 — never `crate::context::asset::domain::...`)
  - Derives `Debug`, `Clone`, `serde::Serialize`, `specta::Type`.
- **Trait** `OpenFigiClient` (declared in this file; allows mocking in tests per B26):
  - `async fn map_isin(&self, isin: &str) -> anyhow::Result<Vec<RawFigiHit>>`
  - `async fn search_keyword(&self, query: &str) -> anyhow::Result<Vec<RawFigiHit>>`
  - `RawFigiHit` is a private struct holding the raw OpenFIGI fields needed to project an `AssetLookupResult` (`name`, `ticker`, `security_type`, `currency`).
- **Concrete implementation** `ReqwestOpenFigiClient` in the same file:
  - Holds a `reqwest::Client`.
  - Endpoints (constants in the file):
    - `MAP_URL = "https://api.openfigi.com/v3/mapping"` (POST, body `[{"idType":"ID_ISIN","idValue":<isin>}]`).
    - `SEARCH_URL = "https://api.openfigi.com/v3/search"` (POST, body `{"query":<query>}`).
  - WEB-021 — no API key header.
  - Returns `anyhow::Err` on network failure, timeout, or any non-2xx response (covers WEB-025 including rate-limit responses).
- **Orchestrator** `AssetWebLookupUseCase` (B14):
  - Holds `Arc<dyn OpenFigiClient + Send + Sync>` (B19 — no infrastructure types in public signature; trait only).
  - `pub fn new(client: Arc<dyn OpenFigiClient + Send + Sync>) -> Self`.
  - `pub async fn search(&self, query: String) -> anyhow::Result<Vec<AssetLookupResult>>`:
    1. Trim the query.
    2. Routing (WEB-014): `if query.len() == 12 && query.chars().all(|c| c.is_ascii_alphanumeric())` → call `map_isin`; else → call `search_keyword`.
    3. Map raw hits to `AssetLookupResult`:
       - `name` ← raw `name`.
       - `reference` (WEB-046): on the ISIN path the trimmed input ISIN; on the keyword path `Some(ticker)` if `ticker` is non-empty, else `None`.
       - `currency` (WEB-024): pass through if present, else `None`.
       - `asset_class` (WEB-023): translated by a private free function `map_security_type(s: &str) -> Option<AssetClass>` in this file:
         - `"Common Stock"` → `AssetClass::Stocks`
         - `"ETF"` → `AssetClass::ETF`
         - `"Mutual Fund"` → `AssetClass::MutualFunds`
         - `"Corporate Bond"` | `"Government Bond"` → `AssetClass::Bonds`
         - `"Cryptocurrency"` | `"Digital Currency"` → `AssetClass::DigitalAsset`
         - `"REIT"` | `"Real Estate Investment Trust"` → `AssetClass::RealEstate`
         - `"Cash"` → `AssetClass::Cash`
         - any other value → `None`
    4. Truncate to first 10 (WEB-022).
- **Inline tests** (`#[cfg(test)] mod tests`, B26 with mockall-like hand-rolled mock of `OpenFigiClient`; B25 — non-trivial only):
  - `routes_12_alphanumeric_query_to_map_isin` (WEB-014)
  - `routes_short_query_to_search_keyword` (WEB-014)
  - `routes_query_with_dash_to_search_keyword` (WEB-014 — non-alphanumeric)
  - `routes_13_char_alphanumeric_to_search_keyword` (WEB-014 — wrong length)
  - `truncates_results_to_ten` (WEB-022)
  - `maps_security_type_common_stock_to_stocks` (WEB-023)
  - `maps_security_type_etf_to_etf` (WEB-023)
  - `maps_security_type_corporate_bond_to_bonds` (WEB-023)
  - `maps_security_type_unknown_results_in_none_asset_class` (WEB-023)
  - `passes_currency_through_when_present` (WEB-024)
  - `currency_absent_when_openfigi_omits_it` (WEB-024)
  - `reference_is_input_isin_on_isin_path` (WEB-046)
  - `reference_is_ticker_on_keyword_path_when_present` (WEB-046)
  - `reference_absent_on_keyword_path_when_no_ticker` (WEB-046)
  - `propagates_client_error_as_anyhow` (WEB-025 — orchestrator surfaces failures; api.rs maps them to `NetworkError`)

#### File: `src-tauri/src/use_cases/asset_web_lookup/api.rs`

- `#![allow(clippy::unreachable)]` (Tauri/specta macros).
- **Typed error** `WebLookupCommandError` (B23 exception for Tauri responses):

  ```rust
  #[derive(Debug, Serialize, Type, thiserror::Error)]
  #[serde(tag = "code")]
  pub enum WebLookupCommandError {
      #[error("Network error while contacting the lookup service")]
      NetworkError,
  }
  ```

  - Single variant, per WEB-025 / contract.

- **Tauri command** `search_asset_web` (B8/B9/B13):

  ```rust
  #[tauri::command]
  #[specta::specta]
  pub async fn search_asset_web(
      uc: State<'_, AssetWebLookupUseCase>,
      query: String,
  ) -> Result<Vec<AssetLookupResult>, WebLookupCommandError>
  ```

  - Body: `uc.search(query).await.map_err(|e| { tracing::warn!(target: BACKEND, error = %e, "search_asset_web failed (WEB-025)"); WebLookupCommandError::NetworkError })`.
  - Logging via `tracing` with `target: BACKEND` (B16/B17/B18).

- No inline tests in `api.rs` (thin adapter — covered by orchestrator tests + E2E).

#### File: `src-tauri/src/use_cases/mod.rs`

- Add module declaration:
  ```rust
  /// Asset Web Lookup: OpenFIGI search to pre-fill the Add Asset form (WEB).
  pub mod asset_web_lookup;
  ```

#### File: `src-tauri/src/core/specta_builder.rs`

- Add to `use` line: `asset_web_lookup`.
- Register types:
  ```rust
  .typ::<asset_web_lookup::AssetLookupResult>()
  .typ::<asset_web_lookup::WebLookupCommandError>()
  ```
- Register command in `collect_commands![...]`: `asset_web_lookup::search_asset_web`.

#### File: `src-tauri/src/lib.rs`

- Add use: `use crate::use_cases::asset_web_lookup::{AssetWebLookupUseCase, ReqwestOpenFigiClient};`
- In the async setup block, after the existing use-case wiring:

  ```rust
  let openfigi_client = Arc::new(ReqwestOpenFigiClient::new());
  let asset_web_lookup_uc = AssetWebLookupUseCase::new(openfigi_client);
  app_handle.manage(asset_web_lookup_uc);
  ```

  - Manage is independent of `AppState` (no DB needed — same pattern as `UpdateState`).

#### File: `src-tauri/Cargo.toml`

- Add dependency `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }` (rustls to stay consistent with the sqlx feature flag already in use).
  - Triggers an `infra-reviewer` pass since the manifest changes.

#### Backend rules coverage check

- B0/B0a/B0b: code lives under `use_cases/`, no business logic in `core/` or `repository/`.
- B5/B6: only imports `crate::context::asset::AssetClass` through the public `mod.rs`.
- B8/B9/B13: api.rs is thin, single command, command also registered in `core/specta_builder.rs`.
- B10/B11: use case can import contexts; module re-exports through `mod.rs`.
- B12: no event emission.
- B16/B17/B18: `tracing` with `target: BACKEND`.
- B19: orchestrator depends on the `OpenFigiClient` trait, not on `reqwest::Client` directly.
- B23: `anyhow::Result` internally, `Result<T, WebLookupCommandError>` at the Tauri boundary.
- B25/B26: tests mock the `OpenFigiClient` trait; each test exercises real logic.

### 2.4 Frontend tasks (`src/features/assets/`)

The web lookup is a sub-feature of the assets feature (per F1/F2). It sits **before** `AddAssetModal` in the asset-creation flow.

#### File: `src/features/assets/gateway.ts` (extend existing)

- Import the new binding: `searchAssetWeb`, `WebLookupCommandError`, and the generated `AssetLookupResult`. Keep all `commands.*` calls in this file (F3).
- Add method:
  ```ts
  async searchAssetWeb(
    query: string,
  ): Promise<Result<AssetLookupResult[], WebLookupCommandError>> {
    return await commands.searchAssetWeb(query);
  }
  ```
- Verify positional argument count/order matches `bindings.ts` exactly (`commands.searchAssetWeb(query)` — single positional `query`, never wrapped).

#### Folder: `src/features/assets/web_lookup/`

##### File: `WebLookupModal.tsx`

- Smart container component (F6/F8).
- Props:
  - `isOpen: boolean`
  - `onClose: () => void`
  - `onSuccess?: (assetId: string) => void` — propagated to `AddAssetModal` after a successful save
- Internal state machine (single `step: "search" | "form-prefilled" | "form-manual"`):
  - `search` (WEB-010): renders `<SearchPanel />` (input, results list, loading/empty/error/idle states, "Fill manually" bypass).
  - `form-prefilled` (WEB-040, WEB-047): renders the existing `<AddAssetModal />` with new optional `prefill` prop **and** a back button surfaced in the modal header (or rendered in the `actions` slot — see refactor note below). Selecting back returns to `step: "search"` with the previous query and results retained in memory (state lifted to this component, **not** the search hook, so the unmount of `SearchPanel` does not lose the cache — WEB-047).
  - `form-manual` (WEB-013): renders `<AddAssetModal />` with no `prefill`. No back action surfaced (the "Fill manually" path is one-way; user closes the dialog to abandon).
- `useEffect` mount log: `logger.info("[WebLookupModal] mounted")` (F13).
- All visible text via `useTranslation` (F16).
- E2E hooks: `WebLookupModal` itself does not own a form `id`; the inner `SearchPanel` carries `id="web-lookup-search-form"` (E1). Buttons: `aria-label={t("asset.web_lookup.action_back")}` for back (E4), `aria-label={t("asset.web_lookup.action_fill_manually")}` for the bypass (E4).

##### File: `SearchPanel.tsx`

- Dumb-ish component driven by `useWebLookupSearch` hook.
- Renders:
  - `<form id="web-lookup-search-form">` (E1) with a single `TextField` (`id="web-lookup-search-query"` — E2) and a submit `Button` (`type="submit"`, `form="web-lookup-search-form"` — E3) labelled `t("asset.web_lookup.action_search")`.
  - Submit disabled when `query.trim().length === 0` (WEB-011) or while `isLoading` (WEB-030).
  - Below the form, depending on hook state:
    - `idle` — placeholder text `t("asset.web_lookup.idle_hint")`.
    - `loading` — spinner / shimmer + `aria-busy="true"` (WEB-030).
    - `results` — list of selectable rows (one per `AssetLookupResult`), each row showing name + reference + asset_class + currency conditionally (WEB-031). Uses `<button type="button" aria-label={t("asset.web_lookup.select_result", { name: r.name })}>...</button>` for E2E (E4). Clicking calls `onSelect(result)`.
    - `empty` — `t("asset.web_lookup.no_results")` (WEB-032).
    - `error` — banner with `role="alert"` (E5) showing `t("asset.web_lookup.error_network")` and a retry button labelled `t("asset.web_lookup.action_retry")` (WEB-033). Retry re-issues the last query.
  - "Fill manually" link/button always visible (WEB-013, WEB-032, WEB-033) — `aria-label={t("asset.web_lookup.action_fill_manually")}`.
- Props: `{ onSelect: (r: AssetLookupResult) => void; onFillManually: () => void; initialState?: SearchState }` (initialState consumed once on mount to support WEB-047 cache restore).

##### File: `useWebLookupSearch.ts`

- Hook owns: `query: string`, `state: SearchState` where `SearchState = { kind: "idle" } | { kind: "loading"; query: string } | { kind: "results"; query: string; results: AssetLookupResult[] } | { kind: "empty"; query: string } | { kind: "error"; query: string }`.
- Exposes: `query`, `setQuery`, `state`, `submit()`, `retry()`, `getCachedSearchState()` (used by `WebLookupModal` to persist cache for WEB-047), `restoreState(s: SearchState)`.
- `submit()`:
  1. Guard: if `query.trim().length === 0`, no-op (WEB-011).
  2. Set state to `loading`.
  3. Call `assetGateway.searchAssetWeb(trimmedQuery)`.
  4. On `result.error` → set state to `error` and `logger.error("[useWebLookupSearch] network error", { error: result.error })` (F14, WEB-033).
  5. On `result.data.length === 0` → state `empty` (WEB-032).
  6. Else → state `results` (WEB-031).
- Tests (`useWebLookupSearch.test.ts`, F18, F19) — non-trivial behaviour only:
  - submit with empty query is a no-op (no gateway call) — WEB-011
  - submit transitions idle → loading → results — WEB-030 / WEB-031
  - empty array transitions to `empty` — WEB-032
  - gateway error transitions to `error` and logs — WEB-033
  - retry re-issues the last query and clears the error — WEB-033
  - calling `submit` while in `loading` is ignored (WEB-030)

##### File: `useWebLookupModal.ts`

- Hook owns the `step` state and the cached `SearchState` for WEB-047.
- Exposes: `step`, `selected: AssetLookupResult | null`, `cachedSearchState`, `goToFormFromResult(r)`, `goToFormManual()`, `goBackToSearch(currentSearchState)`, `closeAll()`.
- Used by `WebLookupModal` to compose `SearchPanel` + `AddAssetModal`.
- Tests (`useWebLookupModal.test.ts`):
  - selecting a result transitions to `form-prefilled` and stores the selection
  - "fill manually" transitions to `form-manual`
  - back from `form-prefilled` restores the cached search state and selected = null (WEB-047)
  - back is **not** offered when step is `form-manual` (WEB-013 — bypass path is one-way)

##### File: `useWebLookupModal.test.ts` and `useWebLookupSearch.test.ts`

- Colocated tests per F2/F18.

#### Refactor: `src/features/assets/add_asset/AddAsset.tsx` and `useAddAsset.ts`

- Add optional prop `prefill?: AssetPrefill` where:
  ```ts
  type AssetPrefill = {
    name?: string;
    reference?: string;
    currency?: string;
    class?: AssetClass; // triggers risk_level auto-fill
  };
  ```
- In `useAddAsset`:
  - Replace `prefillName?: string` with `prefill?: AssetPrefill` (or extend it; keep `prefillName` for backward compat by aliasing — confirm with reviewer).
  - Initial `formData` derives from `prefill` when provided:
    - `name` ← `prefill.name ?? ""` (WEB-041)
    - `reference` ← `prefill.reference ?? ""` (WEB-041)
    - `currency` ← `prefill.currency ?? "EUR"` (WEB-041 — blank/default if absent)
    - `class` ← `prefill.class ?? "Cash"` (WEB-041 — falls back to current default)
    - `risk_level` ← `prefill.class ? DEFAULT_RISK_BY_CLASS[prefill.class] : DEFAULT_RISK_BY_CLASS.Cash` (WEB-042 — auto-fill risk from class)
    - `category_id` ← `SYSTEM_CATEGORY_ID` (WEB-044 — no UX change)
  - `useEffect` watching `prefill` to re-seed when the user re-enters the form with a different selection (WEB-040, WEB-047).
- All pre-filled fields stay editable — no read-only flag (WEB-043).
- Save path unchanged — still calls `addAsset` from `useAssets()` (WEB-045).
- Add optional prop `onBack?: () => void` to `AddAssetModal`. When supplied, the modal renders a back `Button` (variant `secondary`, `aria-label={t("asset.web_lookup.action_back")}`) **alongside** Cancel/Add in the actions slot. The prop is `undefined` for the manual / FAB-original path, preserving current behaviour.
- Tests in `useAddAsset.test.ts`:
  - extend with: prefill seeds the form (name/reference/currency/class) — WEB-041
  - prefilling class auto-fills risk_level via `DEFAULT_RISK_BY_CLASS` — WEB-042
  - all prefilled fields remain editable — change events update state — WEB-043
  - missing prefill keeps current defaults (regression guard) — WEB-041 partial
  - submit calls `addAsset` with the same DTO shape as before — WEB-045

#### File: `src/features/assets/web_lookup/index.ts`

- `export { WebLookupModal } from "./WebLookupModal";`

#### Wire-in: `src/features/assets/AssetManager.tsx`

- Replace direct usage of `AddAssetModal` with `WebLookupModal` (WEB-010).
- The FAB now opens `WebLookupModal` instead of `AddAssetModal`. `prefillName` from the `createNew` query param flows through: when present, the modal opens directly in `step: "form-manual"` with `prefill={{ name: createNew }}` (preserving the existing "+ New asset" combobox shortcut from transactions). This keeps existing behaviour intact for the prefill-by-name shortcut while still introducing the lookup as the default entry point.
- `onSuccess` propagation unchanged — `handleAddAssetSuccess` still receives the new asset id and routes via the existing `resolveReturnNav` logic.
- No change to `useAssets`, `useAssetTable`, or any other sub-feature.

#### File: `src/i18n/locales/en/common.json` and `src/i18n/locales/fr/common.json`

- Add a new `asset.web_lookup` block (English first, French translated):
  - `title` — "Search for an instrument" / "Rechercher un instrument"
  - `idle_hint` — "Enter a name, ticker or ISIN to search."
  - `query_label` — "Search"
  - `query_placeholder` — "AAPL, US0378331005, Apple…"
  - `action_search` — "Search" / "Rechercher"
  - `action_retry` — "Retry" / "Réessayer"
  - `action_back` — "Back" / "Retour"
  - `action_fill_manually` — "Fill manually" / "Saisir manuellement"
  - `select_result` — "Select {{name}}" / "Sélectionner {{name}}" (E2E aria-label)
  - `no_results` — "No instruments found." / "Aucun instrument trouvé."
  - `error_network` — "Could not reach the lookup service. Try again or fill manually." / "Impossible de joindre le service de recherche. Réessayez ou saisissez manuellement."
  - `loading` — "Searching…" / "Recherche en cours…"
  - `results_columns_aria` — for table-row accessibility if used
- `i18n-checker` agent will verify that every key has both locale variants and that no UI string is hard-coded.

#### Frontend rules coverage check

- F1/F2: `web_lookup/` sub-feature folder colocated with `add_asset/`, hooks + components + tests live together.
- F3: only `gateway.ts` calls `commands.*`.
- F5: any UI mapping kept inside the component (rows are projected directly from the contract type — pure passthrough; presenter not needed unless reviewer asks).
- F6/F8: smart vs dumb split between `WebLookupModal`, `SearchPanel`, and `AddAssetModal`.
- F10/F19: hook logic isolated; renderHook tests use stable references (extract `onSelect`, `onClose` outside the render callback).
- F11/F12: re-uses `Dialog`, `TextField`, `Button` from `ui/components/`; no shared component is modified.
- F13/F14/F15: mount info log + critical error log via `@/lib/logger`.
- F16: 100 % i18n.
- F17: error states surface user-friendly text + log + retry; validation (empty query) is inline disable.
- F21: never redeclare `AssetClass`; consume the Specta-generated type.
- F22: no cross-feature import — the `web_lookup` sub-feature lives **inside** `features/assets/` and only imports siblings.

### 2.5 E2E tasks (`tests/e2e/specs/`)

- After frontend implementation, run `test-writer-e2e` agent with the contract.
- The agent writes a WebDriver test (e.g. `tests/e2e/specs/asset-web-lookup.e2e.ts`) that:
  - Clicks the assets FAB → asserts the lookup dialog opens (form `id="web-lookup-search-form"` exists — E1).
  - Bypass path: clicks "Fill manually" → asserts the legacy Add Asset form opens (`id="add-asset-form"`).
  - Lookup path: uses `setReactInputValue("web-lookup-search-query", "AAPL")` (E6), submits via `button[type="submit"][form="web-lookup-search-form"]` (E3).
  - Note for the agent: OpenFIGI is a live external service — the E2E should either (a) stub the network at the WebDriver level via `page.route` equivalents, or (b) assert structural behaviour only (loading state visible, then either results or `[role="alert"]` error). Coordinate with the agent on the strategy; a stub is preferred per E10 (deterministic).
- The plan does not pre-write the test file — the agent does, then iterates with the user until green per `docs/e2e-rules.md`.

### 2.6 Documentation tasks

- `ARCHITECTURE.md` → under `Use Cases (use_cases/)`, add a new sub-section:
  ```
  #### Asset Web Lookup (use_cases/asset_web_lookup/)
  Outbound HTTP use case that queries OpenFIGI to pre-fill the Add Asset form (spec: docs/spec/asset-web-lookup.md).
  - orchestrator.rs — AssetWebLookupUseCase, OpenFigiClient trait, ReqwestOpenFigiClient, AssetLookupResult value object; routes 12-char alphanumeric queries to the ISIN mapping endpoint and others to the keyword search; truncates to 10 results; maps OpenFIGI securityType to AssetClass.
  - api.rs — search_asset_web(query: String) -> Result<Vec<AssetLookupResult>, WebLookupCommandError>; single error variant NetworkError covering all failure modes (WEB-025).
  - No event emitted; downstream add_asset publishes AssetUpdated as usual.
  ```
- Under `Frontend → Features → Assets`, append: "Sub-features: `asset_table/`, `add_asset/`, `edit_asset_modal/`, `web_lookup/` (WEB — search via OpenFIGI to pre-fill the Add Asset form; bypass with Fill manually)."
- `docs/todo.md` — only update if a deferred follow-up emerges (e.g. caching, retry-with-backoff, telemetry). Entries in **English** per CLAUDE.md.
- `docs/ubiquitous-language.md` — propose two `pending` entries (per memory rule: never `confirmed` without user approval):
  - **AssetLookupResult** — transient value object returned by the OpenFIGI lookup; never persisted; used to pre-fill the Add Asset form. Status: pending.
  - **OpenFIGI lookup** — the outbound HTTP search that, given a name / ticker / ISIN, returns up to 10 candidate instruments. Status: pending.

### 2.7 ADR analysis

- Reviewed `docs/adr/`: ADR-001 (i64 monetary amounts) — **not applicable** (no money in this feature). ADR-002 (Holding) — not applicable. ADR-003/004/005 (cross-context use cases inject services) — applicable in spirit: this use case is correctly placed under `use_cases/` rather than inside `context/asset/` because it has an external dependency that is not part of the asset bounded context. ADR-006 (Unit of Work) — not applicable (no DB write).
- No new ADR required: placing an outbound HTTP use case under `use_cases/` is already the precedent set by `update_checker/` and is consistent with B0a/B19. If reviewer-arch requests one, scope it as "Outbound HTTP use cases live under `use_cases/`, not inside a bounded context".

---

## 3. Rules Coverage Matrix

| Rule    | Layer            | Task / file                                                                                           |
| ------- | ---------------- | ----------------------------------------------------------------------------------------------------- |
| WEB-010 | frontend         | `AssetManager.tsx` opens `WebLookupModal` instead of `AddAssetModal` from the FAB                     |
| WEB-011 | frontend         | `SearchPanel` disables submit when `query.trim().length === 0`; `useWebLookupSearch.submit` no-op     |
| WEB-012 | frontend         | `SearchPanel` exposes a single `TextField` — no mode selector                                         |
| WEB-013 | frontend         | "Fill manually" button always rendered in `SearchPanel`; transitions modal to `step: "form-manual"`   |
| WEB-014 | backend          | `AssetWebLookupUseCase::search` routes 12-char alphanumeric to `map_isin`, others to `search_keyword` |
| WEB-020 | backend          | `api.rs::search_asset_web` Tauri command + `AssetLookupResult` value object in `orchestrator.rs`      |
| WEB-021 | backend          | `ReqwestOpenFigiClient` issues no auth header — verified by inspection + reviewer pass                |
| WEB-022 | backend          | `orchestrator.rs::search` truncates to first 10 results                                               |
| WEB-023 | backend          | `map_security_type` private fn covers the seven mappings; unknown → `None`                            |
| WEB-024 | backend          | `orchestrator.rs::search` passes through OpenFIGI currency or `None`                                  |
| WEB-025 | backend          | `WebLookupCommandError::NetworkError` single variant; `api.rs` maps any orchestrator error to it      |
| WEB-030 | frontend         | `useWebLookupSearch` exposes `loading` state; `SearchPanel` disables submit + shows spinner           |
| WEB-031 | frontend         | `SearchPanel` results list renders name + reference + asset_class + currency conditionally            |
| WEB-032 | frontend         | `SearchPanel` empty branch shows `t("asset.web_lookup.no_results")`                                   |
| WEB-033 | frontend         | `SearchPanel` error branch with `role="alert"`, retry button, "Fill manually" stays accessible        |
| WEB-040 | frontend         | `WebLookupModal.goToFormFromResult` transitions to `step: "form-prefilled"` with selection            |
| WEB-041 | frontend         | `useAddAsset` prefill seeding (name / reference / currency / class)                                   |
| WEB-042 | frontend         | `useAddAsset` derives initial `risk_level` from `DEFAULT_RISK_BY_CLASS[prefill.class]`                |
| WEB-043 | frontend         | `AddAssetModal` fields stay editable — no read-only flag added                                        |
| WEB-044 | frontend         | `useAddAsset` keeps `category_id = SYSTEM_CATEGORY_ID` regardless of prefill                          |
| WEB-045 | frontend+backend | Save calls existing `addAsset` gateway → existing `add_asset` Tauri command, unchanged                |
| WEB-046 | backend          | `orchestrator.rs::search` sets `reference` per path: ISIN input on ISIN path, ticker on keyword path  |
| WEB-047 | frontend         | `useWebLookupModal` retains cached `SearchState`; back button restores `step: "search"` with cache    |

---

## 4. Commit Checkpoints (suggested titles for `/smart-commit`)

1. After backend green + types regenerated + `just check` clean:
   `feat(asset-web-lookup): add search_asset_web command and OpenFIGI client`
2. After frontend green + format + reviewer-frontend issues fixed:
   `feat(asset-web-lookup): add web lookup dialog before Add Asset form`
3. After E2E green:
   `test(asset-web-lookup): add E2E coverage for web lookup flow`
4. After docs + spec-checker:
   `docs(asset-web-lookup): record use case and update architecture`

> Each title is a suggestion; `/smart-commit` will draft the body from the diff and the user confirms before committing.

---

## 5. Out-of-scope (explicit non-goals)

- No caching of lookup results (results are recomputed per submit).
- No retry / backoff strategy beyond a manual user-driven retry (WEB-033).
- No OpenFIGI API key support (WEB-021); not added even speculatively.
- No telemetry / metrics for lookup latency.
- No persistence of lookup history.
- No new event published — the downstream `add_asset` already raises `AssetUpdated` (WEB-045).
- No change to the `Asset` aggregate, `AssetService`, or any repository.
- No new database migration.
