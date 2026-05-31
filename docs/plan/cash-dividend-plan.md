# Implementation Plan — Cash Dividend (DIV)

> Source spec: `docs/spec/cash-dividend.md` · Contract: `docs/contracts/account-contract.md`
> Bounded context: `account` (+ `use_cases/holding_transaction/` orchestrator, `use_cases/account_details/` read model).
> Layout: backend `src-tauri/src`, frontend `src` (ARCHITECTURE.md present).

A cash dividend is a new `TransactionType::Dividend`: cash income attributed to the **paying asset** that credits the account's Cash Holding (mirroring Sell — CSH-050/012), leaves the paying asset's holding untouched (DIV-023/024), writes **no** `AssetPrice` (DIV-027), and is kept out of realized P&L. v1 also folds dividends into per-asset performance (DIV-070–073) and routes entry through a consolidated header "Add" menu (DIV-010/012).

---

## 1. Workflow TaskList

### Setup

- [ ] 📖 Read spec: `docs/spec/cash-dividend.md`
- [ ] 📖 Read contract: `docs/contracts/account-contract.md` (`record_dividend`, `DividendDTO`, `TransactionType::Dividend`, `HoldingDetail.dividends_received`/`.total_return_pct`, `AccountDetailsResponse.total_dividends_received`)
- [ ] 📖 Read constraining ADRs: `docs/adr/001-use-i64-for-monetary-amounts.md`, `docs/adr/006-unit-of-work.md` (atomic credit + replay), `docs/adr/003-cross-context-use-case-orchestration.md` + `docs/adr/004-use-cases-inject-services-not-repositories.md` (orchestrator shape)
- [ ] 📖 Read conventions: `ARCHITECTURE.md`, `docs/test_convention.md`; BE — `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/backend-patterns.md`; FE — `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/frontend-visual-proof.md`; reference the CSH spec + the existing deposit/sell flows as the implementation template

### Backend phase → **PR 1**

- [ ] 🗄️ Migration: `src-tauri/migrations/{ts}_add_dividend_transaction_type.sql` — **doc-only no-op** (`SELECT 1;`) mirroring `202605060001_add_cash_transaction_types.sql` (column is free-text TEXT, no CHECK). Then `just db-migrate` + `just prepare-sqlx`.
- [ ] ✍️ `test-writer-backend` → stubs for `record_dividend` + the read-model fields from the contract; confirm red
- [ ] 🏗️ Backend implementation (minimal — make failing tests pass; no defensive/anticipatory code)
- [ ] 🔍 `reviewer-backend` + `reviewer-arch` + `reviewer-sql` (parallel) → `/review-triage` → apply Follow-ups; halt for user on (b)/(c)
- [ ] 🔗 `just generate-types` → `src/bindings.ts`
- [ ] 🔧 `npx tsc --noEmit` → fix TS errors from new bindings only + add `Dividend`/field to existing FE `Transaction`/`HoldingDetail`/`AccountDetailsResponse` test fixtures (compile-fix; no UI)
- [ ] 🧹 `just format`
- [ ] 💾 `/smart-commit`: `feat(account): record_dividend command + Dividend transaction type` (bundles spec/contract/plan/UL/spec-index docs)
- [ ] 🔀 `/create-pr` (PR 1 = BE). After merge, branch FE off updated `main`.

### Frontend phase → **PR 2**

- [ ] ✍️ `test-writer-frontend` → stubs (gateway unit, presenter unit, modal + header-menu + holding-row RTL); confirm red. `modified_functions`: `presenter.ts:toHoldingRow` (DIV-072 fields), `AccountDetailsView.tsx` header (DIV-012)
- [ ] 💻 Frontend implementation (minimal)
- [ ] 📸 `/visual-proof` — DividendTransactionModal + the header "Add" menu + HoldingRow (dividends/total-return); light + dark; stage screenshots
- [ ] 🔍 `reviewer-frontend` + `reviewer-arch` (parallel) → `/review-triage` → apply Follow-ups
- [ ] 🧹 `just format`
- [ ] 💾 `/smart-commit`: `feat(account-details): dividend modal + consolidated header Add menu`

### Closure (rides in PR 2)

- [ ] ✍️ `test-writer-e2e` → record-dividend scenario (header menu → modal → cash credited, holding row dividends/total-return updates); `/setup-e2e` already done
- [ ] ▶️ `npm run test:e2e` → green (main agent triages)
- [ ] 🔍 `reviewer-e2e` (+ `reviewer-security` — new `record_dividend` Tauri command) (parallel) → `/review-triage`
- [ ] 📚 Docs: close in `docs/todo.md`; flip `docs/roadmap.md` Phase 4 "Dividend" → shipped; `ARCHITECTURE.md` only if a new module/path is introduced (none expected — reuses holding_transaction)
- [ ] ✅ `spec-checker` [HARD GATE — every DIV rule + `record_dividend` covered]
- [ ] 🧹 `just format`
- [ ] 💾 `/smart-commit`: `test(account): cash-dividend E2E + closure`
- [ ] 🔀 `/create-pr` (PR 2 = FE + E2E + closure)

---

## 2. Detailed Implementation Plan

### Migrations

- `src-tauri/migrations/{ts}_add_dividend_transaction_type.sql` — doc-only no-op recording the new `Dividend` discriminant (TEXT column, no schema change). Run `just db-migrate` then `just prepare-sqlx`.

### Backend (`account` BC + `use_cases/holding_transaction/`)

- **`context/account/domain/transaction.rs`** — add `TransactionType::Dividend`. Add a `new_dividend(...)` factory (mirror `new_deposit`): `asset_id` = paying asset; `total_amount` = `amount × exchange_rate` (account ccy); `unit_price`/`quantity` follow a fixed convention; `realized_pnl = None` (DIV-023/024). Inline tests.
- **`context/account/domain/account.rs`** — extend `apply_transaction` / chronological replay: `Dividend` → paying-asset holding qty delta **0**; cash holding **+= total_amount** (DIV-023). It joins the cash-affecting set used by edit/delete replay (DIV-040/041). Inline tests (dividend leaves position unchanged; cash credited; replay after delete can underflow → guard).
- **`use_cases/holding_transaction/orchestrator.rs`** — `record_dividend(DividendDTO)`: validate (DIV-011/021/022), resolve paying asset, eligibility (active non-cash holding, not Cash Asset), lazy-`ensure_cash_asset` + credit within one UoW (ADR-006), persist, publish `TransactionUpdated` (DIV-026). Mirror the deposit/sell path.
- **`use_cases/holding_transaction/api.rs`** — `#[tauri::command] record_dividend`. Register in `core/specta_builder.rs`.
- **`use_cases/holding_transaction/error.rs`** (+ `context/account/domain/transaction_error.rs` as needed) — error variants per contract: `AccountNotFound`, `AssetNotFound`, `AssetNotHeld`, `DividendOnCashAsset`, `AmountNotPositive`, `InvalidDate`, `DateInFuture`, `DateTooOld`, `ExchangeRateNotPositive`, `DatabaseError`. No `InsufficientCash` (credit-only). `correct_transaction`/`cancel_transaction` already carry it for dividend edit/delete (DIV-040/041).
- **`use_cases/account_details/orchestrator.rs`** — compute `HoldingDetail.dividends_received` (Σ Dividend `total_amount` per `(account, asset)`), `total_return_pct` (`(unrealized_pnl + dividends_received) × 100 / cost_basis`, i128 intermediates, `None` per MKT-034/035 — DIV-071), and `AccountDetailsResponse.total_dividends_received` (Σ across all dividend tx — DIV-073). Mirror MKT unrealized/performance computation.

### Frontend (`features/account_details/`)

- **`gateway.ts`** — `recordDividend(dto)` → `commands.recordDividend` (the only file allowed to call `commands.*`).
- **`dividend_transaction/DividendTransactionModal.tsx` + `useDividendTransaction.ts`** — new sub-feature mirroring `deposit_transaction/`. Asset selector (active non-cash holdings), date, amount (asset ccy), exchange rate when currencies differ (mirror buy/sell), note; validation (DIV-021), in-flight/success/error (DIV-025), error pipeline via a local `dividendErrorToI18n` presenter (F27; reuse the AccountCrud/HoldingTransaction error shape).
- **`account_details_view/AccountDetailsView.tsx`** — replace the standalone Deposit / Withdraw / "Add a position" header buttons with a single **"Add ▾" dropdown** (DIV-012) whose items are Deposit, Withdraw (existing visibility condition), Add a position, Record dividend. ⚠️ This is a refactor of shipped header wiring — keep each modal's existing open/close handlers; only the trigger surface changes.
- **`account_details_view/HoldingRow.tsx`** + **`shared/presenter.ts`** (`toHoldingRow`, `HoldingRowViewModel`) — surface `dividends_received` + `total_return_pct` alongside Performance % (DIV-072); `dividends_received` always shown, `total_return_pct` → `—` when null.
- **Account Details header total** — display `total_dividends_received` alongside Global Value / cost basis / realized P&L (DIV-073).
- **i18n** — `src/i18n/locales/{en,fr}/common.json`: dividend modal labels, the "Add" menu items, snackbar success, error keys (reuse existing `error.*` where they fit).

### Rules Coverage

| Rule    | Layer              | Task                                                                 | Notes                                                                      |
| ------- | ------------------ | -------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| DIV-010 | frontend           | "Record dividend" item in header "Add" menu → modal                  | entry point                                                                |
| DIV-011 | backend            | `record_dividend` eligibility guards                                 | `AssetNotHeld`/`DividendOnCashAsset`                                       |
| DIV-012 | frontend           | `AccountDetailsView.tsx` header "Add ▾" dropdown                     | supersedes CSH-019/TRX-055 buttons; refactor flagged; `[unit-test-needed]` |
| DIV-020 | frontend           | `DividendTransactionModal` fields (asset selector)                   | mirror deposit modal                                                       |
| DIV-021 | frontend + backend | inline + backend validation                                          | amount>0, date, rate>0                                                     |
| DIV-022 | frontend + backend | currency conversion via `exchange_rate`                              | TRX-021 mechanism                                                          |
| DIV-023 | backend            | orchestrator UoW: credit cash, persist Dividend                      | ADR-006; CSH-050/012                                                       |
| DIV-024 | backend            | `apply_transaction` Dividend: qty delta 0, no realized P&L           | holding untouched                                                          |
| DIV-027 | backend            | orchestrator writes no `AssetPrice`                                  | assert in test                                                             |
| DIV-025 | frontend           | `useDividendTransaction` in-flight/success/error                     | snackbar + inline                                                          |
| DIV-026 | backend + frontend | publish `TransactionUpdated`; ACD re-fetch                           | existing event                                                             |
| DIV-040 | backend + frontend | reuse `correct_transaction` (replay; asset_id immutable)             | event published                                                            |
| DIV-041 | backend + frontend | reuse `cancel_transaction` (replay; `InsufficientCash` on underflow) | event published                                                            |
| DIV-050 | frontend           | transaction list Type "Dividend", P&L `—`                            | TXL cross-amend                                                            |
| DIV-051 | backend            | dividend cash already in `total_global_value`                        | no extra compute                                                           |
| DIV-070 | backend            | `account_details` `dividends_received`                               | Σ per (account,asset)                                                      |
| DIV-071 | backend            | `account_details` `total_return_pct`                                 | MKT-034/035 null conds                                                     |
| DIV-072 | frontend           | `HoldingRow`/`presenter` display                                     | always show dividends; total-return `—` when null; `[unit-test-needed]`    |
| DIV-073 | backend + frontend | `total_dividends_received` + header display                          | Σ all dividend tx                                                          |

---

## 3. PR Plan

- **Strategy**: `2 PRs` (BE → FE+E2E)
- **Estimate**: BE ~7 files / ~300 LOC; FE ~9 files / ~350 LOC (incl. the header "Add"-menu consolidation refactor). Neither exceeds the ~20-file/~500-LOC split threshold, but the layers are cleanly separable and the FE carries a header refactor → 2 PRs for reviewability.
- **PR 1 — `feat(account): cash-dividend backend`**
  - Scope: spec/contract/plan/UL/spec-index docs + migration + Rust (domain, orchestrator, api, account_details read-model) + `record_dividend` Tauri command + bindings + FE fixture compile-fixes. Terminates at the Backend-phase `/create-pr`.
  - Dependency: none (mergeable alone; bindings present but unused).
  - Branch suffix: `feat/cash-dividend` (current branch).
- **PR 2 — `feat(account-details): cash-dividend UI + consolidated Add menu`**
  - Scope: gateway, DividendTransactionModal + hook, header "Add ▾" dropdown (consolidating Deposit/Withdraw/Open-balance), HoldingRow + presenter display, account-header total, i18n, visual proof, E2E, docs closure, spec-checker. Terminates at the Closure `/create-pr`.
  - Dependency: rebase off `main` after PR 1 merges (needs the new bindings).
  - Branch suffix: `feat/cash-dividend-ui`.
