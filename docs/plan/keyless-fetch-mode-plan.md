# Implementation Plan — Stooq Keyless Fetch Mode (KEY-050–055)

> Amendment to the shipped KEY feature. Spec: `docs/spec/api-key-management.md` (KEY-050–055). Contract: `docs/contracts/asset-contract.md` (`fetch_*` gain `use_api_key: bool`). ADR: [ADR-016](../adr/016-stooq-optional-keyless-fetch-mode.md) (supersedes ADR-015).
> Only the keyless-mode delta is in scope; KEY-010–044 are already implemented.

---

## 1. Workflow TaskList

**Setup**

- [ ] 📖 Read: `docs/spec/api-key-management.md` (KEY-050–055), `docs/contracts/asset-contract.md` (fetch tasks), [ADR-016](../adr/016-stooq-optional-keyless-fetch-mode.md), `docs/backend-rules.md`, `docs/error-model.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/test_convention.md`

**Backend phase**

- [ ] 🗄️ No migration (no schema change — the mode is a per-request flag, never persisted backend-side)
- [ ] ✍️ Backend test stubs (`test-writer-backend` — keyless fetch omits apikey; keyed-no-key still short-circuits; keyless never short-circuits; red confirmed)
- [ ] 🏗️ Backend implementation (minimal — only what makes the failing tests pass; no defensive code)
- [ ] 🔍 Backend review (`reviewer-backend` + `reviewer-arch` + **`reviewer-security`** [fetch commands gain an arg; keyless URL omits the key] in parallel → `/review-triage`)
- [ ] 🔗 `just generate-types` → `src/bindings.ts`
- [ ] 🔧 `npx tsc --noEmit` → fix binding-driven TS errors only
- [ ] 🧹 `just format`
- [ ] _(no separate BE commit — single PR; continue to FE)_

**Frontend phase**

- [ ] ✍️ Frontend test stubs (`test-writer-frontend` from the modified hooks + new storage/settings; pass `modified_functions` from § Rules Coverage; red confirmed)
- [ ] 💻 Frontend implementation (minimal)
- [ ] 📸 `/visual-proof` — SettingsPage with the new "Use Stooq API key" toggle (on/off, light + dark)
- [ ] 🔍 Frontend review (`reviewer-frontend` → `/review-triage`)
- [ ] 🧹 `just format`
- [ ] 💾 Commit: `feat(connection): optional keyless Stooq fetch mode` via `/smart-commit` [HARD GATE]

**Closure**

- [ ] ✍️ E2E (`test-writer-e2e`) — Settings toggle off → global refresh dispatches without opening Connections (KEY-051); toggle on + no key → refresh opens Connections (KEY-040 unchanged). No live Stooq.
- [ ] ▶️ `just test-e2e-headless` → green
- [ ] 🔍 `reviewer-e2e` → `/review-triage`
- [ ] 📚 Docs: ARCHITECTURE.md only if a new pattern (none expected); tick this plan
- [ ] ✅ `spec-checker` [HARD GATE — KEY-050–055 + the `use_api_key` arg on both fetch commands]
- [ ] 🧹 `just format`
- [ ] 💾 Commit: closure via `/smart-commit`
- [ ] 🔀 `/create-pr` (single PR — BE+FE+E2E)

---

## 2. Detailed Implementation Plan

### Backend

| #   | File                                                                                                                 | Task                                                                                                                                                                                                                                                               |
| --- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| B1  | `src-tauri/src/context/asset/domain/asset_price.rs`                                                                  | `PriceProvider::fetch_price(&self, symbol, api_key: Option<&str>)` — change `&str` → `Option<&str>`. Update the mockall mock + doc comment (None = keyless/anonymous, KEY-053).                                                                                    |
| B2  | `src-tauri/src/context/asset/repository/stooq_client.rs`                                                             | `fetch_price` builds the `q/d/l/` URL with `&apikey={key}` only when `api_key` is `Some`; omits it when `None` (anonymous, KEY-053). PoW gate (StooqGate) unchanged — solved in both modes (KEY-043/053). Parse/window logic unchanged.                            |
| B3  | `src-tauri/src/use_cases/asset_price_fetch/dispatcher.rs`                                                            | `spawn` gains `use_api_key: bool`. The KEY-044 short-circuit fires only when `use_api_key && stooq_key.is_none()`; in keyless mode (`!use_api_key`) it is suppressed (KEY-053) and the loop calls `fetch_price(&symbol, stooq_key.as_deref())` (None → anonymous). |
| B4  | `src-tauri/src/use_cases/asset_price_fetch/orchestrator.rs`                                                          | `fetch_all` / `fetch_for_account` gain `use_api_key: bool`. Resolve the Stooq key only when `use_api_key` (skip the keychain read in keyless mode); pass both `use_api_key` and the resolved `Option<String>` to `spawn`.                                          |
| B5  | `src-tauri/src/use_cases/asset_price_fetch/api.rs`                                                                   | Both commands gain `use_api_key: bool`, forwarded to the orchestrator (contract).                                                                                                                                                                                  |
| B6  | (no specta_builder change — commands already registered; only their args change, picked up by `just generate-types`) | —                                                                                                                                                                                                                                                                  |

> Note: FX refresh (Frankfurter/ECB) in `spawn` is **unaffected** by `use_api_key` — it never used a Stooq key (ADR-009 keyless). The flag governs only the Stooq apikey branch.

### Frontend

| #   | File                                                                              | Task                                                                                                                                                                                           |
| --- | --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/lib/stooqKeyModeStorage.ts` (new)                                            | Mirror `autoFetchStorage.ts`: `getUseStooqApiKey(): boolean` (**default true** — absent key ⇒ keyed, KEY-050/054) + `setUseStooqApiKey(enabled)`. localStorage key `stooq_use_api_key`.        |
| F2  | `src/features/settings/useSettings.ts` (+ test)                                   | Add `useApiKey` state (from F1) + `toggleUseApiKey` (mirrors `toggleAutoFetch`).                                                                                                               |
| F3  | `src/features/settings/SettingsPage.tsx` (+ test)                                 | Add the "Use Stooq API key" toggle row with a help hint (KEY-050); stable id `settings-use-api-key`.                                                                                           |
| F4  | `src/features/accounts/refresh_prices/useRefreshGlobalPrices.ts` (+ test)         | Read `getUseStooqApiKey()`. Keyless → dispatch `fetchAllAssetPrices(false)` directly, no gate (KEY-051). Keyed → existing KEY-040 gate, then `fetchAllAssetPrices(true)`. `[unit-test-needed]` |
| F5  | `src/features/account_details/refresh_prices/useRefreshAccountPrices.ts` (+ test) | Same branch for `fetchAccountAssetPrices(accountId, useApiKey)` (KEY-051). `[unit-test-needed]`                                                                                                |
| F6  | `src/App.tsx` (+ test)                                                            | `shouldLaunchFetch`: in keyless mode the KEY-041 no-key skip does not apply — launch dispatches with `use_api_key=false` (KEY-052). Keyed mode unchanged. `[unit-test-needed]`                 |
| F7  | `src/features/{accounts,account_details}/gateway.ts` (+ tests)                    | `fetch*` gateway calls pass the new positional `use_api_key` arg (match fresh `bindings.ts`).                                                                                                  |
| F8  | `src/i18n/locales/{en,fr}/common.json`                                            | `settings.use_api_key` label + help hint.                                                                                                                                                      |

### Rules Coverage

| Rule    | Layer              | Task                                                                | Notes                                           |
| ------- | ------------------ | ------------------------------------------------------------------- | ----------------------------------------------- | -------------------- |
| KEY-050 | frontend           | F1, F2, F3, F8                                                      | setting, default keyed                          |
| KEY-051 | frontend           | F4, F5                                                              | gate bypass in keyless                          | `[unit-test-needed]` |
| KEY-052 | frontend           | F6                                                                  | launch not skipped in keyless                   | `[unit-test-needed]` |
| KEY-053 | backend            | B1, B2, B3, B4                                                      | keyless URL omits key; short-circuit suppressed |
| KEY-054 | frontend + backend | F4/F5/F6 keyed branch + B3 short-circuit retained                   | default-mode unchanged                          |
| KEY-055 | frontend + backend | F4/F5/F6 read setting at dispatch + B3/B4 mode travels with request | mode fixed per task                             |

**`modified_functions`**: `[accounts/refresh_prices/useRefreshGlobalPrices.ts:refresh, account_details/refresh_prices/useRefreshAccountPrices.ts:refresh, App.tsx:shouldLaunchFetch]`

---

## 3. PR Plan

- **Strategy**: `1 PR`
- **Estimate**: BE ~5 files / ~80 LOC · FE ~10 files / ~220 LOC. The fetch-command signature change couples BE bindings and FE gateway in lockstep, so a single PR is correct (splitting would ship a BE PR whose new arg no caller uses, then an FE PR that can't compile against old bindings).
- **PR — `feat(connection): optional keyless Stooq fetch mode`**: BE (B1–B5) + `just generate-types` + FE (F1–F8) + E2E + spec-checker closure. Branch: `feat/stooq-keyless-toggle` (current). After merge → release (price-fetch fix).
