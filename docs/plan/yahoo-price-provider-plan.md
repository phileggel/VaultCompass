# Implementation Plan — Yahoo Finance keyless price source (Stooq + BYOK retired)

> Governing decision: [ADR-017](../adr/017-yahoo-finance-keyless-price-source.md). Spec: [`market-price.md`](../spec/market-price.md) (MKT-100/102/110/117/125 amended). Contract: [`asset-contract.md`](../contracts/asset-contract.md) (fetch commands drop `use_api_key`; `AssetPriceSource = Manual | YahooFinance`).
>
> Shape: a provider swap (add Yahoo, delete Stooq) **plus** retirement of the entire BYOK/KEY feature. ~23 deletions + ~25 modifications. **No DB migration** (keys lived in the OS keychain, not SQLite). Single combined PR.

## PR Plan

- **Strategy**: `1 PR` (user decision 2026-06-12).
- **Branch**: `feat/yahoo-price-provider`.
- **Estimate**: BE ~14 files (mostly deletions + 1 new client + 1 new mapper); FE ~11 files (mostly deletions); E2E/docs ~8. Net heavily deletion-weighted.
- **Single `/create-pr`** at the end of Phase 4. No intermediate PRs.

## Workflow TaskList

**Setup**

- [x] Read spec `market-price.md`, contract `asset-contract.md`, ADR-017.
- [x] Constraining ADRs: ADR-001 (i64 micros), ADR-012 (latest-write-wins), ADR-014 (refresh-lock scope), ADR-017 (this).

**Phase 2 — Backend** _(sonnet)_

- [ ] `test-writer-backend` → Yahoo client + exchange-suffix mapper + GBp normalization tests (red).
- [ ] Implement Yahoo + delete Stooq + delete connection BC (see Detailed Plan).
- [ ] `reviewer-backend` + `reviewer-arch` + `reviewer-security` (commands removed) → `/review-triage`.
- [ ] `just generate-types` → `bindings.ts`; `npx tsc --noEmit`; `just format`.

**Phase 3 — Frontend** _(sonnet)_

- [ ] `test-writer-frontend` for changed surfaces (red). `modified_functions`: `[presenter.ts:formatSource, App.tsx:(launch gate), useRefreshGlobalPrices.ts, useRefreshAccountPrices.ts]`.
- [ ] Delete connections feature + key gates; update MKT source badge/i18n.
- [ ] `/visual-proof` (Settings page, holding source badge); `reviewer-frontend` → `/review-triage`; `just format`.

**Phase 4 — Closure** _(sonnet)_

- [ ] `test-writer-e2e` Yahoo-fetch scenario; delete `connections.test.ts` + `keyless_fetch_mode.test.ts`; de-KEY `auto_fetch.test.ts`.
- [ ] `just test-e2e-headless` green; `reviewer-e2e` + `reviewer-security` → `/review-triage`.
- [ ] Docs closure: delete `api-key-management.md` + `connection-contract.md`; delete/supersede note for retired ADRs already done; roadmap, UL, lessons, ARCHITECTURE.
- [ ] `spec-checker` [HARD GATE]; `just format`; `/smart-commit` (per layer); `/create-pr`.

## Detailed Implementation Plan

### Backend — ADD

- `src-tauri/src/context/asset/repository/yahoo_client.rs` (NEW) — `ReqwestYahooClient` implementing the existing `PriceProvider` trait; GET `query1.finance.yahoo.com/v8/finance/chart/{symbol}` with `User-Agent: Mozilla/5.0`; parse `chart.result[0].meta.regularMarketPrice` + `.currency` + `.regularMarketTime`; branch on `chart.error` → not-found; apply MKT-125 pence normalization (GBp/ZAc/ILA ÷100 → major ISO) before returning.
- `src-tauri/src/context/asset/domain/yahoo_symbol.rs` (NEW) — `derive_yahoo_symbol(reference, exchange)`: bare ticker for US/empty suffix, `{ref}.{suffix}` otherwise, `/`→`-` class-share translation (MKT-110).
- `src-tauri/src/context/asset/domain/yahoo_exchange_mapper.rs` (NEW) — MIC → Yahoo suffix table (XLON→`L`, XETR→`DE`, XPAR→`PA`, XNAS/XNYS→`` empty, …).

### Backend — MODIFY

- `context/asset/domain/asset_price.rs` — `AssetPriceSource`: `Stooq`→`YahooFinance` (drop `Finnhub` if present); `fetch_price` signature drops the `api_key: Option<String>` param. **Also**: rewrite the enum + trait doc comments that describe Stooq/`api_key`/N/D (no transition comments — describe Yahoo) and update the ~6 existing `AssetPriceSource::Stooq` unit-test cases (lines ~168–190).
- `use_cases/asset_price_fetch/{orchestrator,dispatcher,api}.rs` — remove `use_api_key` param + Stooq key resolution; orchestrator no longer depends on `ConnectionService`; dispatcher calls `fetch_price(&symbol)`.
- `core/specta_builder.rs` — drop the **4 connection commands** AND the **8 connection `.typ::<>()` type registrations** (`Provider`, `StorageTier`, `ProviderKeyTestOutcome`, `ProviderConnection`, `ConnectionError`, `SaveProviderKeyArgs`, `TestProviderKeyArgs`, `RemoveProviderKeyArgs`) AND the `context::{… connection}` import (line 2). Drop the 2 `use_api_key` fetch args.
- `lib.rs` — **live DI rewiring** (not just module decls): replace `ReqwestStooqClient::new()` with `ReqwestYahooClient::new()` as the injected `price_provider` (lines ~243–251); delete the `ConnectionService::new(LayeredKeyStore, StooqProbe)` construction + the `connection_keys_dir` plumbing; drop the `ReqwestStooqClient` (line 18) and `ConnectionService/LayeredKeyStore/StooqProbe` (line 21) imports.
- `context/mod.rs` — drop `pub mod connection` (line 6); drop Stooq module decls in `context/asset/{domain,repository}/mod.rs`.

### Backend — DELETE

- `context/asset/repository/stooq_client.rs`, `context/asset/domain/stooq_symbol.rs`, `context/asset/domain/stooq_exchange_mapper.rs`.
- `shared/infrastructure/stooq.rs` (proof-of-work `StooqGate`).
- `context/connection/` — entire BC (12 `.rs` files incl. `infrastructure/stooq_probe.rs`).
- BE integration test `tests/asset_price_fetch_crud.rs` — drop `StubKeyStore`/`StubProbe`, the `use_api_key` args.

### Frontend — MODIFY

- `features/settings/{useSettings.ts,SettingsPage.tsx}` — remove the "Use Stooq API key" toggle.
- `features/accounts/refresh_prices/useRefreshGlobalPrices.ts` + `features/account_details/refresh_prices/useRefreshAccountPrices.ts` — remove KEY-040 key-gate; call fetch directly (no `use_api_key` arg).
- `App.tsx` — remove the `shouldLaunchFetch(connections)` gate (the `provider==="Stooq" && has_key` check), the `connectionGateway` import (line 7), and the `connectionGateway.getProviderConnections()` call (lines ~41–42); re-gate auto-fetch launch on the auto-fetch setting (MKT-120) alone.
- `App.test.ts` — rewrite: drop the KEY-041 / `setUseStooqApiKey` / `connectionGateway`-mock cases; keep a launch test re-gated on the MKT-120 setting.
- shell nav — remove the "Connections" entry. **Verify location first** (plan-reviewer found no `SideMenu` ref; trace the `nav.connections` i18n consumer — the entry may live elsewhere or be already absent).
- `features/account_details/shared/presenter.ts` — `formatSource`: `Stooq` arm → `YahooFinance` arm returning `mkt.source_yahoo`.

### Frontend — DELETE

- `features/connections/` (7 files: ConnectionsModal.tsx + ConnectionsModal.integration.test.tsx + gateway.ts + gateway.test.ts + useProviderRow.ts + shared/presenter.ts + shared/presenter.test.ts).
- `lib/stooqKeyModeStorage.ts`; any connections shell mount.
- i18n (en + fr): `mkt.source_stooq` (add `mkt.source_yahoo`), the `connections.*` / `key.*` blocks, and reword `auto_fetch_description` (currently says "from Stooq" → Yahoo Finance).

### Rules Coverage

| Rule                     | Layer   | Task                                      | Notes                                           |
| ------------------------ | ------- | ----------------------------------------- | ----------------------------------------------- |
| MKT-100                  | backend | `AssetPriceSource = Manual\|YahooFinance` | enum in asset_price.rs                          |
| MKT-101                  | backend | user-driven writes stay `source=Manual`   | unchanged; verify Manual arm survives enum edit |
| MKT-102                  | backend | fetch writes `source=YahooFinance`        | dispatcher                                      |
| MKT-110                  | backend | `derive_yahoo_symbol` + exchange mapper   | new files; `[unit-test-needed]`                 |
| MKT-114                  | backend | unknown symbol (`chart.error`) → skip     | yahoo_client not-found branch                   |
| MKT-117                  | backend | observation date from Yahoo timestamp     | yahoo_client                                    |
| MKT-118                  | backend | always record; today-fallback on bad date | yahoo_client (inverse of MKT-114 skip)          |
| MKT-125                  | backend | GBp/ZAc/ILA ÷100 normalization            | yahoo_client; `[unit-test-needed]`              |
| asset-contract fetch\_\* | backend | drop `use_api_key`                        | orchestrator/dispatcher/api/specta              |

> New-code tasks (`yahoo_client.rs`, `yahoo_symbol.rs`, `yahoo_exchange_mapper.rs`): implement only what makes the failing `test-writer-backend` tests pass — no speculative multi-provider abstraction (BYOK extensibility is intentionally removed per ADR-017).

## Notes

- Minimal implementation: implement only what makes failing tests pass — no speculative multi-provider abstraction (BYOK extensibility is intentionally removed per ADR-017).
- `PriceProvider` trait stays (one impl now: Yahoo); do not collapse it away — `test-writer-backend` mocks it.
- Path list verified via Glob 2026-06-12.
