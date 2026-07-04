# Next-batch plan — branch `next`

Session batch of 9 tasks collected by interview on 2026-07-04. All commits land on
the `next` branch; one commit per small task, several (BE/FE) for the bigger ones.

Status legend: ⬜ pending · 🔎 analyzed · ✅ shipped

## Task list

### T1 — Market-price refresh: shell progress indicator (+ coalesced refreshes) ✅

- **1a (required)**: progress indicator mounted in the shell, visible everywhere
  while market prices refresh.
- **1b (optional)**: coalesce the re-fetch cascade that makes the UI flick.
- **Analysis**: a bulk fetch runs in `use_cases/asset_price_fetch/dispatcher.rs`
  (per-asset loop, 250 ms spacing) and emits one payload-less `AssetPriceUpdated`
  per success + one final `AssetPriceFetchCompleted {ok, skipped, unpriced}`.
  Three hooks re-fetch on every `AssetPriceUpdated` (`useAccountDetails.ts:71`,
  `useAccountPerformance.ts:95`, `useAccountSummaries.ts:61`) → N assets ×
  3 views = 3N backend round-trips per refresh; that cascade is the flicker.
  The shell has no bottom bar (only the bottom-center snackbar); the header
  (`src/features/shell/Header.tsx`) is the natural mount. No progress-bar
  component exists yet; `useAppStore` has no fetch-in-progress state.
- **Direction (pending user answers)**: emit fetch progress from the dispatcher
  (start + per-asset done/total), track it in the store, render a thin linear
  progress bar in the shell; while a fetch is active, views suppress per-event
  re-fetch and do a single re-fetch on `AssetPriceFetchCompleted`.

### T2 — Holding weight % in account details ✅

- Per-line weight = line value in account currency ÷ `total_global_value`,
  rendered right after the Current Value column.
- **Analysis**: `HoldingDetail` (BE `use_cases/account_details/orchestrator.rs`)
  exposes `current_price` in **asset** currency only; the FX-converted
  account-currency value exists transiently while the BE sums
  `total_global_value` but is not exposed per holding → add a per-holding
  account-currency market-value field to the DTO, then compute weight in the
  presenter (`features/account_details/shared/presenter.ts`,
  `AccountSummaryViewModel.totalGlobalValueRaw` is already there).
  Percent formatting reuses the `microToFormatted(x, 2)%` pattern.
- **Assumption**: the cash line gets a weight too; denominator is
  `total_global_value` (cash included); closed holdings show none.

### T3 — Net cash input since inception in the account-details header ✅

- Header displays net cash input from day one (deposits − withdrawals).
- **Analysis**: not in `AccountDetailsResponse` today. The deposits/withdrawals
  filter logic already exists in `use_cases/account_performance/orchestrator.rs`
  (PRF-070 `cash_flow`). Add `total_net_cash_input: i64` (account currency) to
  `AccountDetailsResponse`, surface in the header next to
  `total_global_value` / `total_management_fees`.

### T4 — Account parameter gating the % management-fee mechanism ✅

- New boolean on the account, **default false** for new accounts; migration
  backfills **true** for existing accounts. False → % fee UI disabled.
- **Analysis**: Account aggregate at `context/account/domain/account.rs`
  (factories `new`/`with_id`/`restore`/`restore_with_positions`; row mapping in
  `repository/account.rs`; `add_account`/`update_account` DTO commands).
  Migration pattern: `ALTER TABLE accounts ADD COLUMN … NOT NULL DEFAULT …`
  (cf. `202604250001_add_currency_to_accounts.sql`). Surfaces to gate:
  one-off `ManagementFeeModal` (Percent header button,
  `AccountDetailsView.tsx:196-205`), recurring `FeeScheduleModal`
  (HoldingRow "Manage" button), startup catch-up
  (`features/shell/fee_generation/useFeeGeneration.ts` →
  `apply_due_fee_deductions`). FE forms: `features/accounts/shared/AccountForm.tsx`
  - add/edit modals + gateway.
- Open points → user questions: BE enforcement scope, disabled-UI presentation,
  spec treatment.

### T5 — Fee % indicator on the holding line ✅

- Show the configured fee rate on the asset line when a schedule exists.
- **Analysis**: schedules are one-per-(account, asset)
  (`context/account/domain/fee_schedule.rs`, `annual_rate_percent_micros`,
  `active`) so no multiplicity issue; one-off fees have no schedule and don't
  count. `AccountDetailsResponse` doesn't expose schedule info per holding →
  extend `HoldingDetail` with the active schedule's annual rate (Option), or
  reuse `get_fee_schedule` per row (N+1 — rejected). Show only `active` schedules.
- Open point → user question: display placement/format.

### T6 — Visual-proof the Management Fees column + header total (todo closure) ✅

- Capture `HoldingRow` Management Fees column + `total_management_fees` header
  via the container preview harness (mockIPC + `createMemoryHistory` +
  `useAppStore.setState`). Runs **after** T2/T3/T5 so one capture covers the
  new columns too. Closes the `docs/todo.md` entry.

### T7 — Techdebt: unify date display style ✅

- **Analysis**: three styles today — `formatIsoDateNumeric`
  (`src/ui/format/date.ts`, used by `TransactionTable.tsx:95`), short-month
  `formatIsoDate` (`features/account_details/shared/formatDate.ts`, used by
  `ClosedHoldingRow.tsx:45`, `useAccountDetailsView.ts:259`,
  `PriceHistoryModal.tsx:155,223`), raw ISO (`CurrencyRatesView.tsx:159`).
  E2E date _input_ helper (`e2e/helpers/date.ts` `isoToDisplayDate`) targets
  DateField inputs and is unaffected by display-rendering changes, but rendered
  date assertions (e.g. `e2e/currency/currency_rates.test.ts:82-85`) must be
  checked after the change.
- Open point → user question: which convention wins.

### T8 — Techdebt: Dialog stable F25 ids ⬜

- `Dialog.tsx` surface (line 54) gets an `id` prop; the close buttons in
  `Dialog.tsx:72` **and** `FormModal.tsx:46` (both use
  `data-testid="modal-close-btn"`) move to a stable `id`; update
  `e2e/helpers/modal.ts` `dismissLeftoverModal`. 11 direct Dialog consumers +
  ConfirmationDialog pass-through. Mechanical fanout, single commit.

### T9 — Techdebt: M3 debit/credit color aliases ⬜

- Tokens live in `src/ui/global.css`. Add `--color-m3-loss`/`--color-m3-gain`
  (or debit/credit) aliases to the same palette values; migrate the ~10
  financial-polarity sites (`TransactionTable.tsx` cash out/in + realized P&L,
  `PnlCell.tsx`, `HoldingRow.tsx`, `AccountTable.tsx`,
  `SellTransactionModal.tsx` hint) — genuine error/validation sites keep
  `error`. Update the class-name test assertions.

### T10 — Interest on an asset line (assurance-vie euro fund) 🔎

- Use case: a euro-denominated fund line (French assurance vie) yields a yearly
  interest whose rate varies per year. How to record it?
- **Design options**:
  - **(A) Capitalized interest on the asset line** — new `Interest` transaction
    type: quantity increase at zero cost (FreeShares mechanics — the deposit
    mirror of the ManagementFee deduction), entered as a % of the current
    holding (computed) or a direct amount, dated (typically 31 Dec). Matches how
    euro funds actually credit interest (capitalized into the fund, not paid
    out). A dedicated type keeps it distinguishable from FreeShares for
    reporting (an "interest received" figure, like dividends).
  - **(B) Cash-line interest** — a Deposit-like interest transaction on the
    account cash line. Fits interest-bearing _cash_, not a euro-fund line
    (the fund amount itself must grow).
  - **(C) Reuse FreeShares as-is** — zero code, but semantics and reporting
    conflate free shares with interest.
- New transaction type = new business rules → spec-worthy (same question as T4:
  lightweight spec amendment vs full cycle).

## Ordering & commit strategy

1. **Account-details cluster**: T2 → T3 → T5 (shared DTO/presenter/view files;
   one commit each, BE+FE together per task since each is small).
2. **T6** — one container-harness capture right after the cluster.
3. **T4** — spec amendment first, then migration + BE enforcement commit, then
   FE gating commit.
4. **T1** — BE progress events + FE shell bar & coalescing (1–2 commits).
5. **T7 / T8 / T9** — independent refactor commits, any order.
6. **T10** — last (new spec + new transaction type; BE commit + FE commit).
   Slips to the next session if this one runs long.

## Decisions (user-confirmed 2026-07-04)

1. **T1**: determinate progress — dispatcher emits done/total, thin progress bar
   in the shell; while a fetch is active, views suppress per-asset re-fetches
   and reload once on `AssetPriceFetchCompleted` (1b included).
2. **T4**: UI **+ BE** enforcement — backend rejects `record_management_fee` /
   `create_fee_schedule` for disabled accounts and the startup catch-up skips
   their schedules. Disabled look: affordances **and** the Management Fees
   column + header total are hidden.
3. **Spec treatment**: lightweight amendments — new rules appended to the
   management-fee spec (T4) and a compact new spec for T10; no full
   contract/planner cycle; spec-checker at the end.
4. **T5**: rate renders inside the Management Fees cell — "123.45 · 1.5%",
   rate shown only when an active schedule exists.
5. **T7**: locale-numeric everywhere; short-month helper retired; raw-ISO
   straggler fixed; E2E rendered-date assertions re-checked.
6. **T10**: design A — new `Interest` transaction type, quantity increase at
   zero cost (% of holding or direct amount), **cash line included** as a valid
   target (interest-bearing cash). Spec must pin the cash-line cost-basis rule
   (no phantom unrealized P&L from zero-cost mechanics).
