# Implementation Plan — Explicit ISIN Lookup (WEB amendment)

> Trigram: **WEB** (existing spec amendment, no new trigram)
> Spec: `docs/spec/asset-web-lookup.md`
> Contract: `docs/contracts/asset-contract.md`
> Branch: `feat/explicit-isin-lookup`

Scope: split the single `lookup_asset` entry point into two explicit paths (`Isin` / `Keyword`) with a typed mode parameter, add backend ISIN format validation (length + charset + Luhn-mod-10 check digit per ISO 6166), introduce a new `InvalidIsinFormat` typed error, tighten the ISIN-path per-share-class cap to 3 (keyword path stays at 10), and surface two stacked input fields on the frontend.

---

## 1. Workflow TaskList

### Setup

- [ ] 📖 Read spec: `docs/spec/asset-web-lookup.md` (amended — WEB-011/012/014/015/016 NEW/020/025/033/046/050a–f)
- [ ] 📖 Read contract: `docs/contracts/asset-contract.md` (Web Lookup section + `LookupMode` shared type)
- [ ] 📖 Read constraining ADRs: none directly applicable (lookup carries no monetary amounts, no cross-context orchestration). ADR-001 (i64) noted for completeness — not engaged here.
- [ ] 📖 Read conventions: `ARCHITECTURE.md` + `docs/backend-rules.md` (B7 typed errors, B11 domain vocabulary) + `docs/error-model.md` (per-BC `*ApplicationError`, wire-flat shape) + `docs/ddd-reference.md` (domain validator placement) + `docs/frontend-rules.md` (F25 stable ids, F27 typed error pipeline) + `docs/i18n-rules.md` + `docs/test_convention.md`

### Backend phase

- [ ] 🗄️ Database Migration — **not required** (lookup is read-only external HTTP, no persistence)
- [ ] ✍️ Backend test stubs (`test-writer-backend` — all stubs written, red confirmed). Anchored on `lookup_asset(query, mode)` + `InvalidIsinFormat` error variant + ISIN format validator + dual cap behavior in `process_hits`.
- [ ] 🏗️ Backend Implementation (minimal — make failing tests pass, green confirmed). Implement only what makes failing tests pass — no defensive code, no anticipation of future rules.
- [ ] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` in parallel → `/review-triage` → apply Follow-ups). `reviewer-sql` skipped (no migration). `reviewer-security` triggered (Tauri command signature change).
- [ ] 🔗 Type Synchronization (`just generate-types`)
- [ ] 🔧 Run `npx tsc --noEmit` → fix TS errors from new bindings only (no UI work)
- [ ] 🧹 `just format` (rustfmt + clippy --fix)
- [ ] 💾 Suggested commit title: `feat(asset): explicit ISIN/keyword lookup mode + format validator`

### Frontend phase

- [ ] ✍️ Frontend test stubs (`test-writer-frontend` — all stubs written, red confirmed). Pass `modified_functions` list:
  - `src/features/assets/gateway.ts:lookupAsset` — signature change (now takes `mode`)
  - `src/features/assets/web_lookup/useWebLookupSearch.ts:search` — accepts mode, dispatches to gateway with explicit path
  - `src/features/assets/web_lookup/useWebLookupModal.ts` — orchestrates per-field state (split into `isinQuery` + `keywordQuery`)
  - `src/features/assets/web_lookup/SearchPanel.tsx` — two stacked input rows with per-field submit, loading, and inline error
- [ ] 💻 Frontend Implementation (minimal — make failing tests pass)
- [ ] 📸 Visual proof (`/visual-proof` — capture lookup dialog in light + dark, idle / loading-isin / error-invalid-isin / results states)
- [ ] 🔍 Frontend Review (`reviewer-frontend` → `/review-triage` → apply Follow-ups)
- [ ] 🧹 `just format`

### Closure

- [ ] ✍️ E2E scenarios — **deferred**: WebLookupModal isn't covered by current E2E suite; existing E2E uses "Fill manually" path. Add a single happy-path ISIN lookup scenario only if the existing e2e helpers already support OpenFIGI stubbing; otherwise file as `/techdebt` (out of scope for this surgical feature).
- [ ] 🔍 Cross-cutting Review (`reviewer-security` — Tauri command signature changed → audit input handling for the new validator; `reviewer-infra` skipped — no config changes; `reviewer-e2e` skipped — no E2E delta)
- [ ] 📚 Documentation Update: nothing in `docs/todo.md` to close (this feature was not on the TODO list — surfaced from a fresh user request). `ARCHITECTURE.md` — no update needed (no new module path, the existing `use_cases/asset_web_lookup/` continues to own the surface).
- [ ] ✅ Spec check (`spec-checker`) [HARD GATE — halt on any uncovered rule or command]
- [ ] 🧹 `just format`
- [ ] 💾 Suggested closure commit title (doc/todo touchups only, if any): `chore(asset): close out ISIN lookup spec checker`
      _Note for 1-PR strategy: each phase commit is a separate logical commit in the same branch; all push together in one PR. Order on `main` after rebase-merge: BE → FE → closure._
- [ ] 🔀 `/create-pr` → single PR per the PR Plan below

---

## 2. Detailed Implementation Plan

### Migrations

None. Lookup is read-only external HTTP with no persisted state.

### Backend

**New files**:

- `src-tauri/src/context/asset/domain/isin.rs` — pure-domain ISIN format validator.
  - `pub fn validate_isin(raw: &str) -> StdResult<String, IsinFormatError>` — trims, uppercases, runs the three checks of WEB-016 (length, charset, Luhn-mod-10 check digit). Returns the normalized 12-character ISIN on success.
  - `pub enum IsinFormatError { WrongLength, InvalidCharset, BadCheckDigit }` — domain-error tuple, surfaced via a single wire variant (`InvalidIsinFormat`) per WEB-025 — the FE doesn't need granularity below "not a valid ISIN".
  - Inline unit tests covering: happy path (`IE00B53L3W79`, `US5949181045`), wrong length, lowercase-after-uppercase, non-alphanumeric, last-char-not-digit, check-digit mismatch (e.g. mutate one digit of a known-good ISIN), whitespace-trim happy path.
  - Reference table: a curated set of known-good ISINs for the inline tests (sourced from common assets — Microsoft, iShares S&P 500, BNP Paribas).

**Modified files**:

- `src-tauri/src/context/asset/domain/mod.rs` — re-export `validate_isin` and `IsinFormatError`.
- `src-tauri/src/use_cases/asset_web_lookup/error.rs` — add `InvalidIsinFormat` variant to the application/wire error enum. Follow `docs/error-model.md`: wire-flat `{ code: "InvalidIsinFormat" }` (no payload — the FE renders a single copy).
- `src-tauri/src/use_cases/asset_web_lookup/orchestrator.rs`:
  - Replace the heuristic `is_isin = trimmed.len() == 12 && all alnum` in `search()` with an explicit `mode: LookupMode` parameter.
  - On `LookupMode::Isin`: call `validate_isin(query)?` first; embed the normalized ISIN in `QueryContext { isin: Some(normalized) }` and pass it to `process_hits`. Map `IsinFormatError` → `InvalidIsinFormat` at the use-case boundary.
  - On `LookupMode::Keyword`: keep the existing diacritics normalization (WEB-015) and keyword pipeline.
  - `pub enum LookupMode { Isin, Keyword }` — Specta-typed, derives match the existing convention in this BC.
  - Thread `mode` (or just `ctx.isin.is_some()`) into `process_hits` so the per-share-class cap can branch.
- `src-tauri/src/use_cases/asset_web_lookup/api.rs` — `lookup_asset` signature: `async fn lookup_asset(state: State<_>, query: String, mode: LookupMode) -> Result<Vec<AssetLookupResult>, WebLookupApplicationError>`. Specta picks up `LookupMode` automatically.
- `src-tauri/src/use_cases/asset_web_lookup/primary_listing_processor.rs`:
  - Add `pub const MAX_ENTRIES_PER_SHARE_CLASS_ISIN: usize = 3;` next to the existing `MAX_ENTRIES_PER_SHARE_CLASS` (renamed in-doc to `_KEYWORD = 10` for clarity).
  - `pick_primary_entries` takes a `cap: usize` parameter (or `process_hits` resolves it from `ctx.isin.is_some()`). Pick: thread through `process_hits` → `pick_primary_entries` to keep the public surface minimal.
  - Update inline tests: rename existing `picks_up_to_max_per_share_class` to `keyword_path_caps_per_share_class_at_ten`; add `isin_path_caps_per_share_class_at_three`.
- `src-tauri/src/core/specta_builder.rs` — verify `LookupMode` is registered for type generation. Probably automatic via the command signature, but check.

**No changes**:

- `primary_listing_processor.rs` country-prefix logic (`ISIN_COUNTRY_TO_PRIMARY_VENUES`, `priority_for`) — already correct. The dual cap is the only new behavior.
- The OpenFIGI HTTP client (`map_isin`, `search_keyword`) — signature unchanged.
- `WebLookupApplicationError` — only one new variant added; other variants unchanged.

### Frontend

**Modified files**:

- `src/features/assets/gateway.ts` — `lookupAsset(query: string, mode: LookupMode): Promise<Result<AssetLookupResult[], WebLookupApplicationError>>`. Add the `mode` arg and forward to `commands.lookupAsset(query, mode)`.
- `src/features/assets/web_lookup/useWebLookupSearch.ts` — `search` accepts an explicit mode argument; pass it to the gateway. State machine unchanged (idle / loading / results / empty / error). Error state carries enough information (or the modal layer carries the "which field triggered the last action" flag) so the panel can render the inline error beside the right field.
- `src/features/assets/web_lookup/useWebLookupModal.ts` — split the single query state into two: `isinQuery` and `keywordQuery`. Each has its own enable rule (WEB-011) and submit handler that calls `useWebLookupSearch.search(query, mode)`. Track `lastMode` so the result/error state can be attributed back to the originating field. The shared results list is replaced when either submit fires (per WEB-031's "shared list").
- `src/features/assets/web_lookup/SearchPanel.tsx` — **this is the file that contains the input + submit + result list + states**, not `WebLookupModal.tsx` (which is the modal shell). Restructure the search section into two stacked rows:
  - Row 1 (ISIN): label `asset.web_lookup.isin_label` + `<input id="web-lookup-isin-input">` + button `id="web-lookup-isin-submit"`.
  - Row 2 (Keyword): label `asset.web_lookup.keyword_label` + `<input id="web-lookup-keyword-input">` + button `id="web-lookup-keyword-submit"`.
  - Loading spinner anchored to the field that triggered the action; both buttons disabled while loading.
  - Inline error message rendered beside the field that triggered the action (WEB-033) — uses presenter for the typed error.
  - i18n-aware aria-labels via `t()` per F24.
- `src/features/assets/web_lookup/WebLookupModal.tsx` — no structural change. Verify props plumbing for the new `lastMode` if `SearchPanel` needs it from the modal layer.
- Presenter — `grep` for the existing web-lookup error presenter (likely inline in `SearchPanel.tsx` per the current code: `error_rate_limit` / `error_network` keys are hardcoded). Extract into `src/features/assets/web_lookup/presenter.ts` if helpful for testability, OR keep inline if the extraction breaks the bit-by-bit gold rule (defer to whoever implements). Either way: add the `InvalidIsinFormat` → `asset.web_lookup.error_invalid_isin` mapping.
- `src/i18n/locales/en/common.json` — extend the existing `asset.web_lookup` namespace:
  - ADD: `isin_label` = "ISIN"
  - ADD: `isin_placeholder` = "e.g. IE00B53L3W79"
  - ADD: `isin_submit` = "Search ISIN"
  - ADD: `keyword_label` = "Name or ticker"
  - ADD: `keyword_placeholder` = "AAPL, Apple…"
  - ADD: `keyword_submit` = "Search"
  - ADD: `error_invalid_isin` = "Not a valid ISIN. Expected 12 characters with a valid check digit."
  - REMOVE (single-field obsoleted): `query_label`, `query_placeholder`, `action_search` — verify no other caller uses them via `grep -rn` before deleting.
- `src/i18n/locales/fr/common.json` — French translations of the same keys (same shape; same ADD/REMOVE).

**Modified-function coverage** (Step 5):

| File:Function                                                 | Why `[unit-test-needed]`                                             |
| ------------------------------------------------------------- | -------------------------------------------------------------------- |
| `src/features/assets/gateway.ts:lookupAsset`                  | Signature changed (new `mode` arg)                                   |
| `src/features/assets/web_lookup/useWebLookupSearch.ts:search` | Now takes mode + dispatches per path                                 |
| `src/features/assets/web_lookup/useWebLookupModal.ts`         | Split state per field + new error routing + `lastMode` tracking      |
| `src/features/assets/web_lookup/SearchPanel.tsx`              | Two stacked rows; per-field loading + inline error (WEB-012/030/033) |

**No changes**:

- `useWebLookupSearch` state machine semantics (idle/loading/results/empty/error) — only inputs and dispatching changes.
- Result row rendering (WEB-031 unchanged).
- Pre-fill behavior (WEB-040–WEB-046 unchanged from the FE perspective; WEB-046 only changes BE-side what `reference` resolves to on the ISIN path — already the case in practice).

### Rules Coverage

| Rule        | Layer              | Task                                                            | Notes                                     |
| ----------- | ------------------ | --------------------------------------------------------------- | ----------------------------------------- |
| WEB-010     | frontend           | (existing) entry point unchanged                                | no work                                   |
| WEB-011     | frontend           | `useWebLookupModal.ts` — per-field enable rule                  | amendment: ISIN button enabled at ≥1 char |
| WEB-012     | frontend           | `SearchPanel.tsx` — two stacked rows                            | `[unit-test-needed]` (existing component) |
| WEB-013     | frontend           | (existing) Fill manually unchanged                              | no work                                   |
| WEB-014     | backend            | `orchestrator.rs` — explicit `mode` parameter                   | amendment                                 |
| WEB-015     | backend            | `orchestrator.rs` — keyword path only                           | amendment (narrowed scope)                |
| WEB-016     | backend            | NEW: `context/asset/domain/isin.rs` validator                   | NEW rule                                  |
| WEB-020     | backend            | `api.rs` — `lookup_asset(query, mode)`                          | signature change                          |
| WEB-021     | backend            | (existing) no auth                                              | no work                                   |
| WEB-022     | backend            | (existing) 30-row final cap                                     | no work                                   |
| WEB-023     | backend            | (existing) asset class mapping                                  | no work                                   |
| WEB-024     | backend            | (existing) currency passthrough                                 | no work                                   |
| WEB-025     | backend            | `error.rs` — add `InvalidIsinFormat` variant                    | amendment                                 |
| WEB-030     | frontend           | `SearchPanel.tsx` — per-field loading                           | `[unit-test-needed]`                      |
| WEB-031     | frontend           | (existing) row rendering                                        | no work                                   |
| WEB-032     | frontend           | (existing) empty state                                          | no work                                   |
| WEB-033     | frontend           | `SearchPanel.tsx` + presenter — per-field inline error          | `[unit-test-needed]`                      |
| WEB-040     | frontend           | (existing) selection transition                                 | no work                                   |
| WEB-041–044 | frontend           | (existing) pre-fill, default risk, editable, category default   | no work                                   |
| WEB-045     | frontend + backend | (existing) save via add_asset                                   | no work                                   |
| WEB-046     | backend            | `orchestrator.rs` — normalized ISIN in `reference`              | follows from WEB-016 validator output     |
| WEB-047     | frontend           | (existing) back nav                                             | no work                                   |
| WEB-048     | backend            | (existing) priority sort                                        | no work                                   |
| WEB-049     | backend            | (existing) Exchange resolution                                  | no work                                   |
| WEB-050     | backend            | parent rule — splits into 050a–f                                | no direct work                            |
| WEB-050a    | backend            | (existing) Common Stock filter on keyword                       | no work                                   |
| WEB-050b    | backend            | (existing) drop null share class                                | no work                                   |
| WEB-050c    | backend            | (existing) dedup by share class                                 | no work                                   |
| WEB-050d    | backend            | (existing) share-class enrichment                               | no work                                   |
| WEB-050e    | backend            | `primary_listing_processor.rs` — dual cap (3 ISIN / 10 keyword) | amendment                                 |
| WEB-050f    | backend            | (existing) final 30-row cap                                     | no work                                   |

---

## 3. PR Plan

- **Strategy**: `1 PR`
- **Estimate**: BE ~6 files / ~280 LOC (1 new validator + tests + 4 mods); FE ~7 files / ~220 LOC (component, hooks, gateway, presenter, 2 i18n)
- **Rationale**: BE and FE are tightly coupled — the BE `mode` parameter is non-optional, so the FE MUST pass it from day one. Splitting into BE-first would leave `main` in a transitional state where the FE auto-routing is dead but the new mode isn't yet wired. Both layers fit comfortably under the ≤1000 LOC PR target.
- **PR list**:
  - **PR #1** — `feat(asset): explicit ISIN/keyword lookup with format validator`
    - Scope: BE (validator + mode + dual cap + error variant) + FE (two-field UI + per-field state + presenter + i18n) + visual proof
    - Dependency: none — branches off `main`
    - Branch suffix: `feat/explicit-isin-lookup`

---

## Notes

- ISIN sample ISINs for inline tests: `IE00B53L3W79` (iShares Core S&P 500 UCITS ETF), `US5949181045` (Microsoft), `FR0000131104` (BNP Paribas). Pick from public references; treat as test fixtures only.
- `LookupMode` enum naming: PascalCase variants `Isin` (not `ISIN`) to match Rust + Specta + existing enum conventions in the BC (`AssetClass::Stocks`, etc.).
- The error variant goes in the **application** error (per `docs/error-model.md`): `WebLookupApplicationError::InvalidIsinFormat`. Surfaces as wire-flat `{ code: "InvalidIsinFormat" }`.
- No `LookupMode` variant needs to be added to the FE presenter mode map — mode is a request param, not a response payload field.
