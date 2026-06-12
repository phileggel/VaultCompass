# Implementation Plan — Free Share Distribution (FSD)

> Spec: `docs/spec/free-share-distribution.md` (FSD-010..070, 16 rules)
> Contract: `docs/contracts/account-contract.md` (`record_free_shares` + `FreeSharesDTO` + `TransactionType::FreeShares`)
> Mirrors the DIV (cash-dividend) implementation shape throughout — same BC, same orchestration path, same modal/TXL surfaces.

---

## 1. Workflow TaskList

**Setup**

- [ ] 📖 Read spec: `docs/spec/free-share-distribution.md`
- [ ] 📖 Read contract: `docs/contracts/account-contract.md` (§ Free Share Distribution + `Transaction` packing convention)
- [ ] 📖 Read constraining ADRs: `docs/adr/001-use-i64-for-monetary-amounts.md` (micro-units), `docs/adr/002-replace-asset-account-with-holding.md` (Holding model), `docs/adr/006-unit-of-work.md` (atomic record), `docs/adr/013-recompute-account-performance-on-read.md` (PRF replay)
- [ ] 📖 Read conventions: `ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/backend-patterns.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/frontend-visual-proof.md`, `docs/test_convention.md`

**Backend phase** _(PR 1)_

- [ ] 🗄️ No migration — `transaction_type` persists as TEXT (strum); a new enum variant needs no schema change (DIV precedent). No `just migrate` / `just prepare-sqlx` needed.
- [ ] ✍️ Backend test stubs (`test-writer-backend` from the contract's `record_free_shares` row + FSD-022/023/027/028/070 replay rules — red confirmed)
- [ ] 🏗️ Backend Implementation (minimal — implement only what makes the failing tests pass; no defensive code, no anticipation of future rules; green confirmed)
- [ ] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` + **`reviewer-security`** [new `#[tauri::command]`] in parallel → `/review-triage` → apply Follow-ups) — _no `reviewer-sql` (no migration)_
- [ ] 🔗 Type Synchronization (`just generate-types` → `src/bindings.ts`)
- [ ] 🔧 Run `npx tsc --noEmit` → fix TS errors from new bindings only (no UI work)
- [ ] 🧹 `just format`
- [ ] 💾 Commit: `feat(account): record free-share distributions` via `/smart-commit` [HARD GATE]
- [ ] 🔀 `/create-pr` (PR 1 — backend). After merge, branch the FE phase off updated `main`.

**Frontend phase** _(PR 2, part 1)_

- [ ] ✍️ Frontend test stubs (`test-writer-frontend` from contract; pass the `modified_functions` list from § Rules Coverage; red confirmed)
- [ ] 💻 Frontend Implementation (minimal — implement only what makes the failing tests pass; green confirmed)
- [ ] 📸 Visual proof (`/visual-proof` — FreeSharesModal: idle / filled / error / edit-mode; AccountDetailsView Record menu open; TransactionListPage with a free-shares row; light + dark)
- [ ] 🔍 Frontend Review (`reviewer-frontend` → `/review-triage` → apply Follow-ups)
- [ ] 🧹 `just format`
- [ ] 💾 Commit: `feat(account): free-shares modal + transaction-list rendering` via `/smart-commit` [HARD GATE]
- [ ] _(no `/create-pr` here — PR 2 continues with closure)_

**Closure** _(PR 2, part 2)_

- [ ] ✍️ E2E scenarios (`test-writer-e2e` — record → holding reflects → delete restores; see § E2E)
- [ ] ▶️ Run E2E suite (`just test-e2e-headless` → green; main agent triages failures)
- [ ] 🔍 Cross-cutting Review (`reviewer-e2e` [E2E files] → `/review-triage`) — _`reviewer-security` already ran in the BE phase; `reviewer-infra` only if config/scripts change_
- [ ] 📚 Documentation Update — **housekeeping bundle**: `docs/roadmap.md` (Phase 4 table: add "Free distribution ✅ Done" row), tick this plan's checkboxes, tick/close the stale shipped plans (`docs/plan/api-key-management-plan.md`, `docs/plan/fx-rate-plan.md` — flagged by `/whats-next` 2026-06-11), `docs/spec-index.md` FSD status stays `active`
- [ ] ✅ Spec check (`spec-checker`) [HARD GATE — all 16 FSD rules + `record_free_shares` covered; halt on any gap]
- [ ] 🧹 `just format`
- [ ] 💾 Commit: `test(account): FSD E2E + closure` via `/smart-commit` [HARD GATE]
- [ ] 🔀 `/create-pr` (PR 2 — frontend + E2E + closure)

---

## 2. Detailed Implementation Plan

### Migrations

None. `transaction_type` is stored as TEXT and deserialized via strum — adding the `FreeShares` variant requires no schema change (same as `Dividend` in DIV).

### Backend

| #   | File                                                          | Task                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1  | `src-tauri/src/context/account/domain/transaction.rs`         | Add `TransactionType::FreeShares` variant. Add factory `Transaction::free_shares(account_id, asset_id, date, quantity, note)` mirroring the dividend factory (~line 250): packs `unit_price = 0`, `exchange_rate = 1_000_000`, `fees = 0`, `total_amount = 0`, `realized_pnl = None` per the contract convention (FSD-022/023, ADR-001 micros). Inline tests.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| B2  | `src-tauri/src/context/account/domain/account.rs`             | (a) Aggregate-root `apply_free_shares` — **NOT a copy of `apply_dividend` (~line 637), which is cash-only/no-op on holdings (DIV-024)**: FreeShares is the opposite shape — it mutates the holding (`quantity +=`, cost basis unchanged → VWAP dilutes) and touches no cash; the quantity-effect mirror is the `recalculate_holding` replay (~807–862) (FSD-022/023). BE tests must assert the holding-quantity change. (b) Holding-replay match: `FreeShares` arm adds quantity at zero cost (FSD-027/028). (c) `replay_cash_holding` (~line 673): `FreeShares` has **no cash effect** — exclude from cash-affecting arms (FSD-022d). (d) Correction-validation match (~line 357): `FreeShares` editable fields are date/quantity/note, `quantity > 0` enforced (FSD-040, FSD-021). Inline tests incl. the FSD-028 record → delete → compare invariant. |
| B3  | `src-tauri/src/context/account/service.rs`                    | `record_free_shares` mirroring `record_dividend` (~line 471): UoW, persists via aggregate, publishes `TransactionUpdated` (FSD-022/026). Inline tests.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| B4  | `src-tauri/src/use_cases/holding_transaction/error.rs`        | `FreeSharesApplicationError` (`AssetNotFound`, `AssetNotHeld`, `FreeSharesOnCashAsset`) + `FreeSharesError` composite per `docs/error-model.md`, mirroring `DividendError` (FSD-011).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B5  | `src-tauri/src/use_cases/holding_transaction/orchestrator.rs` | `record_free_shares` mirroring the dividend orchestration (~lines 215–280): asset existence + cash-asset guard (asset BC read), active-holding guard, delegate to account service (FSD-011, ADR-004).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| B6  | `src-tauri/src/use_cases/holding_transaction/api.rs`          | `#[tauri::command] record_free_shares(dto: FreeSharesDTO) -> Result<Transaction, FreeSharesError>` (contract row).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| B7  | `src-tauri/src/core/specta_builder.rs`                        | Register `record_free_shares`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| B8  | `src-tauri/src/use_cases/account_performance/orchestrator.rs` | Extend the **four** `TransactionType` match blocks (~lines 546/555/622/663): `FreeShares` = no cash flow, no external flow; units enter the as-of-date replay (FSD-070). The compiler's exhaustiveness check finds every site.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

### Frontend

| #   | File                                                                                                            | Task                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | `src/features/account_details/gateway.ts` (+ `gateway.test.ts`)                                                 | `recordFreeShares(dto)` — Result pass-through per F27; positional/args shape per fresh `bindings.ts`.                                                                                                                                                                                                                                                                                  |
| F2  | `src/features/account_details/free_shares/FreeSharesModal.tsx` + `useFreeSharesTransaction.ts` (+ 2 test files) | New sub-feature mirroring `dividend_transaction/`: asset selector (active non-cash holdings), `DateField` (default today), quantity field, note; inline validation, submit spinner, snackbar "Free shares recorded" (FSD-020/021/025). Supports **edit mode** for FSD-040 (date/quantity/note editable, asset locked), like the cash modals (CSH-110/111 pattern). Stable ids per F25. |
| F3  | `src/features/account_details/account_details_view/AccountDetailsView.tsx` (+ test)                             | Record menu gains "Free shares" item, stable id `add-menu-free-shares` (FSD-010, extends the DIV-012 menu at ~line 199).                                                                                                                                                                                                                                                               |
| F4  | `src/features/transactions/shared/presenter.ts` (+ `presenter.test.ts`)                                         | `toTransactionRow`: Type label "Free shares" (TXL-023). For the `—` money-column placeholders (TXL-022, FSD-050): **match the DIV precedent** — the existing Dividend Realized-P&L dash renders in `TransactionListPage.tsx` (~line 222, `account_details.pnl_placeholder`), not in the presenter; put the FreeShares dashes in the same layer DIV uses.                               |
| F5  | `src/features/transactions/transaction_list/TransactionListPage.tsx` (+ test)                                   | Edit routing for free-shares rows → FreeSharesModal in edit mode via the URL-driven `?modal=edit-free-shares&editTxId=` pattern (FSD-040); delete uses the existing cancel flow (FSD-041 — `CascadingOversell` surfaced via the standard error pipeline).                                                                                                                              |
| F6  | `src/i18n/locales/en/common.json` + `src/i18n/locales/fr/common.json`                                           | `transaction.type.free_shares`, modal labels, snackbar, menu item, error keys for the new variants.                                                                                                                                                                                                                                                                                    |

### Rules Coverage

| Rule    | Layer              | Task                                                                   | Notes                                                     |
| ------- | ------------------ | ---------------------------------------------------------------------- | --------------------------------------------------------- |
| FSD-010 | frontend           | F3 — Record menu item                                                  | `[unit-test-needed]` — modifies `AccountDetailsView` menu |
| FSD-011 | backend            | B5 orchestrator guards + B4 error variants                             | mirrors DIV-011                                           |
| FSD-020 | frontend           | F2 modal fields                                                        |                                                           |
| FSD-021 | frontend + backend | F2 inline validation + B2(d)/B3 re-validation                          | TRX-020 date bounds                                       |
| FSD-022 | backend            | B2(a)(c) + B3 UoW                                                      | ADR-006                                                   |
| FSD-023 | backend            | B1 zero-cost packing + B2(a) dilution                                  | ADR-001 micros                                            |
| FSD-024 | backend            | nothing to write — covered by a test asserting no `AssetPrice` write   | negative-space test                                       |
| FSD-025 | frontend           | F2 in-flight/success/error states                                      |                                                           |
| FSD-026 | frontend + backend | B3 publishes `TransactionUpdated`; FE re-fetch already wired (ACD-039) |                                                           |
| FSD-027 | backend            | B2(b) replay arm                                                       | sells after distribution use diluted VWAP                 |
| FSD-028 | backend            | B2(b) + dedicated record→delete→compare test                           | reversibility invariant                                   |
| FSD-040 | frontend + backend | B2(d) correction arm + F5 edit routing + F2 edit mode                  | asset_id immutable                                        |
| FSD-041 | frontend + backend | existing cancel flow + B2(b) replay → `CascadingOversell`              |                                                           |
| FSD-050 | frontend           | F4 presenter                                                           | `[unit-test-needed]` — modifies `toTransactionRow`        |
| FSD-051 | frontend           | no code — emergent from existing ACD/MKT math; assert via RTL/E2E      |                                                           |
| FSD-070 | backend            | B8 match arms                                                          | `[unit-test-needed]` (backend integration)                |

**`modified_functions` for `test-writer-frontend`**: `[transactions/shared/presenter.ts:toTransactionRow, account_details/account_details_view/AccountDetailsView.tsx:AccountDetailsView (Record menu), transactions/transaction_list/TransactionListPage.tsx (edit routing)]`

### E2E

`e2e/account_details/free_shares.test.ts` — one self-cleaning critical path: seed account + asset + buy (10 units) → open Record menu → "Free shares" → record 5 → holding row shows 15 + diluted average price → TXL shows the row (Type "Free shares", `—` money columns) → delete via TXL → holding restored to 10 at original average price (FSD-028 at UI level). Stable ids only (E1/E4); zero network.

---

## 3. PR Plan

- **Strategy**: `2 PRs` (user-selected; executed autonomously — monitor CI + codecov per the per-file convention before each merge, then `just release`)
- **Estimate**: BE ~9 files / ~420 LOC · FE ~11 files / ~480 LOC · E2E ~1 file

**PR 1 — `feat(account): record free-share distributions`**

- Scope: B1–B8 + backend tests + `just generate-types` bindings. Terminates at the Backend-phase `/create-pr`.
- Dependency: none (branch off `main`). Mergeable alone — bindings present, FE not yet consuming.
- Branch: `feat/free-share-distribution` (current branch)

**PR 2 — `feat(account): free-shares UI + E2E closure`**

- Scope: F1–F6 + visual proof + E2E + **housekeeping bundle** (roadmap Phase-4 row, this plan's checkboxes, stale-plan ticks for api-key-management/fx-rate) + spec-checker closure. Terminates at the Closure `/create-pr`.
- Dependency: rebase off `main` after PR 1 merges (consumes new bindings).
- Branch: `feat/free-share-distribution-fe`

After PR 2 merges with CI + codecov green: `/dep-audit` → `just release --preview` → `just release` → publish the draft GH release (`gh release edit vX.Y.Z --draft=false`).
