# Plan — Cash Tracking (CSH) and Holding-Transaction Refactor

**Branch**: `feat/dashboard` (misnamed — kept; rename in a separate PR if you like)
**Originated from**: `/start dashboard feature from roadmap doc` session, 2026-05-05
**Status at handoff**: Refactor Phase C complete (`just check-full` green); Phases D–F pending. Cash-tracking spec drafted with all spec-reviewer 🔴 findings addressed; needs `/contract` once the refactor lands.

---

## Why two features

This branch ended up combining two features — they ship in this order:

1. **Refactor**: consolidate transaction-recording into `use_cases/holding_transaction/`. No new behaviour. Prerequisite to (2).
2. **Cash Tracking (CSH)**: new spec — cash-as-Holding, Deposit/Withdrawal, Buy/Sell re-linked to cash, Global Value metric.

The original goal was a Portfolio Dashboard (PFD), which surfaced a data-model gap (no cash tracking). That gap blocks the dashboard. PFD remains paused in `docs/spec-index.md` as `planning`.

---

## Decisions locked in (do not re-litigate)

1. **Cash representation**: cash is a `Holding` whose asset is a system-seeded "Cash {CCY}" asset. One Cash Holding per `(account_id, account.currency)` pair. (Not a separate field on Account.)
2. **One Cash Holding per account** — in account currency. No per-asset-currency cash holdings. FX is already handled by `exchange_rate` + `fees` on Buy/Sell transactions; no new FX logic.
3. **Transaction types in v1**: Deposit, Withdrawal, plus Buy/Sell re-linked to cash. **No** Dividend in v1 (Phase 4 of roadmap, separate spec later).
4. **Insufficient cash on Buy/Withdrawal**: hard reject with `InsufficientCash { current_balance_micros, currency }` (CSH-080/CSH-081). Payload carries the balance so the form can display it without a follow-up fetch.
5. **Lazy cash-holding lifecycle**: created only on first Deposit or Sell (CSH-012). Buys and Withdrawals do **not** lazy-create — they fail eligibility. TRX-034 cleanup deletes the cash holding when no cash-affecting transactions remain (no never-delete invariant — early draft had one, simplified out).
6. **Cash Asset seeding**: lazy, inside `use_cases/holding_transaction/` via shared `ensure_cash_asset(currency)` helper. Account creation does NOT seed.
7. **Holding-lifecycle consolidation**: all transaction-recording operations move into `use_cases/holding_transaction/` (buy, sell, correct, cancel, open_holding). This is the prerequisite refactor.
8. **Refactor scope = Level 1**: move only. No error-enum split, no DTO/parameter consolidation. Both deferred to `docs/todo.md` ("(backend) — Consolidate transaction-recording command contracts").
9. **No migration**: app not yet shipped; local DBs reset via `just clean-db`.
10. **No ADR**: the lifecycle exception was simplified out; not needed.
11. **Price fallback in `total_global_value`**: a non-cash holding without a recorded `AssetPrice` contributes **0** (no fallback to `average_price`). User decision.
12. **Frontend impact of refactor**: zero — Tauri command signatures unchanged, `bindings.ts` regen produces only minor Specta-version artifacts (the bulk of any diff is whitespace).

---

## State at handoff

### What's done

**Refactor (Level 1) — Phases A, B, C complete**:
- New module `src-tauri/src/use_cases/holding_transaction/` with `mod.rs`, `api.rs`, `orchestrator.rs`, `shared/ensure_cash_asset.rs` (no-op stub).
- Five orchestrators: `OpenHoldingUseCase` (moved from `use_cases/open_holding/`, now deleted), plus new `BuyHoldingUseCase`, `SellHoldingUseCase`, `CorrectTransactionUseCase`, `CancelTransactionUseCase`. Each injects `AccountService` + `AssetService`; the new four delegate straight to AccountService methods (cash side-effect comes in CSH).
- Tauri command handlers for `buy_holding`, `sell_holding`, `correct_transaction`, `cancel_transaction`, `open_holding` moved into `use_cases/holding_transaction/api.rs`. DTOs (`BuyHoldingDTO`, `SellHoldingDTO`, `CorrectTransactionDTO`, `OpenHoldingDTO`) colocated.
- `TransactionCommandError` + `to_transaction_error` stay in `context/account/api.rs` (still used by `get_transactions`); `to_transaction_error` is now `pub(crate)` so the use-case handlers can import it.
- `core/specta_builder.rs` registers commands from the new module path.
- `lib.rs` constructs and `app_handle.manage()`s the four new use cases.
- `just check-full` passes.

**Cash Tracking spec — drafted + reviewed + critical findings addressed**:
- `docs/spec/cash-tracking.md` — full spec with new Tauri Commands section, 28 rules covering Cash Asset/Holding lifecycle, Deposit, Withdrawal, Buy/Sell re-link, OpeningBalance interaction, eligibility guard, display, reactivity. All 8 spec-reviewer 🔴 findings addressed. Open Questions all resolved (one tracking item remains for UL confirmation).
- `docs/spec-index.md` — CSH (active), PFD (planning, paused).
- Cross-spec amendments:
  - `docs/spec/account-details.md`: `total_global_value` field added to AccountDetailsResponse; ACD-020 + ACD-034 cross-references.
  - `docs/spec/financial-asset-transaction.md`: TRX-056 gains `OpeningBalanceOnCashAsset` error variant.
  - `docs/spec/transaction-list.md`: TXL-022/TXL-023 cover Deposit/Withdrawal types.
- `docs/ubiquitous-language.md` — 5 pending entries: `Deposit`, `Withdrawal`, `Cash Asset`, `Cash Holding`, `Global Value` (status: `pending` — must be confirmed by user before implementation).
- `docs/todo.md` — entry for the deferred contract consolidation (per-command error enums + DTO unification, post-refactor).

### What's NOT done

**Refactor — Phases D, E, F**:
- **D — Docs**: `ARCHITECTURE.md` not yet updated (move buy/sell/correct/cancel listings from "Bounded Contexts → Account" to a new "Use Cases → holding_transaction" section). `docs/contracts/account-contract.md` not yet updated (the existing note "All commands below live in `context/account/` except `open_holding`..." needs to be revised to reflect that all transaction-recording commands now live in `use_cases/holding_transaction/`).
- **E — Reviewers**: `reviewer-arch` + `reviewer-backend` agents not yet run on the refactor diff.
- **F — Commit**: refactor is currently uncommitted on this branch; will be committed (alongside the cash spec drafts) as part of the handoff push.

**Cash Tracking implementation** (Workflow A continuation, after refactor):
- `/contract` — derive `docs/contracts/cash-tracking-contract.md` from the spec.
- `contract-reviewer` agent.
- `feature-planner` agent — produce `docs/plan/cash-tracking-feature-plan.md` (this file is the *meta* plan; the feature-planner output is the implementation plan per CLAUDE.md Workflow A).
- `test-writer-backend`, backend implementation, `just format`, `reviewer-backend`, `just generate-types`, smart-commit.
- `test-writer-frontend`, frontend implementation, `just format`, `reviewer-frontend`, `/visual-proof`, smart-commit.
- `test-writer-e2e`, E2E tests, smart-commit.
- `reviewer-arch` (always) + `reviewer-sql` (if migrations) + `reviewer-infra` (if infra changes).
- Update `ARCHITECTURE.md` + `docs/todo.md`.
- `spec-checker`, smart-commit, `/create-pr`.

---

## Resumption — exact steps for the cloud session

### Step 1 — finish the refactor (D, E, F)

1. **Update `ARCHITECTURE.md`**: move the listing of `buy_holding`, `sell_holding`, `correct_transaction`, `cancel_transaction` from the Account context section to a new "Holding Transaction (`use_cases/holding_transaction/`)" subsection under Use Cases. Mention the four new use-case structs and the `ensure_cash_asset` stub. Keep `get_transactions`, `get_asset_ids_for_account`, account CRUD in the Account context section.

2. **Update `docs/contracts/account-contract.md`**: the note at the top reads `> All commands below live in context/account/ except open_holding...`. Replace with a clear breakdown:
   - `context/account/`: get_accounts, add_account, update_account, delete_account, get_asset_ids_for_account, get_transactions
   - `use_cases/holding_transaction/`: buy_holding, sell_holding, correct_transaction, cancel_transaction, open_holding
   No signature changes; this is a location-note only update.

3. **Run reviewers**:
   - `reviewer-arch` agent — give it the diff scope: `src-tauri/src/use_cases/holding_transaction/`, `src-tauri/src/context/account/api.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/core/specta_builder.rs`, `src-tauri/src/use_cases/mod.rs`. Verify cross-context isolation, gateway pattern, factory methods, data flow.
   - `reviewer-backend` agent — same scope. Verify Clippy patterns, anyhow error handling, no `unwrap()` in production paths, inline tests.
   - Fix any findings. Re-run `just check-full`.

4. **`/smart-commit`** for the refactor (if not already committed by the handoff push).

### Step 2 — resume the cash work

5. **`/contract`** (the kit skill, not the agent). Derives `docs/contracts/cash-tracking-contract.md` from `docs/spec/cash-tracking.md`. Spec already declares the new commands explicitly in the "Tauri Commands" section, so derivation should be straightforward.

6. **`contract-reviewer`** agent — verify contract vs spec coverage, error exhaustiveness, type correctness.

7. **`feature-planner`** agent — produce `docs/plan/cash-tracking-feature-plan.md` mapping CSH-NNN rules to DDD layers and CLAUDE.md workflow steps. Use this current meta-plan as input.

8. **Confirm UL entries** with the user — `Deposit`, `Withdrawal`, `Cash Asset`, `Cash Holding`, `Global Value` are all `pending` in `docs/ubiquitous-language.md`. They must be marked `confirmed` by the user before implementation begins (per the explicit warning at the top of the UL doc).

9. **Phase 2 (backend)**:
   - `test-writer-backend` agent — generate failing tests from `docs/contracts/cash-tracking-contract.md`. Cover at minimum: deposit/withdrawal happy path, eligibility rejects on Buy/Withdrawal with no cash, OpeningBalance reject on Cash Asset, lazy cash-asset seeding via `ensure_cash_asset`, total_global_value computation including the "no price = 0" rule.
   - Implement: extend `TransactionType` enum with `Deposit` + `Withdrawal`; migrate `transactions` table; implement `ensure_cash_asset` properly (idempotent upsert); add `record_deposit` and `record_withdrawal` use cases under `use_cases/holding_transaction/`; wire them through `api.rs`; extend `TransactionCommandError` with `InsufficientCash { current_balance_micros, currency }`; extend `OpenHoldingCommandError` with `OpeningBalanceOnCashAsset`; cross-amend ACD `AccountDetailsResponse` with `total_global_value`.
   - `just format`, `reviewer-backend`, `just generate-types`, smart-commit.

10. **Phase 3 (frontend)**:
    - `test-writer-frontend` agent — gateway tests + RTL component tests for Deposit/Withdrawal modals + Account Details header changes.
    - Implement: `DepositTransactionModal` + `WithdrawalTransactionModal` (mirror existing `BuyTransactionModal`/`SellTransactionModal` patterns); Account Details header gets Deposit/Withdraw buttons (CSH-019) + "Global Value" stat; Cash row in active holdings (CSH-090–CSH-098); Asset Manager filters out Cash Assets (CSH-015/CSH-018); inline `InsufficientCash` errors (CSH-081); "No cash recorded" empty state (CSH-095).
    - `just format`, `reviewer-frontend`, `/visual-proof` (capture all states light + dark), smart-commit.

11. **Phase 4 (E2E + closure)**:
    - `test-writer-e2e` agent.
    - `reviewer-arch` (whole feature), `reviewer-sql` (cash migration).
    - Update `ARCHITECTURE.md` + `docs/todo.md`.
    - `spec-checker` agent — verify all CSH-NNN rules and contract commands covered.
    - smart-commit, `/create-pr`.

### Step 3 — resume PFD (after CSH ships)

12. Once cash is in, lift PFD from `planning` to `active` in `docs/spec-index.md` and run `/spec-writer` for the portfolio dashboard. The dashboard's "Global Value", apport, and base-100 metrics now have all the data they need.

---

## Quick file map

```
docs/
├── plan/cash-tracking-plan.md            # <— this file
├── spec/cash-tracking.md                 # CSH spec (drafted, review-clean)
├── spec/account-details.md               # cross-amended (ACD-020, ACD-034, AccountDetailsResponse)
├── spec/financial-asset-transaction.md   # cross-amended (TRX-056)
├── spec/transaction-list.md              # cross-amended (TXL-022, TXL-023)
├── spec-index.md                         # CSH active, PFD planning
├── ubiquitous-language.md                # 5 pending entries — user must confirm
├── todo.md                               # contract-consolidation entry added
└── (no PFD spec yet)

src-tauri/src/
├── context/account/api.rs                # buy/sell/correct/cancel handlers REMOVED; get_accounts/get_transactions/etc remain; to_transaction_error now pub(crate)
├── core/specta_builder.rs                # commands re-pointed to holding_transaction
├── lib.rs                                # 4 new use-case structs constructed + managed
├── use_cases/mod.rs                      # holding_transaction replaces open_holding
├── use_cases/open_holding/                # DELETED
└── use_cases/holding_transaction/         # NEW
    ├── mod.rs
    ├── api.rs
    ├── orchestrator.rs                   # 5 UseCase structs
    └── shared/
        ├── mod.rs
        └── ensure_cash_asset.rs          # stub

src/bindings.ts                            # auto-regen; minor Specta-version artifacts only
```

---

## Open conversational threads — none

All blocking decisions have been made and documented above. The cloud session can proceed straight to Step 1.

---

## Outstanding `[ ]` from cash spec Open Questions

- `[ ]` UL — five pending entries need explicit user confirmation (Deposit, Withdrawal, Cash Asset, Cash Holding, Global Value). Non-blocking for `/contract`; blocking for implementation.
