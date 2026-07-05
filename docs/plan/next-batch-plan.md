# Next-batch plan — 2026-07-05

Batch branch: `next`. One or more surgical commits per task; direct merge via
`just merge`; housekeeping commit before release; release at the end (gate the
tag on CI E2E green, per L-009).

Decisions locked with the user:

- **T1 metric**: per-line windowed performance = position return via Simple
  Dietz (`metric_for_span` shape). Buys/sells are the only external flows;
  dividends received in the window are added to the gain (DIV-071 semantics);
  interest credits / free shares / fee deductions flow through quantity and
  need no special casing. "Since start" keeps today's cost-basis formula.
- **T4 fees**: the typed Total is the all-in broker debit (includes fees).
  Buy: `unit_price` derived from `(total − fees) / qty`; sell: `(total + fees)
/ qty`. The backend accepts the typed total as ground truth so the stored
  ledger total is exact (no floor-division round-trip loss).
- **T5 currency**: All-accounts aggregation is fixed EUR via FXR
  carry-forward rates.
- **T6 content**: bundled `CHANGELOG.md` (vite `?raw` import), sections
  between last-seen and current version, English-only by design.

Execution order: T3 → T7 → T8 → T9 → T10 → T4 → T1 → T2 → T5 → T6 →
housekeeping → release. Rationale: small domain/techdebt tasks warm the
touched areas first; the performance cluster (T1/T2/T5) shares machinery and
runs consecutively after T10 lands in the same file it touches.

---

## T3 — Bank brand name on account (ACC amendment)

Backend (commit 1):

- `src-tauri/migrations/2026070500NN_add_bank_name_to_accounts.sql` —
  `ALTER TABLE accounts ADD COLUMN bank_name TEXT NOT NULL DEFAULT '';`
- `src-tauri/src/context/account/domain/account.rs` — `bank_name: String`
  (empty = unset) on `Account`; thread through `new` / `with_id` / `restore` /
  `restore_with_positions`; no validation beyond trim.
- `src-tauri/src/context/account/service.rs` — `create` / `update` gain the
  param; repository row mapping.
- `src-tauri/src/context/account/api.rs` + `just generate-types`.
- Spec: `docs/spec/account.md` — new ACC rule (bank name optional metadata,
  shown in the accounts table).

Frontend (commit 2):

- `src/features/accounts/shared/AccountForm.tsx` — TextField
  `{idPrefix}-bank-name` after name.
- add/edit hooks (`useAddAccount` / edit) — field threading.
- `src/features/accounts/account_table/AccountTable.tsx` +
  `useAccountTable.ts` — sortable "Bank" column after Name (nullable-last
  sort per ACC-008 pattern); stable header id.

## T7 — Per-asset `interest_bearing` flag (INT/AST amendment, todo closure)

Backend (commit 1):

- Migration: `ALTER TABLE assets ADD COLUMN interest_bearing INTEGER NOT NULL
DEFAULT 0;` (opt-in; existing assets stay off).
- `src-tauri/src/context/asset/domain/asset.rs` — field + `new` / `with_id` /
  `restore` / `update_from`; `src-tauri/src/context/asset/service.rs` DTOs;
  api + bindings.
- Spec: `docs/spec/interest-credit.md` — eligibility rule (cash line always
  eligible; non-cash assets only when `interest_bearing`); `docs/spec/asset.md`
  field rule; close the `docs/todo.md` entry.

Frontend (commit 2):

- Asset form checkbox (`{idPrefix}-interest-bearing`) in
  `src/features/assets/` add/edit.
- `src/features/account_details/account_details_view/useAccountDetailsView.ts`
  — `interestEligibleHoldings` filter gains
  `isCashAsset(h.asset_id) || h.interest_bearing`.

## T8 — Fee-schedule N+1 (techdebt)

One commit:

- `src-tauri/src/context/account/domain/fee_schedule.rs` — trait method
  `get_active_by_account(account_id)`; SQL `WHERE active = 1 AND account_id = ?`
  in the repository impl; `AccountService::list_active_fee_schedules_for_account`.
- `src-tauri/src/use_cases/account_details/orchestrator.rs` — FEE-074 map
  uses the scoped query (drop the in-memory filter).
- `src-tauri/src/use_cases/fee_generation/orchestrator.rs` — FEE-078 loop:
  collect unique account ids, load each account once, then apply schedules
  (1 + A queries instead of 1 + S).
- Remove the techdebt entry.

## T9 — FormModal `id` prop + edit-mount error presenter (techdebt)

One commit:

- `src/ui/components/modal/FormModal.tsx` — `id?: string` on the container
  (mirror `Dialog.tsx`); consumers pass their form-scoped id (16 call sites,
  mechanical).
- `src/features/transactions/shared/presenter.ts` — export a load-failure
  mapping (reuse `transactionMutationErrorToI18n` or a thin
  `transactionLoadErrorToI18n`).
- `src/features/shell/{FreeShares,ManagementFee,Interest}EditModalMount.tsx`
  — replace hardcoded `error.Unknown` with the presenter mapping.
- Remove the techdebt entry.

## T10 — YTD rate-map over-fetch (techdebt)

One commit:

- `src-tauri/src/use_cases/shared/valuation.rs` — `load_rate_map_for_dates(
currency_service, priced_assets, account_currency, dates: &[NaiveDate])`;
  `compute_current_ytd_pct` uses it with `[year_baseline, today]` instead of
  the full period-end sweep via `load_rate_map`.
- Remove the techdebt entry.

## T4 — Buy/sell total-entry mode (TRX/SEL amendment)

Backend (commit 1):

- `BuyHoldingDTO` / `SellHoldingDTO` gain `total_amount: number | null`
  (micro-units, account currency, all-in). When set, the domain derives
  `unit_price = round(((total ∓ fees) × M × M) / (qty × rate))` and stores the
  typed total verbatim (skip `compute_purchase_total` / `compute_sell_total`);
  validation: derived price ≥ 0, total > 0, sell total + fees > 0.
- `src-tauri/src/context/account/domain/account.rs` (`apply_purchase` /
  `apply_sell` entry points) + `use_cases/holding_transaction` DTO threading +
  api + bindings.
- Spec: new TRX/SEL rules (total-entry mode, derivation, exact-total storage).

Frontend (commit 2):

- `src/features/account_details/buy_transaction/` and `sell_transaction/` —
  entry-mode toggle (unit-price mode ⟷ total mode). In total mode the Total
  Amount field becomes editable, Unit Price becomes the read-only derived
  display; fees stay editable. `src/lib/microUnits.ts` derivation helper.
- `src/features/transactions/shared/validateTransaction.ts` — total-mode
  validation path.
- Edit modal untouched: corrections keep showing stored qty/price/fees
  (the stored decomposition is the record; note in spec).

## T1 — Per-line performance period selector (ACD amendment)

Backend (commit 1):

- `src-tauri/src/use_cases/account_details/orchestrator.rs` — in
  `get_account_details_live`, one pass computes per-holding windowed returns
  for all periods (YTD, 1y, 2y, 5y, 10y) using the already-loaded
  `priced_assets` + a rate map covering the window-start dates + today:
  per asset, `start_value` = quantity-at-window-start × price-as-of ×
  rate-as-of; flows = Purchase/Sell of that asset inside the window (converted
  amounts); gain += dividends received in window (DIV-071); Simple Dietz
  denominator per `metric_for_span` shape. New wire struct
  `HoldingPeriodPerformance { ytd, one_year, two_years, five_years, ten_years:
Option<i64> }` on `HoldingDetail`; `None` when denominator 0 or price/rate
  missing (FXR-034). Cash line: all `None`. As-of view: all `None` (selector
  hidden in as-of mode).
- Spec: new ACD rules for the windowed per-line metric.

Frontend (commit 2):

- `AccountDetailsView.tsx` title group — period `SelectField`
  (`account-details-perf-period`, options: YTD / 1y / 2y / 5y / 10y / since
  start; default since start), hidden in as-of view; persisted per account via
  a `src/lib/perfPeriodStorage.ts` adapter (mirrors `perfViewModeStorage`).
- `HoldingRow.tsx` / presenter — performance cell reads the selected period
  ("—" when `None`); since start keeps `performancePct`.

## T2 — Asset selector on account performance view (PRF amendment)

Backend (commit 1):

- `src-tauri/src/use_cases/account_performance/orchestrator.rs` —
  `get_account_performance(account_id, asset_id: Option<String>)`. With an
  asset scope: transactions filtered to that asset for flows/dividends,
  `end_value_as_of` restricted to the single holding (new scoped helper in
  `valuation.rs` reusing `PricedAsset::price_as_of`), cash rows excluded from
  flows. Same `PerformancePeriod` shape.
- Spec: PRF scoping rules.

Frontend (commit 2):

- `AccountPerformancePage.tsx` — asset `SelectField`
  (`account-performance-asset-selector`, default "All assets") next to the
  existing year selector; hook refetches on change.

## T5 — Global performance view (new spec, trigram GPF)

Backend (commit 1):

- New use case `src-tauri/src/use_cases/global_performance/` (orchestrator +
  api + mod, registered in `core/specta_builder.rs`):
  `get_global_performance(account_id: Option<String>, asset_id:
Option<String>)`. All-accounts path: per account, reuse the account
  valuation pass (priced assets + rate maps per account currency), convert
  each account's period end-values and flows to EUR at the period-end /
  flow-date rate, sum, then Simple Dietz on the EUR series. Account-scoped
  path delegates to the T2 machinery. Response mirrors
  `AccountPerformanceResponse` (currency = "EUR", `month_view_available` =
  AND of member accounts' month availability).
- Spec: new `docs/spec/global-performance.md` (GPF) + `docs/spec-index.md`
  registration.

Frontend (commit 2):

- New route `/performance` (router.tsx) + page under
  `src/features/global_performance/` (gateway, hook, page reusing
  `AccountValueChart` + the PRF table shape); account selector
  (`global-performance-account-selector`, "All accounts" default) × asset
  selector (populated from the selected scope); entry button in the accounts
  view header (`accounts-performance`).

## T6 — What's-new dialog (new spec, trigram WNW)

One commit (FE-only):

- Vite `?raw` import of `CHANGELOG.md` (repo root) + parser
  `src/features/whats_new/parseChangelog.ts` — extract `## [x.y.z]` sections
  strictly newer than last-seen and ≤ current (stacks skipped versions).
- `src/lib/whatsNewStorage.ts` — `whats_new_last_seen_version` localStorage
  adapter (get/set).
- `src/features/whats_new/WhatsNewDialog.tsx` + shell mount in `AppShell.tsx`
  (store-driven, `UnpricedPricesModalMount` pattern): on init, once
  `appVersion` resolves, show when last-seen ≠ current AND last-seen is
  non-null (fresh installs seed silently — no dialog on first-ever launch);
  dismissing writes the current version. `Dialog` chrome, id
  `whats-new-dialog`; i18n chrome keys (en/fr), body English-only by design.
- Spec: `docs/spec/whats-new.md` (WNW) + spec-index registration.

---

## Cross-cutting

- Reviewers per cluster: `reviewer-backend` + `reviewer-arch` on every BE
  commit batch; `reviewer-sql` on T3/T7 migrations; `reviewer-frontend` on FE
  batches; `/review-triage` after every batch. `spec-checker` on the amended/new
  specs (ACC, INT/AST, TRX/SEL, ACD, PRF, GPF, WNW) before housekeeping.
- `/visual-proof` for every changed surface: AccountForm, AccountTable,
  asset form, buy/sell modals, account-details header + rows,
  AccountPerformancePage, global performance page, WhatsNewDialog.
- E2E: extend affected specs (accounts table column, buy/sell total mode,
  period selector) with stable ids; grep `e2e/` before touching any existing
  id; full suite before release.
- Coverage: 80% on logic files (hooks/presenters/orchestrators); api/gateway
  pass-throughs exempt.
- Housekeeping commit: delete this plan, close shipped todo/techdebt entries.
- Release: `/dep-audit` → `just test-e2e-headless` → `just release` → publish
  after CI E2E green on main.
