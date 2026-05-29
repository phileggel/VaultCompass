# Implementation Plan — Account Performance (PRF)

> Spec: [`docs/spec/account-performance.md`](../spec/account-performance.md) · Contract: [`docs/contracts/account-contract.md`](../contracts/account-contract.md) · Decision: [ADR-013](../adr/013-recompute-account-performance-on-read.md)
>
> Layout: backend `src-tauri/src`, frontend `src` (from ARCHITECTURE.md). No database migration — values are recomputed on read (ADR-013, PRF-026).

---

## 1. Workflow TaskList

### Setup

- [ ] 📖 Read spec: `docs/spec/account-performance.md`
- [ ] 📖 Read contract: `docs/contracts/account-contract.md` (the `get_account_performance` command anchors the backend tests)
- [ ] 📖 Read constraining ADRs: `docs/adr/013-recompute-account-performance-on-read.md` (recompute-on-read), `docs/adr/001-use-i64-for-monetary-amounts.md` (i64/i128), `docs/adr/003-cross-context-use-case-orchestration.md` + `docs/adr/004-use-cases-inject-services-not-repositories.md` (use-case orchestration)
- [ ] 📖 Read conventions: `ARCHITECTURE.md`, `docs/test_convention.md`; BE → `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/backend-patterns.md`; FE → `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/frontend-visual-proof.md`

### Backend phase — PR #1

- [ ] ✍️ Backend test stubs (`test-writer-backend` from the contract — all stubs written, red confirmed)
- [ ] 🏗️ Backend implementation (minimal — make failing tests pass, green confirmed; no defensive code, no anticipation of future rules)
- [ ] 🗄️ `just prepare-sqlx` — only if the new `get_all_transactions_for_account` query is compile-time-checked (`query!`/`query_as!`); refreshes the `.sqlx` offline cache so `SQLX_OFFLINE=true` builds pass. No migration (ADR-013), so no `just migrate`.
- [ ] 🔍 Backend review (`reviewer-backend` + `reviewer-arch` in parallel → `/review-triage` → apply Follow-ups; halt for user on any (b)/(c)) — _no `reviewer-sql` (no migration)_
- [ ] 🔗 Type synchronization (`just generate-types` → `src/bindings.ts`)
- [ ] 🔧 `npx tsc --noEmit` → fix TS errors from new bindings only (no UI work)
- [ ] 🧹 `just format`
- [ ] 💾 Commit via `/smart-commit` — suggested: `feat(account): account performance backend (PRF)`
- [ ] 🔀 `/create-pr` (PR #1, BE). After merge, branch PR #2 off updated `main`.

### Frontend phase — PR #2

- [ ] ✍️ Frontend test stubs (`test-writer-frontend` from the contract — red confirmed; `modified_functions`: none, see §Modified-function coverage)
- [ ] 💻 Frontend implementation (minimal — make failing tests pass, green confirmed)
- [ ] 📸 Visual proof (`/visual-proof` — capture loading / empty / error / month-view / year-view in light + dark; stage screenshots)
- [ ] 🔍 Frontend review (`reviewer-frontend` + `reviewer-arch` in parallel → `/review-triage` → apply Follow-ups; halt for user on any (b)/(c))
- [ ] 🧹 `just format`
- [ ] 💾 Commit via `/smart-commit` — suggested: `feat(account-performance): performance page UI (PRF)`
- [ ] 🔀 `/create-pr` (PR #2, FE). After merge, branch PR #3 off updated `main`.

### Closure — PR #3

- [ ] ✍️ E2E scenarios (`test-writer-e2e` — critical path: open page, toggle month/year, change year, empty/error states; run `/setup-e2e` first if needed)
- [ ] ▶️ Run E2E suite (`just test-e2e-headless` → green; main agent triages failures)
- [ ] 🔍 Cross-cutting review (`reviewer-e2e` on the new E2E files → `/review-triage`) — _no `reviewer-security` (no new command surface beyond a read; no capability change), no `reviewer-infra` (no config/script/hook change) unless those files end up touched_
- [ ] 📚 Documentation update (`docs/todo.md` — none open for PRF; `ARCHITECTURE.md` — register `use_cases/account_performance/` and the `/accounts/:id/performance` route + that the page subscribes to `TransactionUpdated`/`AssetPriceUpdated`/`AccountUpdated`)
- [ ] ✅ Spec check (`spec-checker`) [HARD GATE — every PRF-NNN rule + the `get_account_performance` command covered]
- [ ] 🧹 `just format`
- [ ] 💾 Commit via `/smart-commit` — suggested: `test(account-performance): E2E + closure (PRF)`
- [ ] 🔀 `/create-pr` (PR #3, E2E + closure)

---

## 2. Detailed Implementation Plan

### Migrations

None. Per ADR-013 / PRF-026, period values are recomputed on read; nothing is persisted. No `just migrate`. The new `get_all_transactions_for_account` is a read query against the existing `transactions` table (no schema delta); if implemented as a compile-time-checked `query!`/`query_as!`, run `just prepare-sqlx` once after backend implementation to refresh the `.sqlx` offline cache (per the `SQLX_OFFLINE=true` default).

### Backend

**New read on the account context** (needed for as-of-date replay — service today only fetches transactions per `(account, asset)`):

- `src-tauri/src/context/account/domain/` (transaction repository trait) — add `get_all_for_account(&self, account_id: &str) -> Result<Vec<Transaction>>` returning every transaction for the account across all assets (incl. cash), ordered chronologically by `(date, created_at)`.
- `src-tauri/src/context/account/repository/transaction.rs` — implement the SQLite query for the new trait method (mirror the row mapping of `get_transactions`).
- `src-tauri/src/context/account/service.rs` — add `get_all_transactions_for_account(&self, account_id: &str) -> StdResult<Vec<Transaction>, AccountApplicationError>` delegating to the repository.

**New use case** `src-tauri/src/use_cases/account_performance/` (mirror `account_summary/`):

- `orchestrator.rs` — `AccountPerformanceUseCase { account_service: Arc<AccountService>, asset_service: Arc<AssetService> }` (ADR-003/004) with `get_account_performance(&self, account_id: &str) -> StdResult<AccountPerformanceResponse, AccountApplicationError>`. Defines `AccountPerformanceResponse`, `PerformancePeriod`, `PerformanceMetric` (`#[derive(Debug, Serialize, Clone, Type)]`). Logic:
  - Load account via `AccountService::get_by_id`; `None` → `AccountNotFound { account_id }` (PRF-016).
  - `month_view_available` from `update_frequency ∈ {Automatic, ManualDay, ManualWeek}` (PRF-013); `currency`, `account_name` from the account.
  - Load all transactions via the new `get_all_transactions_for_account` (PRF-021); empty → empty `yearly`/`monthly`, return early (PRF-043).
  - Per held asset, load price history via `AssetService::get_asset_prices` (PRF-022). Wrap load failures into the application error (PRF-027).
  - Derive data span from earliest transaction date → current period (PRF-040); current period end = today (PRF-020).
  - For each period (years always; months only when `month_view_available`): replay transactions ≤ period end → units + cash (PRF-021, PRF-023); value non-cash holdings at most-recent price ≤ period end else 0, foreign-currency → 0 (PRF-022, PRF-024); `end_value` = cash + Σ (PRF-020); net external flow = Σ Deposit − Σ Withdrawal + Σ OpeningBalance-cost, account currency (PRF-030); gain (PRF-031) + Simple Dietz pct `gain × 100_000_000 / denominator`, i128, numerator-scaled-first, truncating (PRF-032, PRF-025) for period-over-period (PRF-033), year-to-date (PRF-034, omitted-as-None for year rows per PRF-037), since-inception vs net invested (PRF-035). Absent baseline → `None` (PRF-042).
  - Order rows most-recent first (PRF-041).
- `api.rs` — `#[tauri::command] #[specta::specta] pub async fn get_account_performance(account_id: String, state: State<'_, AccountPerformanceUseCase>) -> Result<AccountPerformanceResponse, AccountApplicationError>`.
- `mod.rs` — re-export the use case + command (mirror `account_summary/mod.rs`).

**Wiring:**

- `src-tauri/src/use_cases/mod.rs` — add `pub mod account_performance;`.
- `src-tauri/src/core/specta_builder.rs` — register `account_performance::get_account_performance` in the command list (next to `account_summary::get_account_summaries`, ~line 97).
- `src-tauri/src/lib.rs` (~line 168, where `AccountSummaryUseCase::new` is built and `.manage()`d) — construct and manage `AccountPerformanceUseCase`.

### Frontend

**New feature** `src/features/account_performance/` (gold F0 layout):

- `gateway.ts` — only file calling `commands.getAccountPerformance(accountId)`; passes the typed `Result` through per F27.
- `shared/presenter.ts` — map `AccountPerformanceResponse` → view model: micro→display formatting (currency + micro-percent), sign colour for gains (PRF-036), "—" for absent metrics/pct (PRF-042, PRF-032), month/year row labels.
- `account_performance_view/AccountPerformancePage.tsx` — route component; header (account name, view-mode toggle PRF-011/013, year selector PRF-015, back action PRF-053); table (PRF-036 columns; YTD column hidden in year view per PRF-037); loading/empty/error states (PRF-050/051/052).
- `account_performance_view/useAccountPerformance.ts` — hook: calls gateway, holds view-mode + selected-year state, default month/current-year when eligible else year (PRF-014), slices `monthly` to the selected year (PRF-015), subscribes to `TransactionUpdated`/`AssetPriceUpdated`/`AccountUpdated` re-fetch (PRF-060).
- `index.ts` — barrel export of the route component.
- i18n message keys for labels/states (`docs/i18n-rules.md`); a11y labels per F24.

**Wiring / modifications:**

- `src/router.tsx` — add `accountPerformanceRoute` at `/accounts/$accountId/performance` (mirror `accountDetailsRoute` at line 51) and register in the route tree (~line 102).
- Account Details header (under `src/features/account_details/account_details_view/`) — add the "Performance" action navigating to the route (PRF-010), using a stable `id` (F25).

### Rules Coverage

| Rule    | Layer              | Task                                                                         | Notes                                  |
| ------- | ------------------ | ---------------------------------------------------------------------------- | -------------------------------------- |
| PRF-010 | frontend           | Account Details header "Performance" action → route                          | F25 stable id                          |
| PRF-011 | frontend           | `useAccountPerformance` view-mode state + toggle                             | canonical month/year                   |
| PRF-012 | frontend + backend | BE always builds `yearly`; FE always offers year view                        |                                        |
| PRF-013 | frontend + backend | `month_view_available` from `update_frequency`; FE gates toggle              |                                        |
| PRF-014 | frontend           | default view mode in hook                                                    |                                        |
| PRF-015 | frontend           | year selector; slice `monthly` to year                                       | current year always selectable         |
| PRF-016 | frontend + backend | `get_by_id` None → `AccountNotFound`                                         | contract error                         |
| PRF-020 | backend            | `end_value` = Global Value at period end; current period end = today         | CSH-094                                |
| PRF-021 | backend            | replay transactions ≤ period end → units + cash                              | new `get_all_transactions_for_account` |
| PRF-022 | backend            | value at most-recent price ≤ period end else 0                               | `get_asset_prices`                     |
| PRF-023 | backend            | cash balance at period end at face value                                     |                                        |
| PRF-024 | backend            | foreign-currency non-cash → 0                                                | ADR-001; FX deferred                   |
| PRF-025 | backend            | i128 intermediates → i64 micros                                              | ADR-001                                |
| PRF-026 | backend            | recompute on read; nothing persisted                                         | ADR-013                                |
| PRF-027 | backend            | tx / price load failure → `DatabaseError`                                    | error-model                            |
| PRF-030 | backend            | net external flow = Deposit − Withdrawal + OpeningBalance cost (account ccy) |                                        |
| PRF-031 | backend            | gain = end − start − net flow                                                |                                        |
| PRF-032 | backend            | Dietz pct `gain × 100_000_000 / denom`, i128, numerator-first                |                                        |
| PRF-033 | backend            | period-over-period metric                                                    |                                        |
| PRF-034 | backend            | year-to-date metric                                                          | None for year rows                     |
| PRF-035 | backend            | since-inception vs net invested                                              |                                        |
| PRF-036 | frontend           | gain+pct pairs, sign colour, "—"                                             | presenter                              |
| PRF-037 | frontend           | omit YTD column in year view                                                 |                                        |
| PRF-040 | backend            | data span first-tx → current period; empty period → 0                        |                                        |
| PRF-041 | frontend           | rows most-recent first                                                       | (BE also sorts)                        |
| PRF-042 | backend + frontend | absent baseline → None → "—"                                                 |                                        |
| PRF-043 | backend            | no transactions → empty result                                               |                                        |
| PRF-050 | frontend           | loading skeleton                                                             |                                        |
| PRF-051 | frontend           | empty state + Add Transaction affordance                                     | ACD-035                                |
| PRF-052 | frontend           | error state + Retry                                                          | ACD-038                                |
| PRF-053 | frontend           | back navigation to Account Details                                           |                                        |
| PRF-060 | frontend           | re-fetch on TransactionUpdated/AssetPriceUpdated/AccountUpdated              | mirrors ACD-039/040, MKT-036           |

### Modified-function coverage

None. PRF-010 adds a navigation action to the existing Account Details header (presentational, covered by component integration tests, not a logic-function unit test). All performance logic lives in new files (`useAccountPerformance.ts`, `shared/presenter.ts`, the orchestrator) — covered by `test-writer-frontend` / `test-writer-backend` from the contract. `modified_functions` list passed to `test-writer-frontend`: empty.

---

## 3. PR Plan

- **Strategy**: `3 PRs`
- **Estimate**: BE ~7 files / ~480 LOC · FE ~9 files / ~400 LOC · E2E+closure ~3 files / ~150 LOC. Backend isolates financial math for focused review; FE is a substantial new page; per the project's per-layer split guidance for a feature nearing ~500 LOC/layer.

**PR #1 — `feat(account): account performance backend (PRF)`**

- Scope: spec + contract + ADR-013 (already on branch) + new `use_cases/account_performance/` + `get_all_transactions_for_account` read + `specta_builder` + `lib.rs` wiring + `src/bindings.ts` regen. Terminates at the Backend-phase `/create-pr`.
- Dependency: none (first PR off `feat/account-performance`).
- Branch suffix: `feat/account-performance-be`.

**PR #2 — `feat(account-performance): performance page UI (PRF)`**

- Scope: `src/features/account_performance/` (gateway/hook/presenter/components/index), `router.tsx` route, Account Details "Performance" action, i18n, visual proof. Terminates at the Frontend-phase `/create-pr`.
- Dependency: rebase off `main` after PR #1 merges (consumes the regenerated bindings).
- Branch suffix: `feat/account-performance-fe`.

**PR #3 — `test(account-performance): E2E + closure (PRF)`**

- Scope: E2E scenarios, `ARCHITECTURE.md` update, `spec-checker` closure. Terminates at the Closure `/create-pr`.
- Dependency: rebase off `main` after PR #2 merges.
- Branch suffix: `feat/account-performance-e2e`.
