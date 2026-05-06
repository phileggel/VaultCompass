# Plan — Cash Tracking (CSH) Feature

> Branch: `feat/cash-tracking` (off `main` at `8b02061`).
> Spec: `docs/spec/cash-tracking.md` (28 rules, all Open Questions resolved 2026-05-06).
> Contract: `docs/contracts/account-contract.md` (cash commands + error variants upserted; reviewer findings W1/W2/W3/Nit applied).
> Cross-amends: `docs/spec/account-details.md` (`total_global_value`, ACD-020/034 reverted under hide-at-0).
> Meta-plan: `docs/plan/cash-tracking-plan.md` (background; the prerequisite refactor `refactor/holding-transaction` shipped as PR #4).
> ADRs in scope: ADR-001 (i64 micros), ADR-003 (cross-context use cases use sequential service calls), ADR-004 (use cases inject services), ADR-006 (UoW for cross-aggregate atomicity).

---

## Reality vs. meta-plan — deviations to flag

The meta-plan was drafted before the prerequisite refactor landed. A few anchors have shifted:

- **Single orchestrator, not five.** `use_cases/holding_transaction/orchestrator.rs` exposes one struct `HoldingTransactionUseCase` with five async methods (`open_holding`, `buy_holding`, `sell_holding`, `correct_transaction`, `cancel_transaction`). The meta-plan still talks of "five orchestrator structs" — wrong. New cash methods (`record_deposit`, `record_withdrawal`) attach to that same struct.
- **`ensure_cash_asset` already wired (as a no-op).** `src-tauri/src/use_cases/holding_transaction/shared/ensure_cash_asset.rs` exists with the signature `pub async fn ensure_cash_asset(_asset_service: &Arc<AssetService>, _currency: &str) -> Result<()>`. The implementation is `Ok(())`. This plan upgrades it in place.
- **Holding entity already carries `total_realized_pnl` + `last_sold_date`.** ACD shipped earlier. The cash-replay invariants (CSH-024/033/042/051) build on the existing chronological replay in `Account::recalculate_holding`, not on a new method.
- **`AccountDetailsResponse.total_global_value`** is a new field on the existing DTO — no new command, no new use case (CSH-094). The contract entry is already in `docs/contracts/account-contract.md`.
- **CSH-016 (asset-mutation rejection on `class = Cash`)** is **out of scope** for this plan. It belongs to a follow-up upsert against `docs/contracts/asset-contract.md` and a separate small PR. Tracked at the end as **Parallel work item — CSH-016**.
- **Subagent model.** The `seven_day_sonnet` weekly cap is at 100% until 2026-05-08 21:00 UTC; subagents are flipped to opus via `scripts/swap-agent-model.sh`. Mentioned only because each `test-writer-*` / `reviewer-*` invocation runs against opus until the swap is reverted.

---

## Workflow TaskList

> One-line per CLAUDE.md mandatory step. Tick as you go.

- [ ] Review architecture & rules (`ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/frontend-rules.md`, `docs/e2e-rules.md`, `docs/ddd-reference.md`, `docs/ubiquitous-language.md`)
- [ ] Database migration: write `src-tauri/migrations/{ts}_add_cash_transaction_types.sql`, run `just migrate`, then `just prepare-sqlx`
- [ ] Backend test stubs (`test-writer-backend` against `docs/contracts/account-contract.md` cash subset — all stubs red-confirmed)
- [ ] Backend implementation (minimal — make failing tests pass; green-confirmed)
- [ ] `just format` (rustfmt + clippy --fix)
- [ ] Backend review (`reviewer-backend` → fix issues)
- [ ] Type synchronization (`just generate-types`)
- [ ] Compilation fixup (TypeScript errors from new bindings only — no UI work)
- [ ] `just check` — TypeScript clean
- [ ] Commit: backend layer — `feat(cash): add deposit/withdrawal commands and cash-replay guard`
- [ ] Frontend test stubs (`test-writer-frontend` — gateway, hooks, modals, presenter; `modified_functions: ["useEditTransactionModal.ts:handleSubmit (InsufficientCash mapping)", "AccountDetailsView.tsx (Deposit/Withdraw header buttons + Cash row routing + No-cash banner)", "presenter.ts:toAccountSummary (totalGlobalValue field)", "presenter.ts:toHoldingRow (cash row variant — no cost basis, no actions)"]`)
- [ ] Frontend implementation (minimal — make failing tests pass; green-confirmed)
- [ ] `just format`
- [ ] Visual proof (`/visual-proof` — Account Details with cash row, no-cash banner, Deposit/Withdrawal modals, InsufficientCash inline error; light + dark)
- [ ] Frontend review (`reviewer-frontend` → fix issues)
- [ ] Commit: frontend layer — `feat(cash): deposit/withdraw modals, cash row, global value`
- [ ] E2E tests (`test-writer-e2e` against the cash flows; if `e2e/cash/` not yet seeded, follow `docs/e2e-rules.md` to add it; green-confirmed)
- [ ] Frontend review (`reviewer-frontend` → E2E test files)
- [ ] Commit: E2E tests — `test(e2e): cover deposit, withdrawal, insufficient-cash, global-value`
- [ ] Cross-cutting reviews: `reviewer-arch` (whole feature) + `reviewer-sql` (the cash migration) + `reviewer-infra` (only if any config/script/hook changes)
- [ ] Documentation update: `ARCHITECTURE.md` (cash methods on `HoldingTransactionUseCase`, `total_global_value`, frontend modal additions); `docs/todo.md` (entries in English; tick the contract-consolidation entry only if relevant); flip CSH spec Open Questions check-list and update `docs/plan/cash-tracking-plan.md` "What's NOT done" tracker
- [ ] Spec check (`spec-checker` against `docs/spec/cash-tracking.md` — every CSH-NNN must trace to a test + a file)
- [ ] Commit: tests & docs — `chore(cash): docs, todo, spec-checker pass`
- [ ] `/create-pr`

---

## 1. Migration

**Reason for migration**: the spec adds two new `TransactionType` discriminants (`Deposit`, `Withdrawal`). The `transactions.transaction_type` column is `TEXT NOT NULL DEFAULT 'Purchase'` (see `src-tauri/migrations/202604120002_create_transactions.sql`); the column is free-form text on the SQLite side, but the Rust enum changes must be coupled to a new sqlx-prepared compile-time check. No schema column changes are required for `holdings` — Cash Holdings reuse the existing schema (`average_price = 1_000_000`, `total_realized_pnl = 0`, `last_sold_date IS NULL`).

**Idempotency guards** (CSH-010, CSH-011, CSH-017): the system Cash Asset and the system Cash Category are seeded **lazily at runtime** by `ensure_cash_asset` (not via migration). This is a deliberate departure from the asset-spec's `default-uncategorized` seeding (which is migration-time): cash assets only need to exist for currencies actually used, and seeding is currency-driven, not migration-driven. The migration therefore does **not** insert any rows.

**Migration file** — name suggestion: `src-tauri/migrations/{timestamp}_add_cash_transaction_types.sql`

```sql
-- CSH-022 / CSH-032: Deposit and Withdrawal join the existing TransactionType set.
-- The `transaction_type` column is TEXT — no DDL change is required at the SQL level.
-- This migration is intentionally empty content-wise but materialises the version bump
-- so `just prepare-sqlx` regenerates the offline metadata against the new enum variants.
SELECT 1;
```

> If reviewer-sql prefers a no-op migration to be replaced by a comment-only schema bump or by deferring to compile-time enum changes only, drop the file and rely solely on `just prepare-sqlx`. Confirm during reviewer-sql.

**After migrating**:

1. `just migrate`
2. `just prepare-sqlx` — refresh offline metadata so the new `TransactionType` round-trips through `from_str` / `to_string` cleanly in repository code.
3. `just clean-db` (developer's local DB only — spec line 13 confirms no backfill).

---

## 2. Backend implementation plan

### 2.1 Domain layer (`src-tauri/src/context/account/`)

**Extend `TransactionType` enum (CSH-022, CSH-032)** — `src-tauri/src/context/account/domain/transaction.rs`

- Add `Deposit` and `Withdrawal` variants to the enum (currently `Purchase`, `Sell`, `OpeningBalance`).
- Both derive through `strum_macros::Display` / `EnumString` automatically — confirm round-trip via inline test (mirror `opening_balance_round_trips_through_strum`).
- `Transaction::validate` already accepts any `transaction_type`; no change to validation logic for cash transactions (date bounds, quantity > 0, exchange_rate > 0, total_amount > 0 all hold for Deposit/Withdrawal as encoded in the spec — `unit_price = 1_000_000`, `exchange_rate = 1_000_000`, `fees = 0`, `total_amount = quantity`).

**New error variant on `AccountOperationError` (CSH-080)** — `src-tauri/src/context/account/domain/error.rs`

- Add variant: `InsufficientCash { current_balance_micros: i64, currency: String }` with `#[error(...)]` message "Not enough cash …".
- This is the canonical domain error; `TransactionCommandError` (boundary type, see 2.5) maps it to its own `InsufficientCash { current_balance_micros, currency }` variant.

**Aggregate root methods on `Account` (CSH-022, CSH-032, CSH-040, CSH-050, CSH-024, CSH-033, CSH-042, CSH-051)** — `src-tauri/src/context/account/domain/account.rs`

The existing write methods (`buy_holding`, `sell_holding`, `correct_transaction`, `cancel_transaction`) operate on **a single (account_id, asset_id) pair** at a time and use `recalculate_holding` for that pair. Cash transactions need to mutate **two holdings simultaneously** (the asset holding and the cash holding) within the same aggregate write.

Two new aggregate-root methods:

- `Account::record_deposit(date: String, amount: i64, note: Option<String>) -> Result<&Transaction>` (CSH-022)
  - Resolves `cash_asset_id = system-cash-{self.currency.lower()}` (constant helper at module level).
  - Constructs `Transaction::new` with `transaction_type = TransactionType::Deposit`, `quantity = amount`, `unit_price = 1_000_000`, `exchange_rate = 1_000_000`, `fees = 0`, `total_amount = amount`, `realized_pnl = None`.
  - Pushes the transaction and runs `recalculate_holding(&cash_asset_id, &pair_txs)` over **only** cash-typed transactions for that asset_id (Deposit / Withdrawal). The existing `recalculate_holding` already sums Purchase/OpeningBalance and subtracts Sell — for cash, we need a parallel branch that sums `Deposit.total_amount` and subtracts `Withdrawal.total_amount`. Two options, decide during impl:
    - **Option A (preferred):** add Deposit/Withdrawal arms to the existing `match t.transaction_type` inside `recalculate_holding` — Deposit treated like Purchase (additive, qty += total_amount, vwap stays at 1.0), Withdrawal treated like a constrained Sell (subtract qty, no realized_pnl). Reuses code; minimal diff.
    - **Option B:** dedicated `recalculate_cash_holding` helper. Duplicates loop structure for clarity.
    - Option A wins on KISS — see CLAUDE.md "200 lines could be 50".
  - Pushes `AccountChange::TransactionInserted` + `AccountChange::HoldingUpserted` to `pending_changes`.

- `Account::record_withdrawal(date: String, amount: i64, note: Option<String>) -> Result<&Transaction>` (CSH-032)
  - Same shape as `record_deposit` but with `TransactionType::Withdrawal`.
  - **Eligibility check (CSH-080)** before pushing the transaction: query the in-memory cash holding's current quantity (`cash_holding_quantity()` helper). If `< amount`, return `AccountOperationError::InsufficientCash { current_balance_micros: current_quantity, currency: self.currency.clone() }`.
  - If the cash holding does not exist (CSH-012 — withdrawals do not lazy-create), return `InsufficientCash { current_balance_micros: 0, currency: self.currency.clone() }`.

**Cash side-effect on existing aggregate methods (CSH-040, CSH-050, CSH-042, CSH-024, CSH-051)** — same file

The existing `buy_holding` / `sell_holding` / `correct_transaction` / `cancel_transaction` methods must additionally mutate the cash holding when the touched asset is non-cash:

- `buy_holding`: after the existing asset-holding recalculation, evaluate the cash debit. If the cash holding is missing or `quantity < total_amount`, return `AccountOperationError::InsufficientCash { current_balance_micros: current_cash_quantity_or_zero, currency: self.currency.clone() }`. **Before any mutation is queued** — see CSH-080 ordering. On success, replay the cash holding from scratch over all Deposit/Withdrawal/Purchase/Sell transactions for the cash asset_id (running balance), and queue `AccountChange::HoldingUpserted` for the cash holding.
- `sell_holding`: after the existing asset-holding recalculation, lazy-create the cash holding if absent (CSH-012) and replay it. Always credits — never violates CSH-080.
- `correct_transaction`: full chronological replay of both the touched asset's holding (existing) and the cash holding (new). On any chronological step where running cash drops below 0, return `InsufficientCash` (CSH-042 / CSH-051).
- `cancel_transaction`: same cash replay; deletes that drive the running balance negative for any later transaction return `InsufficientCash` (CSH-024 / CSH-051). Withdrawals deletions never raise it (their removal increases the running balance).

**Internal helper — `Account::replay_cash_holding(cash_asset_id: &str) -> Result<Holding>`**:

- Loads `transactions` filtered by `asset_id == cash_asset_id || transaction_type ∈ {Purchase, Sell}` for the account.
- Sorts by `(date ASC, created_at ASC)` (TRX-036 ordering already used by the repo).
- Iterates: Deposit and Sell → `running += t.total_amount`; Withdrawal and Purchase → `running -= t.total_amount`. Track `running`; if any iteration produces `running < 0`, return `InsufficientCash { current_balance_micros: running_before_step, currency: self.currency.clone() }`.
- After full iteration, build `Holding::with_id` (existing) or `Holding::new` (CSH-012 lazy-create), with `quantity = running`, `average_price = 1_000_000`, `total_realized_pnl = 0`, `last_sold_date = None`.
- If `running == 0` AND no cash transactions remain, queue `AccountChange::HoldingDeleted` instead (TRX-034 / CSH-013).

**Cash holding helpers** — same file, private:

- `fn cash_asset_id(&self) -> String` — returns `format!("system-cash-{}", self.currency.to_lowercase())`. Pulled out so the Tauri layer and the seeding helper stay in sync.
- `fn cash_holding_quantity(&self) -> i64` — convenience over `holding_quantity(cash_asset_id)`.

> **ADR-006 boundary**: All four steps (asset holding upsert, cash holding upsert, transaction insert, optional cash holding delete) accumulate as `AccountChange` entries on the aggregate's `pending_changes`. The existing `AccountRepository::save` already commits the entire `pending_changes` slice atomically inside a single sqlx transaction (see `repository/account.rs`). No new `AppUnitOfWork` super-trait is required — the aggregate boundary is the UoW boundary, consistent with B26 ("Single-aggregate writes do NOT use UoW"). Cash and asset holdings live in the same aggregate (Account), so the existing per-aggregate atomic save handles CSH-022/032/040/050.

### 2.2 Repository layer

**No schema change** for `holdings` — the cash holding fits the existing columns (CSH guard: `average_price = 1_000_000`, `total_realized_pnl = 0`, `last_sold_date = NULL`).

**No new repository trait method.** All cash mutations flow through the existing `AccountRepository::save` and `TransactionRepository::create / update / delete` already wired by `AccountChange`. The repository continues to round-trip `TransactionType` via `strum::Display` / `FromStr` — the new variants cost nothing.

> Verify post-`just prepare-sqlx`: open `src-tauri/.sqlx/` and confirm no compile-time SQL needs touching. None expected (no enum is bound at the SQL boundary; `transaction_type` is `TEXT`).

### 2.3 Service layer (`src-tauri/src/context/account/service.rs`)

**New AccountService methods** (thin orchestrators following the load → call root method → save → emit pattern, B23):

- `AccountService::record_deposit(account_id: &str, date: String, amount: i64, note: Option<String>) -> Result<Transaction>` (CSH-022)
- `AccountService::record_withdrawal(account_id: &str, date: String, amount: i64, note: Option<String>) -> Result<Transaction>` (CSH-032)

Both follow the existing `buy_holding` template:

1. `account_repo.get_with_holdings_and_transactions(account_id)?.ok_or(AccountDomainError::AccountNotFound)?`
2. Call `account.record_deposit(...)` (or `record_withdrawal`).
3. `account_repo.save(&mut account).await?`.
4. Emit `Event::TransactionUpdated` (existing event — CSH-100 explicitly reuses it).
5. Return the inserted transaction (last in `account.transactions`).

The four existing service methods (`buy_holding`, `sell_holding`, `correct_transaction`, `cancel_transaction`) need **no signature change** — they already delegate to aggregate-root methods that now carry the cash side-effect.

### 2.4 Use case layer (`src-tauri/src/use_cases/holding_transaction/`)

**`shared/ensure_cash_asset.rs` — real implementation** (CSH-010, CSH-011, CSH-017)

Replace the stub with:

```text
pub async fn ensure_cash_asset(asset_service: &Arc<AssetService>, currency: &str) -> Result<Asset>
```

Algorithm:

1. Compute `cash_asset_id = format!("system-cash-{}", currency.to_lowercase())`.
2. `asset_service.get_asset_by_id(&cash_asset_id).await?` — if `Some`, return it (CSH-011 idempotent reuse).
3. Otherwise, ensure the system Cash Category exists:
   - `asset_service.get_category_by_id("system-cash-category").await?` — if `None`, create it with `AssetCategory::with_id("system-cash-category".to_string(), "Cash".to_string())` and call a new `AssetService` method `create_category_with_id(category)` (or extend the existing `create_category` to accept a known ID — decide during impl; the latter is less surface area).
4. Create the Cash Asset via `Asset::with_id(cash_asset_id, format!("Cash {}", currency), AssetClass::Cash, cash_category, currency.to_string(), 1, currency.to_string(), false)` and persist via `AssetService::create_asset_with_id` (new method that bypasses the auto-generated UUID — needed because the cash asset ID is deterministic per CSH-011). Same plumbing as `create_category_with_id`.
5. Race-safe: if the create returns a primary-key conflict, do a second `get_asset_by_id` and return it (CSH-011 — "treats a primary-key collision as 'already exists'").

> The new `AssetService::create_asset_with_id` and `create_category_with_id` methods are minimal — they wrap the existing repos' `create` calls but use `Asset::with_id` / `AssetCategory::with_id` instead of `::new`. Their tests can be inline.

**Wire `ensure_cash_asset` into `HoldingTransactionUseCase`** — `src-tauri/src/use_cases/holding_transaction/orchestrator.rs`

For the four existing methods (`buy_holding`, `sell_holding`, `correct_transaction`, `cancel_transaction`), the doc comments already say `ensure_cash_asset` will be wired by CSH. Plan:

- Each method, immediately after fetching the account (which we'll need to do to get the currency for the seeding step) and **before** delegating to `account_service`, calls `ensure_cash_asset(&self.asset_service, &account.currency).await?`.
- This means the orchestrator must read the account once to get the currency. Two options:
  - Fetch via `AccountService::get_by_id(account_id)` — adds a round-trip per call. Acceptable for now; cleanest.
  - Pass currency through the DTO from the frontend. Rejected — leaks domain into transport.
- After seeding, delegate to `AccountService::buy_holding` etc. as today.
- For `correct_transaction` and `cancel_transaction`, the account is also looked up by service — same `ensure_cash_asset` precedes the delegation.

**Two new orchestrator methods** — same file:

- `HoldingTransactionUseCase::record_deposit(account_id: &str, date: String, amount: i64, note: Option<String>) -> Result<Transaction>` (CSH-022)
- `HoldingTransactionUseCase::record_withdrawal(account_id: &str, date: String, amount: i64, note: Option<String>) -> Result<Transaction>` (CSH-032)

Both:

1. Look up the account (for currency).
2. `ensure_cash_asset(&self.asset_service, &account.currency).await?`.
3. Delegate to `AccountService::record_deposit` / `record_withdrawal`.

**Note on `open_holding` (CSH-061)**: extend the existing asset-class guard. Currently it rejects archived assets. Add: if the resolved asset's class is `AssetClass::Cash`, return `OpeningBalanceDomainError::OpeningBalanceOnCashAsset` (new variant — see 2.5).

### 2.5 API/error layer (`src-tauri/src/context/account/api.rs` and `src-tauri/src/use_cases/holding_transaction/api.rs`)

**`TransactionCommandError` — new variant** (`context/account/api.rs`, CSH-080)

```text
InsufficientCash {
    current_balance_micros: i64,
    currency: String,
},
```

`to_transaction_error` gains a new arm:

```text
if let AccountOperationError::InsufficientCash { current_balance_micros, currency } = err {
    return TransactionCommandError::InsufficientCash {
        current_balance_micros: *current_balance_micros,
        currency: currency.clone(),
    };
}
```

This propagates through `buy_holding`, `correct_transaction`, `cancel_transaction` (CSH-041, CSH-042, CSH-024, CSH-051). `sell_holding` cannot raise it (CSH-080) — cargo test asserts the variant is unreachable from `sell_holding`.

**`OpenHoldingCommandError` — new variant** (CSH-061)

```text
OpeningBalanceOnCashAsset,
```

`to_open_holding_error` maps `OpeningBalanceDomainError::OpeningBalanceOnCashAsset` → `OpenHoldingCommandError::OpeningBalanceOnCashAsset`. The new domain variant lives in `src-tauri/src/context/account/domain/error.rs`.

**Two new Tauri commands and their error enums** (`use_cases/holding_transaction/api.rs`, CSH-021, CSH-031)

```text
#[derive(Serialize, Deserialize, Type)]
pub struct DepositDTO {
    pub account_id: String,
    pub date: String,
    pub amount_micros: i64,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Type)]
pub struct WithdrawalDTO {
    pub account_id: String,
    pub date: String,
    pub amount_micros: i64,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum RecordDepositCommandError {
    AccountNotFound,
    AmountNotPositive,
    DateInFuture,
    DateTooOld,
    Unknown,
}

#[derive(Debug, Serialize, Type, thiserror::Error)]
#[serde(tag = "code")]
pub enum RecordWithdrawalCommandError {
    AccountNotFound,
    AmountNotPositive,
    DateInFuture,
    DateTooOld,
    InsufficientCash { current_balance_micros: i64, currency: String },
    Unknown,
}

#[tauri::command] #[specta::specta]
pub async fn record_deposit(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: DepositDTO,
) -> Result<Transaction, RecordDepositCommandError> { … }

#[tauri::command] #[specta::specta]
pub async fn record_withdrawal(
    uc: State<'_, HoldingTransactionUseCase>,
    dto: WithdrawalDTO,
) -> Result<Transaction, RecordWithdrawalCommandError> { … }
```

The two private `to_record_deposit_error` / `to_record_withdrawal_error` mappers translate from `anyhow::Error` (mapping `TransactionDomainError`, `AccountDomainError`, `AccountOperationError`).

**Register the two new commands in `src-tauri/src/core/specta_builder.rs`** (B3) — add to the `collect_commands![]` block alongside `buy_holding`, `sell_holding`, etc.

### 2.6 `AccountDetailsResponse.total_global_value` (CSH-094)

`src-tauri/src/use_cases/account_details/orchestrator.rs`

- Extend the `AccountDetailsResponse` struct with `total_global_value: i64` (account-currency micros, ADR-001).
- Compute after the existing `total_cost_basis`/`total_unrealized_pnl` block:

  ```text
  let cash_term: i64 = active_holdings_iter
      .find(|h| h.asset_id == format!("system-cash-{}", account.currency.to_lowercase()))
      .map(|h| h.quantity)
      .unwrap_or(0);
  let market_term: i64 = details
      .iter()
      .filter(|d| d.asset_id != cash_asset_id) // exclude cash
      .map(|d| match d.current_price {
          Some(p) => (d.quantity as i128 * p as i128 / 1_000_000) as i64, // ADR-001 i128 intermediate
          None => 0, // CSH-094: unpriced contributes 0
      })
      .sum();
  let total_global_value = cash_term + market_term;
  ```

- Add to the `AccountDetailsResponse` constructor.
- Update inline tests to assert the new field. **CSH-093** (cash excluded from `total_cost_basis`) requires also filtering the cash asset out of the existing `total_cost_basis: i64 = details.iter().map(|d| d.cost_basis).sum()` line — same `cash_asset_id` predicate.

> The cash holding still appears in `holdings: Vec<HoldingDetail>` (CSH-090). The presenter on the frontend handles its visual rendering (no cost basis column, etc.).

### 2.7 Backend tests (test-writer-backend)

The contract is `docs/contracts/account-contract.md` (cash subset). Tests live inline (`#[cfg(test)]`) per `docs/test_convention.md` and `docs/backend-rules.md` B33–B36.

| Test file (path)                                                                         | Coverage                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/context/account/domain/transaction.rs` (`#[cfg(test)] mod tests`)         | `Deposit` and `Withdrawal` round-trip through strum (mirror `opening_balance_round_trips_through_strum`)                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `src-tauri/src/context/account/domain/account.rs` (`#[cfg(test)] mod tests`)             | `record_deposit_creates_cash_holding` (CSH-012, CSH-022); `record_withdrawal_rejects_when_no_cash_holding` (CSH-080); `record_withdrawal_rejects_oversize_amount` (CSH-080); `buy_holding_rejects_insufficient_cash` (CSH-041); `buy_holding_debits_cash_on_success` (CSH-040); `sell_holding_credits_cash_lazy_creates_holding` (CSH-050); `correct_transaction_replays_cash` (CSH-042); `cancel_deposit_rejects_when_replay_negative` (CSH-024); `cancel_withdrawal_always_succeeds` (CSH-034); `cash_holding_quantity_invariants_after_full_replay` (CSH-013) |
| `src-tauri/src/use_cases/holding_transaction/shared/ensure_cash_asset.rs` (inline tests) | `seeds_cash_asset_idempotently` (CSH-010, CSH-011); `seeds_cash_category_idempotently` (CSH-017); `concurrent_seed_returns_existing` (CSH-011)                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `src-tauri/src/use_cases/holding_transaction/orchestrator.rs` (inline tests)             | `record_deposit_happy_path`; `record_withdrawal_insufficient_cash_returns_typed_error`; `open_holding_rejects_cash_asset` (CSH-061)                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `src-tauri/src/use_cases/account_details/orchestrator.rs` (inline tests)                 | `total_global_value_includes_cash_and_priced_holdings` (CSH-094); `total_global_value_zero_when_no_cash_and_no_prices`; `total_cost_basis_excludes_cash` (CSH-093)                                                                                                                                                                                                                                                                                                                                                                                               |
| `src-tauri/tests/cash_tracking_crud.rs` (NEW integration test, real sqlite, B35)         | End-to-end: create account EUR, deposit 5000, buy AAPL 1600 EUR, sell half 900 EUR, withdraw 1000 EUR — assert running cash balance after each step; verify atomicity by simulating a row-level error mid-buy (mock that returns Err on second `HoldingUpserted`)                                                                                                                                                                                                                                                                                                |

Re-confirm "stubs red" by running `just test-rust` after writing tests but before implementation.

### 2.8 File map — backend changes summary

| File                                                                      | Change                                                                                                                                                                                                          |
| ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/migrations/{ts}_add_cash_transaction_types.sql`                | NEW (no-op stub if reviewer-sql agrees)                                                                                                                                                                         |
| `src-tauri/src/context/account/domain/transaction.rs`                     | Add `Deposit`, `Withdrawal` variants; new tests                                                                                                                                                                 |
| `src-tauri/src/context/account/domain/error.rs`                           | Add `InsufficientCash` to `AccountOperationError`; add `OpeningBalanceOnCashAsset` to `OpeningBalanceDomainError`                                                                                               |
| `src-tauri/src/context/account/domain/account.rs`                         | Add `record_deposit`, `record_withdrawal`; extend `recalculate_holding` for Deposit/Withdrawal; cash-replay in buy/sell/correct/cancel; `cash_asset_id`/`cash_holding_quantity` helpers                         |
| `src-tauri/src/context/account/service.rs`                                | Add `record_deposit`, `record_withdrawal` methods (event emit)                                                                                                                                                  |
| `src-tauri/src/context/account/api.rs`                                    | Add `InsufficientCash { current_balance_micros, currency }` to `TransactionCommandError`; mapping in `to_transaction_error`                                                                                     |
| `src-tauri/src/context/asset/service.rs`                                  | Add `create_asset_with_id`, `create_category_with_id` (used by `ensure_cash_asset`)                                                                                                                             |
| `src-tauri/src/use_cases/holding_transaction/shared/ensure_cash_asset.rs` | Replace stub with real impl (idempotent upsert of cash asset + category)                                                                                                                                        |
| `src-tauri/src/use_cases/holding_transaction/orchestrator.rs`             | Wire `ensure_cash_asset` in 4 existing methods; add `record_deposit`/`record_withdrawal`; reject Cash class in `open_holding`                                                                                   |
| `src-tauri/src/use_cases/holding_transaction/api.rs`                      | Add `DepositDTO`, `WithdrawalDTO`, `RecordDepositCommandError`, `RecordWithdrawalCommandError`; `record_deposit`/`record_withdrawal` Tauri commands; map `OpeningBalanceOnCashAsset` in `to_open_holding_error` |
| `src-tauri/src/core/specta_builder.rs`                                    | Register `record_deposit`, `record_withdrawal`                                                                                                                                                                  |
| `src-tauri/src/use_cases/account_details/orchestrator.rs`                 | Add `total_global_value` field + computation (CSH-094); exclude cash from `total_cost_basis` (CSH-093)                                                                                                          |
| `src-tauri/tests/cash_tracking_crud.rs`                                   | NEW integration test (real sqlite)                                                                                                                                                                              |

---

## 3. Frontend implementation plan

### 3.1 Bindings (auto-regen)

After `just generate-types` the following appear in `src/bindings.ts` automatically: `DepositDTO`, `WithdrawalDTO`, `RecordDepositCommandError`, `RecordWithdrawalCommandError`, the new `InsufficientCash` discriminant on `TransactionCommandError`, the new `OpeningBalanceOnCashAsset` discriminant on `OpenHoldingCommandError`, the new `Deposit`/`Withdrawal` discriminants on `TransactionType`, the new `total_global_value` field on `AccountDetailsResponse`, and `commands.recordDeposit` / `commands.recordWithdrawal`.

### 3.2 Account Details gateway (`src/features/account_details/gateway.ts`)

Add two methods (the cash flow lives where Buy/Sell live — same use-case boundary as the existing modals):

```ts
async recordDeposit(dto: DepositDTO): Promise<Result<Transaction, RecordDepositCommandError>> {
  return commands.recordDeposit(dto);
},

async recordWithdrawal(dto: WithdrawalDTO): Promise<Result<Transaction, RecordWithdrawalCommandError>> {
  return commands.recordWithdrawal(dto);
},
```

Update `gateway.test.ts` to cover the two new pass-through methods.

### 3.3 Modals (mirror `BuyTransactionModal` / `SellTransactionModal`)

**New file: `src/features/account_details/deposit_transaction/DepositTransactionModal.tsx`** (CSH-019, CSH-020, CSH-022, CSH-025)

Props: `{ isOpen, onClose, accountId, accountName, accountCurrency, onSubmitSuccess }`. No asset selector, no exchange rate, no fees, no unit price (CSH-020). Form id `deposit-transaction-form`, submit button `type="submit" form="deposit-transaction-form"` (E1, E3). Field IDs: `deposit-trx-account`, `deposit-trx-date`, `deposit-trx-amount`, `deposit-trx-note` (E2). Inline error block with `role="alert"` (E5).

**New file: `src/features/account_details/deposit_transaction/useDepositTransaction.ts`**

Mirrors `useBuyTransaction.ts` shape: `{ formData, error, isSubmitting, isFormValid, handleChange, handleSubmit }`. `formData = { date: today, amount: "", note: "" }`. `handleSubmit` calls `accountDetailsGateway.recordDeposit({ account_id, date, amount_micros: decimalToMicro(amount), note })`. On `Err({ code: "AmountNotPositive" })` → set inline error key. On `Err({ code: "DateInFuture" | "DateTooOld" })` → mapped key. Snackbar on success: `t("cash.deposit_recorded")` (CSH-025).

**New file: `src/features/account_details/deposit_transaction/useDepositTransaction.test.ts`** (colocated, F2).

**New file: `src/features/account_details/withdrawal_transaction/WithdrawalTransactionModal.tsx`** (CSH-030, CSH-031, CSH-032, CSH-035)

Same shape as Deposit but action wording is "Withdraw". Adds an InsufficientCash inline error path (CSH-081) — when backend returns `Err({ code: "InsufficientCash", current_balance_micros, currency })`, render: `t("cash.insufficient_cash_inline", { balance: presenter.formatAmount(current_balance_micros, currency) })`. Submit stays enabled (CSH-081 — user can amend).

**New file: `src/features/account_details/withdrawal_transaction/useWithdrawalTransaction.ts`** + colocated `.test.ts`.

> **Validators** (CSH-021, CSH-031): a tiny `src/features/account_details/shared/validateCashForm.ts` with `validateAmount(s: string): string | null` (positive decimal) and `validateDate(s: string): string | null` (TRX-020 bounds: not future, not pre-1900). Reused by both hooks.

### 3.4 Account Details header (`AccountDetailsView.tsx`) (CSH-019, CSH-094, CSH-095)

- Add `accountCurrency` to the data the view passes to modals — read from `useAppStore.accounts.find(a => a.id === accountId)?.currency`.
- Add **Deposit** button always visible next to "Add Transaction" / "Open Balance". Uses `t("account_details.action_deposit")`.
- Add **Withdraw** button — visible only when the cash holding row is visible (CSH-019: gated on `summary.hasCashHolding && cashRow.quantityMicro > 0`). The view-model exposes a `hasVisibleCashRow` boolean.
- Add **Global Value** stat to the header alongside `total_cost_basis` and `total_realized_pnl`. New entry in `summary` from `toAccountSummary`: `totalGlobalValue: string` (formatted) + `totalGlobalValueRaw: number`.
- New modal hooks: `const [depositOpen, setDepositOpen] = useState(false)` / `const [withdrawalOpen, setWithdrawalOpen] = useState(false)`. Both pass `onSubmitSuccess` calling `retry()` (same as existing buy/sell flow).

### 3.5 Cash row in the active holdings table (CSH-090, CSH-091, CSH-092, CSH-097)

- `presenter.toHoldingRow` needs a variant for the cash row: when `detail.asset_id === \`system-cash-\${accountCurrency.toLowerCase()}\``(the predicate stays at the presenter — the backend exposes the ID; frontend infers "is cash" from the`system-cash-`prefix), produce a`HoldingRowViewModel` with:
  - `quantity`: formatted as currency amount (use `microToFormatted` with 2 decimals + currency symbol — adopt `presenter.formatAmount(detail.quantity, accountCurrency)` helper).
  - `averagePrice: ""`, `costBasis: ""`, `realizedPnl: ""` (rendered as blank cells in the row; not "—" — CSH-091 says no columns).
  - `isCash: true` flag.
- `HoldingRow.tsx` reads the `isCash` flag and renders a thin variant: no Buy/Sell/Inspect buttons, only inline **Deposit** / **Withdraw** action buttons (CSH-091).
- The Cash row is sorted to the top of the active table, ahead of ACD-033 alphabetical sort (CSH-092). Implemented in `useAccountDetails` after the existing sort: `[cashRow, ...others]`.
- The Cash row is hidden when `quantity === 0` (CSH-097); driven by ACD-020's existing `quantity > 0` filter on the backend response — no frontend override needed once the backend filters per CSH-097.

### 3.6 No-cash banner (CSH-095)

- New component `src/features/account_details/account_details_view/NoCashBanner.tsx` — small banner row "No cash recorded yet" + primary button "Record a deposit". Rendered at the top of the active holdings table when `!hasVisibleCashRow` AND `!summary.isEmpty` (when summary.isEmpty the existing "No positions yet" empty state already takes over — CSH-095 banner only fires when other holdings exist or all-closed).
- The decision is owned by `useAccountDetails` so it can be presenter-tested.

### 3.7 ACD-034 cash exception (CSH-098)

- The presenter computes `summary.isEmpty` and `summary.isAllClosed` based on the backend's `holdings` (active) and `closed_holdings`. CSH-098: exclude the cash row from the active count.
- Update `toAccountSummary`: `const nonCashActive = response.holdings.filter(h => !isCashAsset(h.asset_id, response.account_currency))`. `isEmpty = total_holding_count === 0 || (nonCashActive.length === 0 && response.closed_holdings.length === 0)`. `isAllClosed = total_holding_count > 0 && nonCashActive.length === 0`.
- Note: `account_currency` is not currently in `AccountDetailsResponse` — read it from `useAppStore.accounts` instead, or add `account_currency: String` to the DTO. Decide during impl; reading from the store is simpler and avoids a backend change. Confirm with reviewer-arch.

### 3.8 Asset selector suppression (CSH-015, CSH-018)

| Surface                                                        | Filter rule                                                                                 | File                                                                                                                               |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Asset Manager table                                            | `assets.filter(a => a.class !== "Cash")` at the table level                                 | `src/features/assets/asset_table/AssetTable.tsx` (or its hook)                                                                     |
| Add Transaction asset combobox (Buy/Sell/OpeningBalance flows) | Same predicate before passing options to `ComboboxField`                                    | `src/features/transactions/add_transaction/useAddTransaction.ts` and any other call site that lists assets                         |
| Edit Transaction modal asset combobox                          | Same predicate                                                                              | `src/features/transactions/edit_transaction_modal/EditTransactionModal.tsx` (filter `assets` from store before mapping to options) |
| Open Balance modal asset combobox                              | Same predicate                                                                              | `src/features/account_details/open_balance/OpenBalanceModal.tsx`                                                                   |
| Categories list (CSH-017)                                      | Filter out `system-cash-category` alongside `default-uncategorized`                         | `src/features/categories/shared/presenter.ts` (or filter at the call site)                                                         |
| Transaction list (TXL) asset filter                            | **Do not filter** — Cash Asset must be selectable to surface Deposits/Withdrawals (CSH-101) | `src/features/transactions/transaction_list/useTransactionList.ts` (no change required)                                            |

### 3.9 Buy / Edit modals — InsufficientCash inline error (CSH-081)

`useBuyTransaction.ts` and `useEditTransactionModal.ts` already map `TransactionCommandError` codes to inline error keys. Add an `InsufficientCash` arm that reads the `current_balance_micros` + `currency` payload fields and renders the localised string from CSH-081. The submit button stays enabled. Marked `[unit-test-needed]` in the rules table — these are modified-function paths not covered by the new test stubs.

### 3.10 Transaction list — Deposit/Withdrawal type column (CSH-101)

`src/features/transactions/transaction_list/useTransactionList.ts` (or its presenter): when `transaction_type === "Deposit"` show "Deposit"; when `"Withdrawal"` show "Withdrawal" (TXL-023 cross-amend). Realized P&L column shows `—` (TXL-022). Quantity / Unit Price / Exchange Rate / Fees / Total Amount render their stored values per CSH-101. Backend already returns these rows when filtered by the Cash Asset (no change needed).

### 3.11 i18n keys

Add to `src/i18n/locales/en/common.json` and `fr/common.json`:

- `cash.deposit_recorded`, `cash.deposit_updated`, `cash.deposit_deleted`, `cash.withdrawal_recorded`, `cash.withdrawal_updated`, `cash.withdrawal_deleted` (CSH-025, CSH-035)
- `cash.no_cash_banner_message`, `cash.no_cash_banner_cta` (CSH-095)
- `cash.insufficient_cash_inline` with `{balance}` interpolation (CSH-081)
- `account_details.action_deposit`, `account_details.action_withdraw` (CSH-019)
- `account_details.total_global_value` (CSH-094)
- `transaction.type_deposit`, `transaction.type_withdrawal` (CSH-101 / TXL-023)
- `validation.amount_not_positive`, `validation.date_in_future`, `validation.date_too_old` (CSH-021/031 — likely already present; reuse)

### 3.12 Frontend tests (test-writer-frontend)

Colocated `.test.ts` per F2. The agent gets the contract + the `modified_functions` list:

| Test file                                                                                              | Covered rules                                                                                                                |
| ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| `src/features/account_details/gateway.test.ts` (existing — extend)                                     | Pass-through for `recordDeposit`, `recordWithdrawal`                                                                         |
| `src/features/account_details/deposit_transaction/useDepositTransaction.test.ts`                       | CSH-020, CSH-021, CSH-022, CSH-025                                                                                           |
| `src/features/account_details/withdrawal_transaction/useWithdrawalTransaction.test.ts`                 | CSH-030, CSH-031, CSH-032, CSH-035, CSH-081                                                                                  |
| `src/features/account_details/shared/validateCashForm.test.ts`                                         | CSH-021, CSH-031                                                                                                             |
| `src/features/account_details/shared/presenter.test.ts` (existing — extend)                            | toHoldingRow cash variant (CSH-091); toAccountSummary `totalGlobalValue` (CSH-094); CSH-098 isEmpty/isAllClosed exclude cash |
| `src/features/account_details/account_details_view/useAccountDetails.test.ts` (existing — extend)      | Cash row sorted to top (CSH-092); banner when quantity 0 (CSH-095); Deposit/Withdraw button gating (CSH-019)                 |
| `src/features/account_details/buy_transaction/useBuyTransaction.test.ts` (existing — extend)           | InsufficientCash inline error (CSH-081 / CSH-041)                                                                            |
| `src/features/transactions/edit_transaction_modal/useEditTransactionModal.test.ts` (existing — extend) | InsufficientCash inline error on edit (CSH-042 / CSH-051)                                                                    |
| `src/features/transactions/transaction_list/useTransactionList.test.ts` (existing — extend)            | Deposit/Withdrawal type column rendering (CSH-101)                                                                           |
| `src/features/assets/asset_table/useAssetTable.test.ts` (existing — extend)                            | Cash class filtered out of Asset Manager (CSH-015)                                                                           |

### 3.13 File map — frontend changes summary

| File                                                                                 | Change                                                                       |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| `src/features/account_details/gateway.ts`                                            | Add `recordDeposit`, `recordWithdrawal`                                      |
| `src/features/account_details/deposit_transaction/DepositTransactionModal.tsx`       | NEW (mirror BuyTransactionModal)                                             |
| `src/features/account_details/deposit_transaction/useDepositTransaction.ts`          | NEW                                                                          |
| `src/features/account_details/withdrawal_transaction/WithdrawalTransactionModal.tsx` | NEW                                                                          |
| `src/features/account_details/withdrawal_transaction/useWithdrawalTransaction.ts`    | NEW                                                                          |
| `src/features/account_details/shared/validateCashForm.ts`                            | NEW (validateAmount, validateDate)                                           |
| `src/features/account_details/shared/presenter.ts`                                   | Cash row variant; `totalGlobalValue`; CSH-098 exclusion                      |
| `src/features/account_details/shared/types.ts`                                       | Add cash-row fields to `HoldingRowViewModel`                                 |
| `src/features/account_details/account_details_view/AccountDetailsView.tsx`           | Deposit/Withdraw header buttons; Global Value; modals wiring; banner routing |
| `src/features/account_details/account_details_view/HoldingRow.tsx`                   | Cash variant rendering (Deposit/Withdraw inline actions)                     |
| `src/features/account_details/account_details_view/NoCashBanner.tsx`                 | NEW                                                                          |
| `src/features/account_details/account_details_view/useAccountDetails.ts`             | Sort cash row to top (CSH-092); expose hasVisibleCashRow                     |
| `src/features/account_details/buy_transaction/useBuyTransaction.ts`                  | Map `InsufficientCash` to inline error (CSH-081)                             |
| `src/features/account_details/index.ts`                                              | Re-export Deposit/Withdrawal modals                                          |
| `src/features/transactions/edit_transaction_modal/useEditTransactionModal.ts`        | Map `InsufficientCash` (CSH-042 / CSH-051)                                   |
| `src/features/transactions/transaction_list/useTransactionList.ts`                   | Type column → "Deposit" / "Withdrawal" (CSH-101)                             |
| `src/features/assets/asset_table/useAssetTable.ts` (or AssetTable.tsx)               | Filter out `class === "Cash"` (CSH-015)                                      |
| `src/features/transactions/add_transaction/useAddTransaction.ts`                     | Filter out cash assets in selector (CSH-018)                                 |
| `src/features/account_details/open_balance/OpenBalanceModal.tsx`                     | Filter out cash assets in selector (CSH-018)                                 |
| `src/features/categories/shared/presenter.ts`                                        | Filter `system-cash-category` (CSH-017)                                      |
| `src/i18n/locales/{en,fr}/common.json`                                               | New cash keys                                                                |

---

## 4. Cash-replay algorithm — precise spec (CSH-024 / CSH-033 / CSH-042 / CSH-051)

This is the single hardest part. State it explicitly so the test-writer and reviewer can match implementation against it.

**Where it lives**: `Account::replay_cash_holding(&mut self, cash_asset_id: &str)` — private method on the aggregate, called from each cash-affecting write path (deposit/withdrawal/buy/sell/correct/cancel).

**Inputs (all in-memory after the candidate write has been staged)**:

- `self.transactions: Vec<Transaction>` — the full transaction list for the account, **including** the candidate.
- The candidate change has already been applied to `self.transactions` (insert / update / delete). The replay's job is to validate the post-change state and recompute the cash holding.

**Algorithm**:

1. **Filter** `self.transactions` to the cash-affecting set:
   - `transaction_type` ∈ `{Deposit, Withdrawal}` (asset_id = cash_asset_id by construction)
   - OR `transaction_type` ∈ `{Purchase, Sell}` (any asset_id; cash side-effect is denominated in account currency).
   - Skip `OpeningBalance` (CSH-060 — does not touch cash).
2. **Sort** the filtered set by `(date ASC, created_at ASC)` — same ordering used by `recalculate_holding`.
3. **Iterate** with running cash balance `running: i64 = 0`:
   - `Deposit` → `running += t.total_amount`
   - `Sell` → `running += t.total_amount`
   - `Withdrawal` → if `running < t.total_amount` → return `AccountOperationError::InsufficientCash { current_balance_micros: running, currency: self.currency.clone() }`; else `running -= t.total_amount`
   - `Purchase` → same eligibility check + subtraction
4. **After the loop**:
   - If the filtered set is empty AND a cash holding exists in `self.holdings` → enqueue `AccountChange::HoldingDeleted` (TRX-034 / CSH-013).
   - Otherwise, construct `Holding::with_id` (or `Holding::new` if absent) with `quantity = running`, `average_price = 1_000_000`, `total_realized_pnl = 0`, `last_sold_date = None`. Enqueue `AccountChange::HoldingUpserted`.
5. Return the upserted (or deleted) cash holding state for the caller's bookkeeping.

**UoW boundary** (ADR-006): the entire sequence — asset holding upsert + cash holding upsert + transaction insert/update/delete — accumulates as `AccountChange` in `pending_changes` and is committed atomically by `AccountRepository::save` in a single sqlx transaction. Rollback on any error reverts everything. No `AppUnitOfWork` super-trait is needed; the aggregate is the UoW boundary.

---

## 5. Rules Coverage table (for spec-checker)

> Every CSH-NNN rule with its planned test location and code touch-point. `[unit-test-needed]` flags `frontend`/`frontend+backend` rules that modify existing functions (no contract entry).

| Rule    | Scope                                         | Test file                                                                                                                                                                   | Implementation file                                                                              |
| ------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| CSH-010 | backend                                       | `use_cases/holding_transaction/shared/ensure_cash_asset.rs`                                                                                                                 | same                                                                                             |
| CSH-011 | backend                                       | `use_cases/holding_transaction/shared/ensure_cash_asset.rs`                                                                                                                 | same                                                                                             |
| CSH-012 | backend                                       | `context/account/domain/account.rs`                                                                                                                                         | `Account::record_deposit`, `Account::sell_holding` (lazy create)                                 |
| CSH-013 | backend                                       | `context/account/domain/account.rs` (cancel_transaction tests)                                                                                                              | `Account::replay_cash_holding` cleanup branch                                                    |
| CSH-014 | backend                                       | `context/account/domain/account.rs` (deposit happy path asserts cash_asset_id matches currency)                                                                             | `Account::cash_asset_id`                                                                         |
| CSH-015 | frontend                                      | `features/assets/asset_table/useAssetTable.test.ts` `[unit-test-needed]`                                                                                                    | `useAssetTable.ts` filter                                                                        |
| CSH-016 | OUT OF SCOPE — see "Parallel work item" below |
| CSH-017 | backend                                       | `use_cases/holding_transaction/shared/ensure_cash_asset.rs`                                                                                                                 | same                                                                                             |
| CSH-017 | frontend                                      | `features/categories/shared/presenter.test.ts` `[unit-test-needed]`                                                                                                         | `presenter.ts` filter `system-cash-category`                                                     |
| CSH-018 | frontend                                      | `features/transactions/add_transaction/useAddTransaction.test.ts` `[unit-test-needed]`; `features/account_details/open_balance/useOpenBalance.test.ts` `[unit-test-needed]` | option-list filtering in each hook                                                               |
| CSH-019 | frontend                                      | `features/account_details/account_details_view/useAccountDetails.test.ts` `[unit-test-needed]`                                                                              | `AccountDetailsView.tsx` button gating                                                           |
| CSH-020 | frontend                                      | `features/account_details/deposit_transaction/useDepositTransaction.test.ts`                                                                                                | `DepositTransactionModal.tsx`, `useDepositTransaction.ts`                                        |
| CSH-021 | both                                          | `features/account_details/shared/validateCashForm.test.ts`; backend re-validation in domain                                                                                 | `validateCashForm.ts`; `Transaction::validate` (already enforces TRX-020 bounds)                 |
| CSH-022 | backend                                       | `context/account/domain/account.rs` `record_deposit_creates_cash_holding`                                                                                                   | `Account::record_deposit`                                                                        |
| CSH-023 | both                                          | `context/account/domain/account.rs` `correct_transaction_replays_cash`; `useEditTransactionModal.test.ts` `[unit-test-needed]`                                              | `Account::correct_transaction` (cash branch)                                                     |
| CSH-024 | backend                                       | `context/account/domain/account.rs` `cancel_deposit_rejects_when_replay_negative`                                                                                           | `Account::cancel_transaction` + `replay_cash_holding`                                            |
| CSH-025 | frontend                                      | `useDepositTransaction.test.ts` snackbar copy assertion                                                                                                                     | hook calls toast on success                                                                      |
| CSH-030 | frontend                                      | `useWithdrawalTransaction.test.ts`                                                                                                                                          | `WithdrawalTransactionModal.tsx`                                                                 |
| CSH-031 | both                                          | `validateCashForm.test.ts`; backend `record_withdrawal_rejects_oversize_amount`                                                                                             | `validateCashForm.ts`; `Account::record_withdrawal`                                              |
| CSH-032 | backend                                       | `context/account/domain/account.rs` (record_withdrawal happy path)                                                                                                          | `Account::record_withdrawal`                                                                     |
| CSH-033 | both                                          | `context/account/domain/account.rs` (correct_transaction_replays_cash for Withdrawal)                                                                                       | aggregate-level                                                                                  |
| CSH-034 | both                                          | `cancel_withdrawal_always_succeeds`                                                                                                                                         | aggregate-level                                                                                  |
| CSH-035 | frontend                                      | `useWithdrawalTransaction.test.ts` snackbar                                                                                                                                 | hook                                                                                             |
| CSH-040 | backend                                       | `buy_holding_debits_cash_on_success`                                                                                                                                        | `Account::buy_holding` + `replay_cash_holding`                                                   |
| CSH-041 | backend                                       | `buy_holding_rejects_insufficient_cash`                                                                                                                                     | aggregate-level (raises `AccountOperationError::InsufficientCash`)                               |
| CSH-042 | backend                                       | `correct_transaction_replays_cash`                                                                                                                                          | `Account::correct_transaction`                                                                   |
| CSH-043 | backend                                       | `cancel_transaction` test variant — purchase delete refunds cash, never violates                                                                                            | aggregate-level                                                                                  |
| CSH-050 | backend                                       | `sell_holding_credits_cash_lazy_creates_holding`                                                                                                                            | `Account::sell_holding`                                                                          |
| CSH-051 | backend                                       | `cancel_sell` cascading test                                                                                                                                                | aggregate-level                                                                                  |
| CSH-060 | backend                                       | `open_holding` for non-cash asset assertion (existing test path; cash exclusion covered by `replay_cash_holding` filter)                                                    | `Account::replay_cash_holding` (filter step skips OpeningBalance)                                |
| CSH-061 | backend                                       | `open_holding_rejects_cash_asset` (orchestrator inline test)                                                                                                                | `HoldingTransactionUseCase::open_holding` + `OpenHoldingCommandError::OpeningBalanceOnCashAsset` |
| CSH-080 | backend                                       | `Account::replay_cash_holding` tests + `record_withdrawal_rejects_when_no_cash_holding`                                                                                     | `AccountOperationError::InsufficientCash` + `to_transaction_error` mapping                       |
| CSH-081 | frontend                                      | `useBuyTransaction.test.ts` `[unit-test-needed]`, `useWithdrawalTransaction.test.ts`, `useEditTransactionModal.test.ts` `[unit-test-needed]`                                | hook error-mapping + form copy                                                                   |
| CSH-090 | backend                                       | `account_details/orchestrator.rs` `cash_holding_included_in_response`                                                                                                       | `AccountDetailsUseCase` (existing iterator over all holdings)                                    |
| CSH-091 | frontend                                      | `presenter.test.ts` cash variant; `useAccountDetails.test.ts` row-rendering check                                                                                           | `presenter.ts`, `HoldingRow.tsx`                                                                 |
| CSH-092 | frontend                                      | `useAccountDetails.test.ts` (cash sorted to top)                                                                                                                            | `useAccountDetails.ts`                                                                           |
| CSH-093 | backend                                       | `account_details/orchestrator.rs` `total_cost_basis_excludes_cash`                                                                                                          | `AccountDetailsUseCase`                                                                          |
| CSH-094 | backend                                       | `total_global_value_includes_cash_and_priced_holdings`, `total_global_value_zero_when_no_cash_and_no_prices`                                                                | `AccountDetailsUseCase`, `AccountDetailsResponse`                                                |
| CSH-094 | frontend                                      | `presenter.test.ts` `totalGlobalValue` field                                                                                                                                | `toAccountSummary`                                                                               |
| CSH-095 | frontend                                      | `useAccountDetails.test.ts` (banner gating)                                                                                                                                 | `NoCashBanner.tsx`, `AccountDetailsView.tsx`                                                     |
| CSH-097 | both                                          | `account_details/orchestrator.rs` (existing ACD-020 filter applies); `useAccountDetails.test.ts` `[unit-test-needed]`                                                       | existing backend filter + frontend re-render                                                     |
| CSH-098 | frontend                                      | `presenter.test.ts` (isEmpty/isAllClosed exclude cash)                                                                                                                      | `toAccountSummary`                                                                               |
| CSH-100 | backend                                       | service method emits `TransactionUpdated` (existing event — assert in service tests)                                                                                        | `AccountService::record_deposit`, `record_withdrawal`                                            |
| CSH-101 | frontend                                      | `useTransactionList.test.ts` `[unit-test-needed]`                                                                                                                           | `useTransactionList.ts` type-column rendering                                                    |

---

## 6. Parallel work item — CSH-016 (OUT OF SCOPE)

CSH-016 amends `update_asset`, `archive_asset`, `unarchive_asset`, `delete_asset` with a `CashAssetNotEditable` error variant when the target asset has `class = AssetClass::Cash`. This belongs to the **asset bounded context** and should ship as a small follow-up PR after this feature lands.

**Track it as**:

- `docs/contracts/asset-contract.md` upsert with the new error variant on each of the four enums.
- New PR `feat/cash-asset-readonly` against `feat/cash-tracking` once the latter is merged (or directly against `main`).
- Implementation: a single guard at the top of each `AssetService::update_asset` / `archive_asset` / `unarchive_asset` / `delete_asset` checking `existing.class == AssetClass::Cash`.

Add this entry to `docs/todo.md`:

```
- [ ] (backend + contract) — CSH-016: reject Cash class in update_asset / archive_asset / unarchive_asset / delete_asset; upsert docs/contracts/asset-contract.md.
```

---

## 7. Risks and assumptions

1. **`recalculate_holding` extension (Option A)** — adding Deposit/Withdrawal arms to the existing match. Confirm during impl that the existing tests for Purchase/Sell/OpeningBalance VWAP still pass after the change (no fall-through hazard).
2. **`account_currency` in `AccountDetailsResponse`** — preferred to add it to the DTO so the presenter doesn't need to dip into `useAppStore`. Decide during reviewer-arch.
3. **No-op migration** — reviewer-sql may push back. If so, drop the migration file and rely solely on `just prepare-sqlx` and the enum change.
4. **Sequencing of `ensure_cash_asset` calls** — placing the seed step in every `HoldingTransactionUseCase` method introduces an extra account-fetch round-trip (for currency). Acceptable for now; if hot-path measurements show contention, lift to `AccountService` returning `Account` from existing aggregate-load calls.
5. **`AccountService::create_asset_with_id` / `create_category_with_id`** — minor extension to AssetService. Inline tests cover the deterministic-ID path; a stale collision is exercised by `seeds_cash_asset_idempotently`.

---

## 8. Acceptance gate

Plan is complete when:

- All 28 CSH-NNN rules in section 5 trace to a test file + an implementation file.
- spec-checker reports green against `docs/spec/cash-tracking.md` and `docs/contracts/account-contract.md`.
- `just check-full` and `just test-rust` both green.
- `/visual-proof` screenshots committed for the deposit modal, withdrawal modal, no-cash banner, cash row, InsufficientCash inline error — light + dark.
- `e2e/cash/` exercises the happy path (deposit → buy → sell → withdraw) and the InsufficientCash rejection path.
- `ARCHITECTURE.md` updated with the cash methods, the `total_global_value` field, the new modals, and the gateway extension.
- `docs/todo.md` carries the CSH-016 follow-up entry (English).
