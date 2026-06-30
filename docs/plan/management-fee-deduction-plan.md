# Implementation Plan — Management Fee Deduction (FEE)

> Spec: `docs/spec/management-fee-deduction.md` · Contract: `docs/contracts/account-contract.md` (account BC)
> Constraining ADRs: **001** (i64 amounts), **003/004** (cross-context use-case orchestration via services), **006** (Unit of Work), **013** (recompute-on-read). The recurring-materialization strategy is `ADR-SUGGESTED` (FEE-040/043/044) — author via `/adr-writer` before/with the BE PR.

## 1. Workflow TaskList

### Setup

- [ ] 📖 Read spec `docs/spec/management-fee-deduction.md`
- [ ] 📖 Read contract `docs/contracts/account-contract.md` (Management Fee section + new Shared Types)
- [ ] 📖 Read ADRs: `001`, `003`, `004`, `006`, `013` (+ write the `ADR-SUGGESTED` materialization ADR)
- [ ] 📖 Read conventions: `ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/backend-patterns.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/frontend-visual-proof.md`, `docs/e2e-rules.md`, `docs/test_convention.md`

### Backend phase — PR #1

- [ ] 🗄️ Migrations (`just migrate` + `just prepare-sqlx`)
- [ ] ✍️ `test-writer-backend` — stubs for `record_management_fee`, `create_fee_schedule`, `update_fee_schedule`, `delete_fee_schedule`, `get_fee_schedule`, `apply_due_fee_deductions` + management-fees aggregation; red confirmed
- [ ] 🏗️ Backend implementation (minimal — make failing tests pass; no speculative code)
- [ ] 🔍 `reviewer-backend` + `reviewer-arch` + `reviewer-sql` (parallel) → `/review-triage` → apply
- [ ] 🔗 `just generate-types`
- [ ] 🔧 `npx tsc --noEmit` (fix binding errors only)
- [ ] 🧹 `just format`
- [ ] 💾 `/smart-commit` — `feat(account): management-fee backend (FEE)`
- [ ] 🔀 `/create-pr` (BE) → merge → branch FE off updated `main`

### Frontend phase — PR #2

- [ ] ✍️ `test-writer-frontend` — gateway + presenter + modal/hook tests; `modified_functions` below; red confirmed
- [ ] 💻 Frontend implementation (implement only what makes failing tests pass — no defensive code, no anticipation of future rules)
- [ ] 📸 `/visual-proof` — ManagementFeeModal, FeeScheduleModal, HoldingRow (Management Fees column), AccountDetailsView header (light + dark)
- [ ] 🔍 `reviewer-frontend` → `/review-triage` → apply
- [ ] 🧹 `just format`
- [ ] 💾 `/smart-commit` — `feat(account): management-fee frontend (FEE)`
- [ ] 🔀 `/create-pr` (FE) → merge → branch E2E off updated `main`

### Closure — PR #3

- [ ] ✍️ `test-writer-e2e` — one-off record + schedule create/catch-up scenarios
- [ ] ▶️ `npm run test:e2e` green
- [ ] 🔍 `reviewer-e2e` + `reviewer-security` (new commands) + `reviewer-infra` (migrations/startup) (parallel) → `/review-triage`
- [ ] 📚 `docs/todo.md`; `ARCHITECTURE.md` (new `use_cases/fee_generation/`, new `context/account` fee_schedule files, `FeeScheduleUpdated` event)
- [ ] ✅ `spec-checker` [HARD GATE]
- [ ] 🧹 `just format`
- [ ] 💾 `/smart-commit` — `test(account): management-fee E2E + closure (FEE)`
- [ ] 🔀 `/create-pr` (E2E) → merge

## 2. Detailed Implementation Plan

### Migrations (`src-tauri/migrations/`) — account bounded context

1. **`202606300001_create_fee_schedules.sql`** — `fee_schedules` table: `id TEXT PK`, `account_id TEXT NOT NULL` (FK accounts, indexed), `asset_id TEXT NOT NULL`, `annual_rate_micros INTEGER NOT NULL` (micro-percent), `frequency TEXT NOT NULL` (Monthly/Quarterly/Annually), `start_date TEXT NOT NULL`, `end_date TEXT NULL`, `active INTEGER NOT NULL DEFAULT 1`, `last_applied_period TEXT NULL`, timestamps. `UNIQUE(account_id, asset_id)` (FEE-031). FK index per reviewer-sql.
2. **`202606300002_add_origin_to_transactions.sql`** — add `origin TEXT NOT NULL DEFAULT 'Manual'` to `transactions` (FeeDeduction discriminator, FEE-022/043). Idempotency-safe per migration conventions.

> Run `just migrate` then `just prepare-sqlx` before any backend code.

### Backend (`src-tauri/src/`)

**Domain**

- `context/account/domain/transaction.rs` — add `TransactionType::ManagementFee` (FEE-022); add `TransactionOrigin { Manual, Scheduled }` enum + `origin` field on `Transaction`; factory for a fee deduction (quantity-reducing, cost basis unchanged — FEE-022/023/050). Average-price concentration via existing TRX-026 floor path.
- `context/account/domain/fee_schedule.rs` **(new)** — `FeeSchedule` aggregate + `FeeFrequency { Monthly, Quarterly, Annually }` (FEE-034, `periods_per_year` 12/4/1); factories `new`/`with_id`/`from_storage`; aggregate-root methods `update_rate_and_end`/`pause`/`reactivate` (FEE-060/061); validation rate `>0` & `<100%`, dates (FEE-032). Domain errors only.
- `context/account/domain/mod.rs` — export `FeeSchedule`, `FeeFrequency`, `TransactionOrigin`.

**Infrastructure / repository**

- `context/account/repository/fee_schedule.rs` **(new)** — `FeeScheduleRepository` trait + Sqlite impl: upsert/get-by-pair/list-active/update/delete (row mapping per `docs/backend-patterns.md`).
- `context/account/repository/transaction.rs` — persist/read `origin` column.
- `context/account/repository/mod.rs` — register fee-schedule repo.

**Error model** (`context/account/error.rs`) — new variants: `ManagementFeeOnCashAsset`, `PercentageNotPositive`, `PercentageAboveHundred`, `RateNotPositive`, `RateAboveHundred`, `EndBeforeStart`, `ScheduleAlreadyExists`, `ScheduleNotFound`. Reuse `AccountNotFound`, `AssetNotFound`, `AssetNotHeld`, `InvalidDate`, `DateInFuture`, `DateTooOld`, `CascadingOversell`, `DatabaseError` (per `docs/error-model.md`).

**Service** (`context/account/service.rs`)

- `record_management_fee(dto)` — eligibility (FEE-012), validate pct (FEE-021), convert pct→qty via holding-as-of replay (`floor(qty_as_of(date) × pct)`, FEE-022a), record FeeDeduction (origin=Manual) in one UoW (ADR-006), oversell guard on replay (FEE-027). Mirror `record_free_shares`.
- Fee-schedule CRUD: `create_fee_schedule` (FEE-030/031/032/033), `update_fee_schedule` (FEE-060/061), `delete_fee_schedule` (FEE-062), `get_fee_schedule` (FEE-030 read). Publish `FeeScheduleUpdated` (FEE-064).

**API** (`context/account/api.rs`) — `#[tauri::command]` wrappers: `record_management_fee`, `create_fee_schedule`, `update_fee_schedule`, `delete_fee_schedule`, `get_fee_schedule`. DTOs per contract (`ManagementFeeDTO`, `CreateFeeScheduleDTO`, `UpdateFeeScheduleDTO`).

**Generation use-case** (`use_cases/fee_generation/` **new**: `orchestrator.rs`, `api.rs`, `mod.rs`)

- `apply_due_fee_deductions()` command (FE-triggered on startup) — list active schedules, for each compute due **completed** periods after `last_applied_period` from `start_date` (FEE-040/044/045), period-boundary dates (FEE-042), sequential removal `floor(qty_as_of × rate ÷ periods_per_year)` (FEE-041/070), skip a period that would oversell (FEE-047), advance cursor (FEE-043), record via `AccountService` (ADR-003/004). Period/date helpers reuse/extend `use_cases/shared/valuation.rs`.

**Aggregation** (`use_cases/account_details/orchestrator.rs`) — add `HoldingDetail.management_fees` (Σ `qty_removed × price_as_of(date)` via `PricedAsset::price_as_of`, FXR-converted, FEE-051/052/054/073) + `AccountDetailsResponse.total_management_fees` (FEE-053); honor as-of date (FEE-072). FEE-071: ensure fee deductions stay excluded from PRF flow/dividend terms (verify `account_performance` replay treats `ManagementFee` like a quantity event, no cash flow).

**Wiring** — `core/specta_builder.rs` register 5 commands + `apply_due_fee_deductions`; `core/event_bus` add `FeeScheduleUpdated` (FEE-064).

### Frontend (`src/`) — after `just generate-types`

- `features/account_details/gateway.ts` — wrappers: `recordManagementFee`, `createFeeSchedule`, `updateFeeSchedule`, `deleteFeeSchedule`, `getFeeSchedule`; `features/transactions/gateway.ts` or a shell gateway — `applyDueFeeDeductions`. Typed `Result` pass-through (F27).
- `features/account_details/management_fee/` **(new)** — `useManagementFee.ts` + `ManagementFeeModal.tsx` (one-off: asset selector, DateField, percentage field, note — FEE-010/020/021/025).
- `features/account_details/fee_schedule/` **(new)** — `useFeeSchedule.ts` + `FeeScheduleModal.tsx` (rate %, FeeFrequency select, start/end DateField, active toggle, Save/Delete — FEE-011/030/032/034/060/061/062/064).
- `features/account_details/account_details_view/HoldingRow.tsx` — **Management Fees** column (FEE-052/056) + "Manage fee" row action opening the schedule modal (FEE-011).
- `features/account_details/account_details_view/AccountDetailsView.tsx` — header **total Management Fees** (FEE-053); add "Management fee" to the Record menu opening the one-off modal (FEE-010).
- `features/account_details/shared/presenter.ts` — map `management_fees` / `total_management_fees` to the view model (FEE-052/053); `shared/validateManagementFee.ts` + `validateFeeSchedule.ts` — pct/rate/date validation (FEE-021/032).
- `features/transactions/shared/presenter.ts` — TXL Type label "Management fee" + `—` placeholders for the new type (FEE-055).
- **Startup hook** — invoke `applyDueFeeDeductions` once on app mount (FEE-040), precedent `src/lib/update/update_banner/useUpdateBanner.ts` mount effect; place in `App.tsx` or a new `src/shell/` startup hook, then refetch account details.
- i18n `src/i18n/locales/{en,fr}/common.json` — modal labels, "Management fee" type label, snackbar "Management fee recorded", error keys for the new variants (i18n-rules; presenter `error.code → key`).

### Rules Coverage

| Rule                    | Layer                 | Task                                                                       | Notes                                       |
| ----------------------- | --------------------- | -------------------------------------------------------------------------- | ------------------------------------------- |
| FEE-010                 | frontend              | Record-menu "Management fee" → `ManagementFeeModal`                        | AccountDetailsView                          |
| FEE-011                 | frontend              | HoldingRow "Manage fee" → `FeeScheduleModal`                               |                                             |
| FEE-012                 | backend               | `AccountService` eligibility guard                                         | reuse FSD path                              |
| FEE-020/021             | frontend + backend    | `ManagementFeeModal` + `validateManagementFee` + BE re-validate            | `[unit-test-needed]` validate               |
| FEE-022/023/050         | backend               | `record_management_fee` + transaction factory                              | ADR-006, TRX-026 floor                      |
| FEE-024                 | backend               | no AssetPrice write                                                        | mirror FSD-024                              |
| FEE-025                 | frontend              | modal in-flight/success/error                                              |                                             |
| FEE-026                 | frontend + backend    | `TransactionUpdated` + ACD refetch                                         |                                             |
| FEE-027                 | backend               | oversell guard on record/edit                                              | `CascadingOversell`                         |
| FEE-030/031/032/033/034 | f+b                   | `create_fee_schedule` + `FeeScheduleModal`                                 | unique pair, validation                     |
| FEE-040–047             | backend               | `use_cases/fee_generation` catch-up                                        | ADR-SUGGESTED, ADR-013                      |
| FEE-051/052/053/054/073 | backend (+fe display) | `account_details` aggregation `management_fees`                            | `PricedAsset::price_as_of`, FXR             |
| FEE-052/053             | frontend              | HoldingRow column + header total + presenter                               | `[unit-test-needed]` presenter              |
| FEE-055                 | frontend              | TXL type label + placeholders                                              | transactions presenter `[unit-test-needed]` |
| FEE-056                 | frontend              | HoldingRow qty/avg reflect                                                 |                                             |
| FEE-060/061/062         | f+b                   | `update_fee_schedule`/`delete_fee_schedule` + modal                        | structural immutability                     |
| FEE-063                 | f+b                   | reuse `correct_transaction`/`cancel_transaction`                           | no new command                              |
| FEE-064                 | f+b                   | `FeeScheduleUpdated` event + ACD subscribe                                 |                                             |
| FEE-070/071             | backend               | generation skip-empty; PRF neutrality                                      |                                             |
| FEE-072                 | backend (+fe display) | as-of Management Fees figure; rendered by the FEE-052 column in as-of mode |                                             |

**`modified_functions`** (for `test-writer-frontend`): `account_details/shared/presenter.ts:<account-details mapper>`, `account_details/account_details_view/HoldingRow.tsx` (column render), `transactions/shared/presenter.ts:<type-label mapper>`, `account_details/account_details_view/AccountDetailsView.tsx` (header total + Record menu).

## 3. PR Plan

- **Strategy**: **3 PRs** (BE → FE → E2E).
- **Estimate**: BE ~16–18 files / ~700–900 LOC (migration ×2, new fee_schedule domain+repo, transaction variant, new fee_generation use-case, aggregation, service+api, error, event, specta). FE ~12–14 files / ~500–650 LOC (2 modal+hook pairs, gateway, presenter ×2, validators, HoldingRow, AccountDetailsView, startup hook, i18n ×2). E2E ~2–3 files. Both core layers exceed the ~20-file/~500-LOC split threshold → 3 PRs.
- **PR #1 — `feat(account): management-fee backend (FEE)`** — branch `feat/quantity-fee` (current): migration + domain + repo + service + use-cases + api + bindings + ADR. Terminates at the Backend-phase `/create-pr`. Mergeable alone (bindings present, unused).
- **PR #2 — `feat(account): management-fee frontend (FEE)`** — branch `feat/quantity-fee-fe` off merged `main`: gateway, modals/hooks, presenter, HoldingRow/AccountDetailsView, startup hook, i18n, visual proof. Terminates at the Frontend-phase `/create-pr`.
- **PR #3 — `test(account): management-fee E2E + closure (FEE)`** — branch `feat/quantity-fee-e2e` off merged `main`: E2E specs, ARCHITECTURE/todo, spec-checker closure.
