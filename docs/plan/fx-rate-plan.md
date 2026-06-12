# Implementation Plan — Foreign Exchange Rate (FXR)

> **Status: ✅ SHIPPED 2026-06-02 — PRs #63 / #64 / #65 / #66 / #67 (all merged to `main`).** `spec-checker` PASS (46/46 FXR rules + 6/6 contract commands). One deferred follow-up: FXR-090 FX-staleness backend wiring — tracked in `docs/techdebt.md`. The unticked process boxes below are historical (the steps were performed across the 4 PRs); kept for the record.

> Spec: `docs/spec/fx-rate.md` · Contract: `docs/contracts/currency-contract.md`
> New bounded context: `currency` (`src-tauri/src/context/currency/`) + new FE feature (`src/features/currency/`) + a valuation lift across three existing use-case orchestrators.
> Both spec and contract passed `spec-reviewer` + `contract-reviewer` with 0 critical.

**Backend layout = gold (B0/B37–B43)**: the new `currency` BC uses the symmetric `application/ domain/ infrastructure/` trio (`infrastructure/`, NOT `repository/` per B40) + `service.rs` + `api.rs` + `error.rs` + `mod.rs`. Commands register in the existing `src-tauri/src/core/specta_builder.rs` (the codebase has not yet migrated `core/` → `shared/infrastructure/`; match the current location — bit-by-bit, do not migrate).

**Frontend layout = gold (F0/F26–F28)**: new `src/features/currency/` (gateway at root, snake_case sub-feature folders, `shared/`). The holding-row shortcut goes through the shell URL-modal mount (`src/features/shell/CurrencyRateEditMount.tsx`) mirroring `AssetEditModalMount` / `CashTransactionEditMount` so `account_details` never imports `currency` (F26). `modalSearch.ts` stays at `src/lib/modalSearch.ts` (existing location — small addition, no migration).

---

## 1. Workflow TaskList

### Setup _(read once before coding)_

- [x] 📖 Read spec: `docs/spec/fx-rate.md`
- [x] 📖 Read contract: `docs/contracts/currency-contract.md`
- [x] 📖 Read constraining ADRs: `docs/adr/001-use-i64-for-monetary-amounts.md`, `docs/adr/003-cross-context-use-case-orchestration.md`, `docs/adr/004-use-cases-inject-services-not-repositories.md`, `docs/adr/006-unit-of-work.md` (only if a command does a multi-aggregate write), `docs/adr/009-fx-rate-provider-chain.md`, `docs/adr/012-latest-write-wins-source-as-metadata.md`
- [x] 📖 Read conventions: `ARCHITECTURE.md`, `docs/test_convention.md` (always); `docs/backend-rules.md` + `docs/ddd-reference.md` + `docs/error-model.md` + `docs/backend-patterns.md` (BE); `docs/frontend-rules.md` + `docs/i18n-rules.md` + `docs/frontend-visual-proof.md` (FE)
- [x] 📖 Reference the analog: `context/asset/` (AssetPrice domain + repo), `use_cases/asset_price_fetch/` (the fetch task to piggyback), `use_cases/account_details/orchestrator.rs` (valuation), `src/features/shell/AssetEditModalMount.tsx` (URL-modal mount)

### Backend phase A — currency BC (PR 1)

- [x] 🗄️ Migration: create `currency_pairs` + `currency_rates` tables (`just migrate` + `just prepare-sqlx`)
- [x] ✍️ Backend test stubs (`test-writer-backend` — from `currency-contract.md`: 6 commands; red confirmed)
- [x] 🏗️ Backend implementation (minimal — make failing tests pass; no defensive/anticipatory code)
- [x] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` + `reviewer-sql` in parallel → `/review-triage` → apply Follow-ups; halt on (b)/(c))
- [x] 🔗 `just generate-types` → updates `src/bindings.ts`
- [x] 🔧 `npx tsc --noEmit` → fix TS errors from new bindings only (no UI work)
- [x] 🧹 `just format`
- [x] 💾 Commit: `feat(currency): currency bounded context + manual rate CRUD` via `/smart-commit`
- [x] 🔀 `/create-pr` (PR 1 — BC backend). After merge, branch PR 2 off updated `main`. ✅ merged as PR #63 (`b358b4e`)

### Backend phase B1 — multi-currency valuation lift (PR 2a)

> Split from the original PR 2 (2026-06-01): the valuation lift and the provider fetch are two independent stories. The lift depends only on stored rates → works on manually-entered rates and is the user-visible payoff. Provider fetch follows as PR 2b.

- [x] ✍️ Backend test stubs (`test-writer-backend` — `latest_rate_on_or_before` repo method, `resolve_rate_micros` read port, the 3 orchestrators' conversion paths; red confirmed)
- [x] 🏗️ Implementation (implement only what makes failing tests pass — no defensive code): reintroduce `latest_rate_on_or_before` on `CurrencyRateRepository` + impl; add `CurrencyService::resolve_rate_micros`; `Arc`-ify `CurrencyService` (+ update `api.rs` `State` signatures); inject into the 3 orchestrators; lift the 4 currency guards (FXR-030–035/040–042)
- [x] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` in parallel → `/review-triage` → apply Follow-ups; halt on (b)/(c)) — 3× B14 import + BTreeSet dedup applied; `period_end_dates` duplication filed to `docs/techdebt.md`
- [x] 🔗 `just generate-types` (valuation lift reuses existing `HoldingDetail`/`AccountDetailsResponse` shapes → doc-comment-only diff in `bindings.ts`, no wire change)
- [x] 🧹 `just format`
- [x] 💾 Commit: `feat(currency): multi-currency valuation lift` via `/smart-commit` (`255c8f1`)
- [x] 🔀 `/create-pr` (PR 2a — valuation lift). After merge, branch PR 2b off updated `main`.

### Backend phase B2 — FX provider fetch (PR 2b)

- [x] ✍️ Backend test stubs (`test-writer-backend` — cross-rate math, provider-chain fallback, fetch-then-store paths; red confirmed)
- [x] 🏗️ Implementation (minimal): `RateProvider` trait + Frankfurter/ECB clients + EUR cross-rate (FXR-080–083); `CurrencyService::refresh_all_rates` (provider field added via `with_rate_provider`); piggyback into `asset_price_fetch` (ensure-then-fetch, shares MKT-113 guard)
- [x] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` + `reviewer-security` + `reviewer-infra` _(new dep)_ → `/dep-audit` → `/review-triage`) — 5 (a) hardening fixes (finite/positive guards, cross-rate `i64::try_from`, version pin, test assert); 2 (b) → `docs/techdebt.md`; quick-xml CVE-clean
- [x] 🧹 `just format`
- [x] 💾 Commit: `feat(currency): FX provider fetch (Frankfurter + ECB)` via `/smart-commit` (`433a6ea`)
- [x] 🔀 `/create-pr` (PR 2b — provider fetch). After merge, branch PR 3 off updated `main`.

### Frontend phase — currency feature (PR 3)

- [x] ✍️ Frontend test stubs (`test-writer-frontend` — gateway unit, presenter unit, RTL for the Currency Rates view + forms + mount; pass `modified_functions` list below; red confirmed)
- [x] 💻 Frontend implementation (implement only what makes failing tests pass — no defensive code, no anticipation of future rules)
- [x] 📸 `/visual-proof` — capture Currency Rates view (list/empty/loading/error), Add-pair form, record/edit/delete rate modals, the holding-row staleness label + converted values, light + dark
- [x] 🔍 Frontend Review (`reviewer-frontend` + `reviewer-arch` _(`.ts`/`.tsx` touched)_ → `/review-triage`)
- [x] 🧹 `just format`
- [x] 💾 Commit: `feat(currency): Currency Rates view + holding-row FX shortcut` via `/smart-commit`
- [x] 🔀 `/create-pr` (PR 3 — frontend). After merge, branch PR 4 off updated `main`.

### Closure phase — E2E + docs (PR 4)

- [x] ✍️ E2E scenarios (`test-writer-e2e` — declare pair → record rate → foreign holding values live; run `/setup-e2e` first if needed)
- [x] ▶️ `npm run test:e2e` → green (main agent triages failures)
- [x] 🔍 Cross-cutting Review (`reviewer-e2e` _(E2E files)_ + `reviewer-security` _(if fetch/capability touched and not already reviewed in PR 2)_ in parallel → `/review-triage`)
- [x] 📚 Docs: `ARCHITECTURE.md` (new `currency` BC + `CurrencyRateUpdated` event-bus row + FXR-037 subscription note), `docs/contracts/account-contract.md` (add `CurrencyRateUpdated` to Subscribed events), `docs/ubiquitous-language.md` (add `CurrencyPair` / `CurrencyRate` / `CurrencyRateSource`; distinguish valuation rate from frozen per-transaction `exchange_rate`), `docs/todo.md` (close the FXR entry), `docs/roadmap.md` (Phase 3 multi-currency)
- [x] ✅ `spec-checker` [HARD GATE — every FXR rule + every `currency-contract.md` command covered; halt on any gap]
- [x] 🧹 `just format`
- [x] 💾 Commit: `test(currency): FX E2E + spec closure` via `/smart-commit`
- [x] 🔀 `/create-pr` (PR 4 — E2E + closure)

---

## 2. Detailed Implementation Plan

### Migrations (PR 1)

Next sequence after `202605310001_add_dividend_transaction_type.sql`:

- `src-tauri/migrations/202606010001_create_currency_pairs.sql`
  - `currency_pairs ( from_currency TEXT NOT NULL, to_currency TEXT NOT NULL, PRIMARY KEY (from_currency, to_currency) )`
  - Wrapped in a transaction; `CREATE TABLE IF NOT EXISTS` (reviewer-sql idempotency). Both columns NOT NULL.
- `src-tauri/migrations/202606010002_create_currency_rates.sql`
  - `currency_rates ( from_currency TEXT NOT NULL, to_currency TEXT NOT NULL, date TEXT NOT NULL, rate INTEGER NOT NULL, source TEXT NOT NULL, PRIMARY KEY (from_currency, to_currency, date) )`
  - `rate` = i64 micros (ADR-001); `source` = text discriminant `Manual|Frankfurter|Ecb` (FXR-100). FK index on `(from_currency, to_currency)` per reviewer-sql FK-index rule (references `currency_pairs`). Consider `FOREIGN KEY (from_currency, to_currency) REFERENCES currency_pairs(from_currency, to_currency)` — confirm SQLite composite-FK ergonomics during impl; if it complicates the upsert-ensures-pair path (FXR-013), keep the index without a hard FK and document the choice.
  - Run `just migrate` then `just prepare-sqlx` before writing any `sqlx::query!` code.

### Backend — currency BC (PR 1) · `src-tauri/src/context/currency/`

- `mod.rs` — module wiring (`application`, `domain`, `infrastructure`, `service`, `api`, `error`).
- `domain/mod.rs`, `domain/currency_pair.rs` — `CurrencyPair` aggregate. Factories: `new(from, to)` (validates ISO 4217 + `from != to`, FXR-023/011), `from_storage(from, to)` (no validation). No mutating methods (pair is immutable once created; FXR-014).
- `domain/currency_rate.rs` — `CurrencyRate` aggregate + `CurrencyRateSource` enum `{ Manual, Frankfurter, Ecb }` (FXR-100). Factories: `new(from, to, date, rate, source)` (validates rate > 0 FXR-021, date well-formed + not future FXR-022, ISO 4217 + `from != to` FXR-023), `from_storage(...)` (no validation). Rate stored i64 micros (ADR-001, FXR-024).
- `error.rs` (BC root) — **single flat `CurrencyError` enum** per the gold error model (`error-model.md`: one `{BC}Error` per BC, NOT the `*ApplicationError`/`*DomainError` split the older `asset` BC still carries — that split is an explicit anti-pattern). `#[serde(tag = "code")]`. Variants hold the full failure surface: `NotPositive`, `NonFinite`, `DateInFuture`, `InvalidDateFormat { date }`, `InvalidCurrency { currency }`, `IdentityPair`, `RateNotFound { from_currency, to_currency, date }`, `DatabaseError`. Domain factories + service methods all return `Result<_, CurrencyError>` directly (never `anyhow`; `StdResult` alias while anyhow is in scope). No use-case composite — the 6 commands are BC-internal, not cross-BC orchestration.
- `infrastructure/mod.rs`, `infrastructure/currency_pair.rs` — repository trait impl: `upsert_pair` (idempotent ensure, FXR-013/054), `list_pairs`, `list_pairs_with_latest_rate` (for `CurrencyPairSummary`, FXR-051 — join most-recent rate per FXR-035). Trait declared in `domain/currency_pair.rs` (B2). Flat in `infrastructure/` (B41).
- `infrastructure/currency_rate.rs` — repository trait impl: `upsert_rate` (latest-write-wins per `(from,to,date)`, FXR-025/ADR-012), `delete_rate` (FXR-053), `list_rates_for_pair(from, to)` (date desc, FXR-050), `latest_rate_on_or_before(from, to, date)` (FXR-035 — used by the valuation lift in PR 2). Trait in `domain/currency_rate.rs`.
- _(no `application/error.rs` — see the single `error.rs` above; infra failures translate to `CurrencyError::DatabaseError` at the repo call site, logged via `tracing::error!` with the BACKEND target constant)_
- `service.rs` — `CurrencyService`:
  - `declare_currency_pair(from, to) -> CurrencyPair` (FXR-054, idempotent)
  - `record_currency_rate(from, to, date, rate_micros) -> CurrencyRate` (FXR-025; ensures the pair exists first, FXR-013 ergonomics; `source = Manual`, FXR-101)
  - `update_currency_rate(from, to, original_date, new_date, new_rate_micros)` (FXR-052; same-date = in-place, changed date = delete-old + upsert-new mirroring MKT-083/084; `RateNotFound` if original missing; `source = Manual`)
  - `delete_currency_rate(from, to, date)` (FXR-053; `RateNotFound` if absent)
  - `list_currency_pairs() -> Vec<CurrencyPairSummary>` (FXR-051)
  - `list_currency_rates(from, to) -> Vec<CurrencyRate>` (FXR-050)
  - Each mutating call publishes `CurrencyRateUpdated` (FXR-026/052/053). Single-aggregate writes → no UoW (B-rule); the ensure-pair-then-write in `record_currency_rate` touches two tables in one BC — wrap in the BC's own transaction (assess UoW per ADR-006 only if it spans aggregates atomically; document the call).
- `api.rs` — 6 `#[tauri::command]` handlers accepting `rate: f64` / `new_rate: f64` and converting to i64 micros at the boundary (FXR-024). Map `CurrencyApplicationError` to the flat wire shape. Return contract types.
- `core/event_bus/event.rs` — add `CurrencyRateUpdated` variant (bare, no payload); ensure the forwarder emits discriminant `"CurrencyRateUpdated"` (FXR-026/037).
- `core/specta_builder.rs` — register the 6 commands (B3 — the only registration site).
- `context/mod.rs` — add `pub mod currency;`.

**Shared types surfaced to FE** (Specta): `CurrencyPair`, `CurrencyRate`, `CurrencyRateSource`, `CurrencyPairSummary` (exact fields per `currency-contract.md` § Shared Types).

### Backend — provider fetch + valuation lift (PR 2)

- `context/currency/infrastructure/frankfurter_client.rs` — Frankfurter HTTP client (`api.frankfurter.dev`), EUR-base (FXR-070, ADR-009). Parses JSON.
- `context/currency/infrastructure/ecb_client.rs` — ECB XML feed fallback (`ecb.europa.eu/.../eurofxref-daily.xml`), EUR-base (FXR-070).
- Cross-rate computation (FXR-080–083): a domain/infra helper computing `rate(from→to) = rate(EUR→to) / rate(EUR→from)` with i128 intermediates and truncating division (FXR-082); degenerate-leg collapse (FXR-080); same-date legs (FXR-081); missing-leg → skip (FXR-083). Decide placement — pure math in `domain/`, HTTP in `infrastructure/`.
- A `CurrencyService::fetch_rates_for_pairs(pairs)` that runs the provider chain per pair, writes `source = Frankfurter|Ecb` (FXR-102), skips per-pair failures silently (FXR-073), publishes `CurrencyRateUpdated` on each success (FXR-074).
- `use_cases/asset_price_fetch/orchestrator.rs` (+ `dispatcher.rs` / `guard.rs`) — extend the existing fetch task: after ensuring pairs for active foreign holdings in scope (FXR-013/071), call the currency fetch; shares the existing single-fetch guard (MKT-113 / FXR-076). Empty FX scope is a no-op, not an error (FXR-072). Inject `CurrencyService` per ADR-004.
- `use_cases/account_details/orchestrator.rs` — inject a currency-rate read port; when `asset_currency != account_currency`, resolve `latest_rate_on_or_before(asset_ccy, account_ccy, today)` (FXR-035), convert `current_price × rate` (i128, FXR-030), compute `unrealized_pnl` (FXR-031), `performance_pct` (FXR-032), `total_return_pct` (FXR-033); include converted value in `total_unrealized_pnl` (FXR-040) + `total_global_value` (FXR-041); no usable rate → existing `None`/`0` path (FXR-034). Lifts MKT-033/034/035 + DIV-071 + CSH-094 guards.
- `use_cases/account_summary/orchestrator.rs` — `total_global_value` includes converted foreign holdings (FXR-041 / ACC-021), same rate-resolution.
- `use_cases/account_performance/orchestrator.rs` — period `end_value` values a foreign holding at `qty × (price ≤ period end) × (rate ≤ period end)` (FXR-042 / PRF-020/024); no usable rate as-of-period-end → 0.
- ⚠️ These three orchestrators currently inject services per ADR-004/005 — add the currency read as another injected service, NOT a repository. No new command; rides existing `get_account_details` / `get_account_summaries` / `get_account_performance`.

### Frontend — currency feature (PR 3) · `src/features/currency/`

- `gateway.ts` — the only file calling `commands.*`: `declareCurrencyPair`, `recordCurrencyRate`, `updateCurrencyRate`, `deleteCurrencyRate`, `getCurrencyPairs`, `getCurrencyRates`. Typed Result pass-through (F27). Match `bindings.ts` signatures exactly (positional args; `rate` as number/f64).
- `shared/presenter.ts` — `currencyErrorToI18n(code)` (error.code → i18n key, F27), rate formatting (micros → decimal), staleness label "Rate as of today" / "Rate Nd old" (FXR-090), source badge label (FXR-102).
- `shared/validateRateForm.ts` — inline validation (rate > 0 FXR-021, date ≤ today FXR-022, distinct ISO codes FXR-023).
- `currency_rates_view/` — `CurrencyRatesView.tsx` (pair list via `getCurrencyPairs`; empty/loading/error states; FXR-051), `useCurrencyRatesView.ts` (subscribes to `CurrencyRateUpdated` re-fetch). Drill-in lists one pair's rates via `getCurrencyRates(from,to)`.
- `declare_pair/` — `DeclarePairModal.tsx` + `useDeclarePair.ts` (FXR-054/055: required from/to, submit disabled while empty/equal, idempotent-success shows existing pair).
- `record_rate/` — `RecordRateModal.tsx` + `useRecordRate.ts` (FXR-020–029; create + edit modes — edit pre-fills pair/date/rate, FXR-052; in-flight FXR-027, success FXR-028, error FXR-029).
- `delete_rate/` — confirmation dialog + `useDeleteRate.ts` (FXR-053).
- `index.ts` — public re-exports.
- i18n: `src/i18n/locales/en/common.json` + `fr/common.json` — currency-rates view labels, form labels, error keys, snackbars, staleness/source strings.
- Route registration for the Currency Rates view (check `src/router.tsx` / route tree) + a nav entry (`src/features/shell/navItems.ts`).

**Cross-feature shortcut (no `account_details` → `currency` import, F26):**

- `src/lib/modalSearch.ts` — extend `ModalSearchParams` with `fxFrom?`, `fxTo?` (and reuse `modal` discriminant e.g. `modal=record-fx-rate`).
- `src/features/shell/CurrencyRateEditMount.tsx` (+ `.test.tsx`) — watches URL params, renders the currency `RecordRateModal` pre-filled; on close clears params. Mounted in `src/AppShell.tsx`. Mirrors `AssetEditModalMount` / `CashTransactionEditMount`.
- `src/features/account_details/.../HoldingRow.tsx` — the foreign-currency "—" placeholder becomes a clickable shortcut that calls `patchModalSearch(navigate, { modal: "record-fx-rate", fxFrom, fxTo })` (FXR-012). `[unit-test-needed]`.
- `src/features/account_details/account_details_view/useAccountDetails.ts` — add `CurrencyRateUpdated` to the event set that triggers re-fetch in the `subscribeToEvents` callback (today that callback re-fetches on `AssetPriceUpdated` / `TransactionUpdated` / `AssetUpdated`; FXR-036). `[unit-test-needed]`.
- `src/lib/store.ts` — add `"CurrencyRateUpdated"` to the `locallyHandledEvents` set (line ~128) so the global store does NOT trigger a global re-fetch on it (FXR-037 — locally-handled, mirrors how `AssetPriceUpdated`/`TransactionUpdated` are treated). `[unit-test-needed]`.
- `src/features/account_details/account_details_view/HoldingRow.tsx` — the foreign-currency "—" cell becomes the FXR-012 clickable shortcut AND renders the staleness label (FXR-090) when a converted value shows; the staleness string itself comes from the new `currency/shared/presenter.ts` (new-file presenter unit coverage), `HoldingRow` only displays it. `[unit-test-needed]`.

### Rules Coverage

| Rule    | Layer              | Task                                                                                                                                     | Notes                                              |
| ------- | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| FXR-010 | backend            | `CurrencyRate` direction semantics (domain)                                                                                              | PR1; ADR-009                                       |
| FXR-011 | backend            | identity-pair guard (`CurrencyPair::new`, `CurrencyRate::new`)                                                                           | PR1                                                |
| FXR-012 | frontend           | holding-row "—" shortcut via `CurrencyRateEditMount`                                                                                     | PR3; URL-modal mount (F26)                         |
| FXR-013 | backend            | `upsert_pair` idempotent ensure; called by fetch + record                                                                                | PR1 (repo) / PR2 (fetch wiring)                    |
| FXR-014 | backend            | no pair delete/archive; `delete_currency_rate` leaves pair                                                                               | PR1                                                |
| FXR-020 | frontend           | `validateRateForm` required fields                                                                                                       | PR3                                                |
| FXR-021 | frontend + backend | rate > 0 — `CurrencyRate::new` + `validateRateForm`                                                                                      | PR1 + PR3                                          |
| FXR-022 | frontend + backend | date ≤ today, ISO — domain + form                                                                                                        | PR1 + PR3                                          |
| FXR-023 | frontend + backend | ISO 4217 + distinct — domain (`InvalidCurrency`/`IdentityPair`) + form                                                                   | PR1 + PR3                                          |
| FXR-024 | backend            | f64 → i64 micros at `api.rs` boundary                                                                                                    | PR1; ADR-001                                       |
| FXR-025 | backend            | `upsert_rate` latest-write-wins                                                                                                          | PR1; ADR-012                                       |
| FXR-026 | backend            | publish `CurrencyRateUpdated` on upsert                                                                                                  | PR1                                                |
| FXR-027 | frontend           | in-flight spinner                                                                                                                        | PR3                                                |
| FXR-028 | frontend           | success snackbar + close                                                                                                                 | PR3                                                |
| FXR-029 | frontend           | inline error, form stays open                                                                                                            | PR3                                                |
| FXR-030 | backend            | `current_price × rate` (account_details orchestrator)                                                                                    | PR2; i128 (ACD-024)                                |
| FXR-031 | backend            | `unrealized_pnl` across currencies                                                                                                       | PR2; amends MKT-034                                |
| FXR-032 | backend            | `performance_pct` across currencies                                                                                                      | PR2; amends MKT-035                                |
| FXR-033 | backend            | `total_return_pct` across currencies                                                                                                     | PR2; amends DIV-071                                |
| FXR-034 | frontend + backend | no-usable-rate → `None`/"—"/0                                                                                                            | PR2 (BE) + PR3 (FE "—")                            |
| FXR-035 | backend            | `latest_rate_on_or_before` resolution                                                                                                    | PR1 (repo) / PR2 (use)                             |
| FXR-036 | frontend           | account_details subscribes `CurrencyRateUpdated`                                                                                         | PR3; `[unit-test-needed]`                          |
| FXR-037 | frontend + backend | event-bus enum (PR1) + `store.ts` `locallyHandledEvents` registration (PR3, `[unit-test-needed]`) + ARCHITECTURE registration (PR4 docs) | spans 3 PRs                                        |
| FXR-040 | backend            | `total_unrealized_pnl` includes converted                                                                                                | PR2; amends MKT-040                                |
| FXR-041 | backend            | `total_global_value` incl. converted (details + summary)                                                                                 | PR2; amends CSH-094/ACC-021                        |
| FXR-042 | backend            | account_performance period value uses rate                                                                                               | PR2; amends PRF-020/024                            |
| FXR-050 | backend            | `list_currency_rates(from,to)` date desc                                                                                                 | PR1                                                |
| FXR-051 | frontend           | Currency Rates view (pair list → drill-in)                                                                                               | PR3                                                |
| FXR-052 | frontend + backend | `update_currency_rate` + edit modal                                                                                                      | PR1 (BE) + PR3 (FE)                                |
| FXR-053 | frontend + backend | `delete_currency_rate` + confirm dialog                                                                                                  | PR1 (BE) + PR3 (FE)                                |
| FXR-054 | frontend + backend | `declare_currency_pair` + Add-pair form                                                                                                  | PR1 (BE) + PR3 (FE)                                |
| FXR-055 | frontend           | Add-pair form behaviour (disabled/idempotent-success)                                                                                    | PR3                                                |
| FXR-070 | backend            | provider chain Frankfurter→ECB                                                                                                           | PR2; ADR-009                                       |
| FXR-071 | backend            | fetch scope = persisted pairs; ensure-then-fetch                                                                                         | PR2                                                |
| FXR-072 | backend            | empty FX scope = no-op                                                                                                                   | PR2                                                |
| FXR-073 | backend            | per-pair failure skipped                                                                                                                 | PR2                                                |
| FXR-074 | backend            | publish `CurrencyRateUpdated` on fetch success                                                                                           | PR2                                                |
| FXR-075 | frontend + backend | piggyback on asset fetch tasks                                                                                                           | PR2 (BE wiring; FE already triggers price refresh) |
| FXR-076 | backend            | shares MKT-113 in-flight guard                                                                                                           | PR2                                                |
| FXR-080 | backend            | EUR cross-rate formula                                                                                                                   | PR2; i128                                          |
| FXR-081 | backend            | same-date legs                                                                                                                           | PR2                                                |
| FXR-082 | backend            | truncating-division rounding                                                                                                             | PR2; FXR-035-style                                 |
| FXR-083 | backend            | missing leg → skip                                                                                                                       | PR2                                                |
| FXR-090 | frontend           | staleness label                                                                                                                          | PR3                                                |
| FXR-091 | frontend           | no-rate → "—" (merged with no-price)                                                                                                     | PR3                                                |
| FXR-100 | backend            | `CurrencyRateSource` enum text discriminant                                                                                              | PR1                                                |
| FXR-101 | backend            | `source = Manual` on user writes                                                                                                         | PR1                                                |
| FXR-102 | backend            | `source = Frankfurter\|Ecb` on fetch writes                                                                                              | PR2                                                |

**`modified_functions` (for `test-writer-frontend`, PR 3):**

- `[account_details_view/HoldingRow.tsx:HoldingRow]` — FXR-012 (foreign-currency "—" cell becomes the clickable shortcut) + FXR-090 (renders the staleness label).
- `[account_details_view/useAccountDetails.ts:useAccountDetails]` — FXR-036 (add `CurrencyRateUpdated` to the `subscribeToEvents` re-fetch set).
- `[lib/store.ts:initStore]` — FXR-037 (add `"CurrencyRateUpdated"` to `locallyHandledEvents`; confirm the exact enclosing fn name at impl time — the `locallyHandledEvents` set near line 128).

These are existing-function edits (no contract command) that would otherwise get no FE unit coverage. FXR-090's staleness _string_ is produced by the new `currency/shared/presenter.ts` (new-file presenter unit coverage, not a modified function) — only its rendering in `HoldingRow` is a modified-function concern.

---

## 3. PR Plan

- **Strategy**: **5 PRs** (original PR 2 split into 2a + 2b on 2026-06-01 — see Backend phase B1/B2)
- **Estimate**: BC backend ~14 files / ~750 LOC · valuation lift ~7 files / ~450 LOC · provider fetch ~8 files / ~500 LOC · frontend ~22 files / ~950 LOC · E2E + closure ~6 files / ~300 LOC. Each PR is one story and under the ~1000-LOC churn target.

| PR  | Title                                                           | Scope                                                                                                                                                                                                                                                                                | Dependency                           | Branch                             |
| --- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------ | ---------------------------------- |
| 1   | `feat(currency): currency bounded context + manual rate CRUD`   | Migration + `context/currency/` (domain/infra/service/api/error) + 6 commands + `CurrencyRateUpdated` enum + specta registration + bindings. Manual CRUD only. ✅ merged as PR #63 (`b358b4e`).                                                                                      | none                                 | `feat/fx-rate` ✅                  |
| 2a  | `feat(currency): multi-currency valuation lift`                 | Reintroduce `latest_rate_on_or_before` + `CurrencyService::resolve_rate_micros` read port; `Arc`-ify `CurrencyService`; inject into `account_details` / `account_summary` / `account_performance`; lift the 4 currency guards. Works on manual rates. Terminates at B1 `/create-pr`. | rebase off `main` after PR 1 merges  | `feat/fx-rate-valuation` (current) |
| 2b  | `feat(currency): FX provider fetch (Frankfurter + ECB)`         | `RateProvider` trait + Frankfurter/ECB clients + EUR cross-rate; `fetch_rates_for_pairs`; piggyback into `asset_price_fetch`; `reviewer-security` on the new HTTP client. Terminates at B2 `/create-pr`.                                                                             | rebase off `main` after PR 2a merges | `feat/fx-rate-fetch`               |
| 3   | `feat(currency): Currency Rates view + holding-row FX shortcut` | `src/features/currency/` (gateway/hooks/view/modals/presenter) + i18n + route/nav + `shell/CurrencyRateEditMount` + `modalSearch` params + account_details shortcut/staleness/subscription. Terminates at the Frontend-phase `/create-pr`.                                           | rebase off `main` after PR 2b merges | `feat/fx-rate-fe`                  |
| 4   | `test(currency): FX E2E + spec closure`                         | E2E scenarios + `reviewer-e2e` + docs (ARCHITECTURE, account-contract subscribed event, ubiquitous-language, todo, roadmap) + `spec-checker` HARD GATE. Terminates at the Closure-phase `/create-pr`.                                                                                | rebase off `main` after PR 3 merges  | `feat/fx-rate-e2e`                 |
