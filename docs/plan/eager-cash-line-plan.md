# Plan — Eager Cash Line at Account Creation

Derived from the eager-cash-line spec amendments (CSH-010/012/013/019/022/024/032/040/041/050/090/092/095/097/098, ACC-025, ACD-020/044/051, DIV-012, FSD-010). No contract change (contract-reviewer confirmed `add_account` interface is unchanged).

**Goal**: every account owns a persisted 0-balance Cash Holding from creation; the cash holding is never auto-deleted (persists at 0); the cash row is always visible; the header "Record" dropdown drops its Deposit/Withdraw entries. Existing accounts are backfilled by a one-off insert-if-absent SQL migration.

**Key design decision (post plan-review)**: cash seeding is **orchestrator-driven**, NOT folded into `AccountService::create` or `Account::new`. Reason: (a) the cash _asset_ must exist before a cash _holding_ can be written (FK `holdings.asset_id → assets.id`), and only the orchestrator seeds it via `ensure_cash_asset`; (b) `AccountService::create` is a setup helper in ~7 use-case test modules (`delete_asset`, `archive_asset`, `account_details`, `account_deletion`, `holding_transaction`, `account_summary`, `account_performance`) — leaving it account-only means **zero downstream test fan-out**. `create` and `Account::new` stay unchanged.

**Atomicity (ADR-006 NOT used)**: the orchestrator runs three sequential commits — `ensure_cash_asset` → `account_service.create` → `account_service.seed_cash_holding`. This mirrors the existing non-atomic `ensure_cash_asset`-then-write pattern in `holding_transaction` (which also does not use ADR-006's `TransactionManager`). A mid-sequence failure leaves an account without its cash holding; that state is self-healing (the same insert-if-absent backfill migration, or a re-run, repairs it) and DB failures here are rare. No `AppUnitOfWork`/`SqlxUnitOfWork` is introduced.

---

## 1. Workflow TaskList

**Setup** — read before coding:

- `docs/spec/cash-tracking.md` (CSH-010/012/013/019/022/090/095/097), `docs/spec/account.md` (ACC-025), `docs/spec/account-details.md` (ACD-020/044/051)
- `docs/contracts/account-contract.md` (`add_account` row — unchanged)
- ADRs: ADR-003/ADR-004 (cross-context orchestration via injected services), ADR-001 (i64 micros). ADR-006 is intentionally NOT applied (see Atomicity above).
- `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/backend-patterns.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/test_convention.md`

**Backend phase**

- [ ] SQL backfill migration (`just migrate` + `just prepare-sqlx`)
- [ ] `test-writer-backend` → red baseline (modified_functions: `account.rs:replay_cash_holding`, `account_details/orchestrator.rs:<partition>`). **Invert** the existing `account.rs:~1771 csh_013_cash_holding_removed_when_last_deposit_cancelled` test (it asserts the drop-at-zero branch being removed → cash holding now persists at 0).
- [ ] Implement backend — **implement only what makes the failing tests pass; no defensive code, no anticipation of future rules**
- [ ] `reviewer-backend` + `reviewer-arch` + `reviewer-sql` → `/review-triage`
- [ ] `just generate-types` → `npx tsc --noEmit` (bindings regenerate identically — verify, fix only binding-driven TS errors)
- [ ] `just format` → `/smart-commit` (backend)

**Frontend phase**

- [ ] `test-writer-frontend` → red baseline. modified_functions: `useAccountDetails.ts:hasVisibleCashRow/holdings`, `AccountDetailsView.tsx` (menu + banner removal). Update the two NoCashBanner consumers: `useAccountDetails.test.ts`, `AccountDetailsView.test.tsx`.
- [ ] Implement frontend — **implement only what makes the failing tests pass; no defensive code**
- [ ] `/visual-proof` (AccountDetailsView — fresh account showing €0.00 cash row, menu without cash entries)
- [ ] `reviewer-frontend` → `/review-triage`
- [ ] `just format` → `/smart-commit` (frontend)

**Closure**

- [ ] `test-writer-e2e` → cash row visible at €0.00 on a freshly created account; Deposit reachable from the cash row; run suite
- [ ] `reviewer-e2e` → `/review-triage` (reviewer-sql already ran in BE phase; no Tauri-command/capability/secret change → reviewer-security not warranted)
- [ ] Docs: close `docs/todo.md` eager-cash-line entry; `ARCHITECTURE.md` (new `use_cases/account_creation`)
- [ ] `spec-checker` [HARD GATE]
- [ ] `just format` → `/smart-commit` (closure)
- [ ] `/create-pr`

---

## 2. Detailed Implementation Plan

### Migrations

`src-tauri/migrations/{ts}_backfill_eager_cash_holdings.sql` — **insert-if-absent** backfill (CSH-012). This migration is **cross-context** (seeds `asset`-context `asset_categories`/`assets` + `account`-context `holdings`) — note for reviewer-sql.

- Begin with `PRAGMA defer_foreign_keys = ON;` (the `foreign_keys` pragma is a no-op inside the sqlx transaction; `defer_foreign_keys` defers FK checks to commit and auto-resets). Insert parent→child regardless.
- (1) Insert the Cash category `system-cash-category` if absent.
- (2) For every distinct currency across existing `accounts`, insert `system-cash-{lower(currency)}` Cash Asset if absent.
- (3) For every `accounts` row lacking a cash holding for its currency, insert `id='cash-'||a.id`, the cash `asset_id`, `quantity=0`, `average_price=1000000`, `total_realized_pnl=0`, `last_sold_date=NULL`. **Do not** touch accounts that already have a cash holding (preserve old lazy-path balances).
- **Column names/types and exact seed values MUST be read from source first**: `migrations/202602080000_init.sql` (assets/asset_categories — note `asset_class`, `category_id`, `is_archived`, `is_deleted`, …) + the six asset ALTER migrations, `migrations/202604120001_replace_asset_accounts_with_holdings.sql` (holdings + FKs), and `src-tauri/src/context/asset/service.rs::seed_cash_asset` (canonical column values) + `src-tauri/src/core/cash.rs` (id derivation). Mirror them exactly.
- Run `just prepare-sqlx` after. **Test against a DB seeded with accounts + a pre-existing cash holding**, not just empty (SQLite FK-deferral bit a prior project).

### Backend

**`src-tauri/src/use_cases/account_creation/{mod.rs, orchestrator.rs, api.rs}`** (new — mirrors `account_deletion`'s file shape, not its dependencies):

- `AccountCreationUseCase { account_service: Arc<AccountService>, asset_service: Arc<AssetService> }` + `new(...)`.
- `create(name, currency, update_frequency) -> StdResult<Account, AccountCrudError>`:
  1. `self.asset_service.seed_cash_asset(&currency).await` — returns `anyhow::Result`; map the error to `AccountCrudError`'s `DatabaseError` and log server-side via `tracing::error!` (error-model gold; do **not** bubble `anyhow`). Idempotent (CSH-010/011/017). Call `seed_cash_asset` directly; do **not** relocate the one-line `ensure_cash_asset` wrapper.
  2. `let account = self.account_service.create(name, currency, update_frequency).await?;` (unchanged — account row only).
  3. `self.account_service.seed_cash_holding(&account.id).await?;` (new method — see below).
  4. return `account`.
- `api.rs`: `#[tauri::command] #[specta::specta] add_account(uc: State<AccountCreationUseCase>, dto: CreateAccountDTO) -> Result<Account, AccountCrudError>` — **identical wire shape to today** (single `CreateAccountDTO { name, currency, update_frequency }` arg, return `Account`, so `bindings.ts` regenerates byte-identical). The command unpacks the dto and calls `uc.create(dto.name, dto.currency, dto.update_frequency)`.

**`src-tauri/src/context/account/domain/account.rs`**:

- New `Account::seed_cash_holding(&mut self)` — pushes a 0-quantity cash `Holding::restore(uuid, id, cash_asset_id(), 0, 1_000_000, 0, None)` into `self.holdings` + `pending_changes` (`HoldingUpserted`). Idempotent guard: no-op if a cash holding already exists. (`Account::new` stays unchanged.)
- `replay_cash_holding` — delete the CSH-013 "drop at zero" branch (lines ~765–785); always upsert the cash holding (incl. at 0). Update the doc comment (no transition comment).

**`src-tauri/src/context/account/service.rs`**:

- New `seed_cash_holding(&self, account_id: &str) -> Result<(), AccountCrudError>`: load the account → `account.seed_cash_holding()` → save the pending-changes (same `repo.save`-flushes-`pending_changes` mechanism `record_deposit` uses via `save_account`). **Error-type seam**: `record_deposit`'s `load_account`/`save_account` helpers are typed to `HoldingTransactionError`, not `AccountCrudError`; this method needs its own load/save returning `AccountCrudError` (both enums share `AccountApplicationError` as their `#[from]` source, so `AccountNotFound`/`DatabaseError` compose) — do **not** reuse the `HoldingTransactionError`-typed helpers directly. No change to `create`.

**`src-tauri/src/core/cash.rs`**:

- Add a small `is_cash_asset(asset_id: &str) -> bool` id-prefix predicate (`asset_id.starts_with("system-cash-")`) — it does **not** exist today (only `system_cash_asset_id(currency)`). Needed by the partition below, which runs on raw holdings before asset-class enrichment.

**`src-tauri/src/use_cases/account_details/orchestrator.rs`** (CSH-090 / ACD-044):

- Change the active/closed `partition` (line ~146) so the Cash Holding is **always** active: `partition(|h| h.quantity > 0 || is_cash_asset(&h.asset_id))`. The partition runs on raw holdings _before_ asset-class enrichment (the `class == Cash` check at ~line 194 is too late), so it must use the id-prefix predicate, not the class. The 0-cash holding enriches into `active_holdings`, never `closed_holdings`.

**`src-tauri/src/core/specta_builder.rs`**: re-point `add_account` registration `account::add_account` → `account_creation::add_account`.
**`src-tauri/src/lib.rs`**: construct + `.manage(AccountCreationUseCase::new(account_service.clone(), asset_service.clone()))`.
**`src-tauri/src/context/account/api.rs`**: remove the old `add_account` command (dead after the move — verified no other caller; `emit_account_updated` stays in `service.rs::create`).

### Frontend

**`src/features/account_details/account_details_view/AccountDetailsView.tsx`**:

- In the DIV-012 "Record" dropdown, remove the `add-menu-deposit` and `add-menu-withdraw` `<button role="menuitem">` items (and the `view.hasVisibleCashRow` gate around withdraw). Keep `add-menu-open-balance`, `add-menu-dividend`, `add-menu-free-shares`.
- Remove the `<NoCashBanner>` render + `view.showNoCashBanner`. Remove the dead empty-state branch keyed on `!hasVisibleCashRow` (cash row is always present → table always renders).

**Delete `NoCashBanner.tsx`** (+ `NoCashBanner.test.tsx`). Update its consumers' tests: `useAccountDetails.test.ts` (drop `showNoCashBanner`/banner cases; assert cash row present at qty 0) and `AccountDetailsView.test.tsx` (drop banner assertions; assert no `add-menu-deposit`/`add-menu-withdraw`).

**`useAccountDetails.ts` / `useAccountDetailsView.ts`**: cash row is always in `holdings` → `hasVisibleCashRow` is constant-true; drop `showNoCashBanner`. Remove `hasVisibleCashRow` if no longer consumed after the menu/banner removal. Class-grouping (ACD-051) already renders cash first.

**`HoldingRow.tsx`**: cash variant already disables Withdraw at `quantityMicro <= 0` (CSH-097) — **no change expected** (verify only).

**i18n** (`src/i18n/locales/{en,fr}/common.json`): remove `account_details.action_deposit` / `action_withdraw` header keys only if orphaned after the menu removal (grep first). Keep `cash.action_record_deposit/withdrawal`.

### Rules Coverage

| Rule                        | Layer    | Task                                                                          | Notes                                                                  |
| --------------------------- | -------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| CSH-010                     | backend  | orchestrator `seed_cash_asset` first                                          | ADR-003/004                                                            |
| CSH-012                     | backend  | `seed_cash_holding` (domain+service+orchestrator); backfill migration         | insert-if-absent                                                       |
| CSH-013                     | backend  | `replay_cash_holding` drops delete branch                                     | `[unit-test-needed]`                                                   |
| ACC-025                     | backend  | `add_account` → `use_cases/account_creation`; 3-step seq                      | non-atomic (see Atomicity)                                             |
| CSH-090                     | backend  | `account_details` partition keeps cash active at 0                            | `[unit-test-needed]`                                                   |
| ACD-044                     | backend  | cash never in `closed_holdings`                                               | same partition change                                                  |
| CSH-022/024/032/040/041/050 | backend  | **reword-only, no code task** — already operate on the always-present holding | verified no behavioural change                                         |
| CSH-019                     | frontend | remove cash actions from header (none remain)                                 |                                                                        |
| DIV-012                     | frontend | remove `add-menu-deposit`/`add-menu-withdraw` from the Record dropdown        | keep New position/Dividend/Free shares                                 |
| CSH-095                     | frontend | delete `NoCashBanner`; cash row always rendered                               | + update 2 consumer tests                                              |
| CSH-097                     | frontend | cash row at 0 via `useAccountDetails` (always includes cash)                  | Withdraw-disable already exists in `HoldingRow` — no HoldingRow change |
| ACD-020/051, CSH-092/098    | frontend | unchanged — cash already first & exempt (ACD-051 shipped)                     | no-op, listed for traceability                                         |

---

## 3. PR Plan

- **Strategy**: **1 PR** — `feat/eager-cash-line`.
- **Rationale**: the lifecycle change couples BE and FE — the FE always-visible cash row depends on the BE always emitting the 0-cash holding; shipping BE alone would let the existing FE render the 0-cash row while the NoCashBanner logic still runs → transient inconsistency. Commit per layer inside the single PR (backend → frontend → closure).
- **Estimate**: ~450–650 LOC across ~13 files. BE side (new `account_creation/` trio + `seed_cash_holding` domain+service + `replay` edit + partition + migration) is the larger half but stays under the ~20-file / ~500-LOC per-layer split threshold; `/start` should re-check against the real diff before the BE commit and split BE→FE if it overruns.
- **Branch**: `feat/eager-cash-line`.
