# Next-batch plan — 2026-07-06

Branch `next`, one or more commits per task, housekeeping commit at the end, then release.
Scope decided in-session: all pending `docs/todo.md` entries **except** the TXL→journal merge
(explicitly deferred), plus one extra small task and three techdebt closures picked by the user.
Design decisions locked by the user: closed-position drift → **freeze at close**; zero-cost
credits → **grant-date valuation**; small task → **check-for-updates in About**; techdebts →
GPF rate-preload narrowing, Holding-factory typed errors, keyboard edit parity.

Merge strategy: single batch branch → `just merge` → release (v0.34.0 precedent). No PRs.
Reviewer batches per task type; every reviewer batch goes through `/review-triage`.

---

## Backend cluster — performance-audit fixes (sequential, shared files)

### T1 🔴 Dietz negative-denominator sign flip

- `src-tauri/src/use_cases/shared/valuation.rs` — `metric_for_span_over_flows` guard
  `denominator == 0` → `denominator <= 0`; align the PRF-032 doc comments (lines 57, 183).
- Spec: amend PRF-032 wording ("denominator is 0" → "denominator not positive") in the
  performance spec.
- Tests (inline, `valuation.rs` / `account_performance` orchestrator): sell-all + withdraw
  early in a period → positive gain must yield `pct = None`, never a negative percentage;
  lifetime case with weighted withdrawals > weighted deposits.

### T2 🟡 Freeze closed-position % at close date

- Rule: for an **asset-scoped** metric, when the scoped position quantity is 0 as of
  `period_end`, the Dietz window ends at the **close date** (date of the sell that brought
  quantity to 0; a later re-buy reopens the window normally). Applies to since-inception,
  YTD, and the `annualized_yield` elapsed-years. Gain is already frozen; this freezes the %.
- `src-tauri/src/use_cases/shared/performance.rs` — asset-scope paths of
  `since_inception_metric` / `metric_for_scope` (year_to_date callsite) + `annualized_yield_metric`
  elapsed span; quantity/close-date probe via the existing transaction replay.
- Spec: new PRF rule + PRF-035 amendment.
- Tests: the audit's verified drift scenario (buy 10k 2023 → sell-all 12k 2024) must produce a
  **constant** since-inception % on every row from 2024 on (no 35→1307→None drift); loss case
  constant; YTD frozen from the close month; re-buy resumes.

### T3 🔵 Zero-cost credits valued at grant date

- `zero_cost_credit_value` — price (`price_as_of`) and FX rate resolved **as-of the grant
  date** instead of `period_end`; post-grant movement then lands in pnl; the
  disposed-within-period quirk disappears structurally.
- Rate-map coverage: grant dates join the pre-resolved date set (`load_rate_map` /
  GPF `prepare_account`).
- Spec: FSD-070 / PRF-071 / INT-024 amendments (grant-date valuation).
- Tests: grant-then-dispose within one period (no phantom pnl offset), grant while held
  (asset_flow at grant-date value, remainder of movement in pnl).

### T4 Opening balance neutral in windowed performance

- Rule (user-decided): **windowed** metrics value an OpeningBalance flow at **market value
  as-of its transaction date** (fallback: typed cost when no usable price/rate) — bridge
  `asset_flow` (both scopes) and per-line ACD window flows; **all-time** metrics
  (since-inception PRF-035, per-line since-start) keep typed cost, so pre-account gains stay
  in lifetime performance.
- `performance.rs` (`period_bridge` / `holding_period_bridge` OB arms),
  `valuation.rs` (windowed flow extraction for OB; lifetime extraction unchanged).
- Rate-map coverage: OB transaction dates join the pre-resolved date set.
- Spec: PRF/ACD amendments + explicit rule that Σ period pnl ≠ since-inception pnl by the
  pre-account gain — intentional, not a bug.
- Tests: OB at historical cost ≠ entry-date market → entry period pnl-neutral (windowed),
  lifetime gain unchanged; unpriced-asset fallback to cost.

### TD1 Narrow GPF rate-preload date set

- `src-tauri/src/use_cases/global_performance/orchestrator.rs` (`prepare_account`) — resolve
  reference-currency rates only for flow/bridge-relevant transaction types, **plus** the
  T3/T4 grant/OB dates. Delegation-equality tests must stay green.
- Closes the 2026-07-05 techdebt entry.

### TD2 Holding factories → typed errors; fix `replay_cash_holding`

- `src-tauri/src/context/account/domain/holding.rs` (or colocated) — `Holding::new` /
  `Holding::with_id` return `Result<_, AccountError>` (typed) instead of `anyhow`; update all
  callers.
- `account.rs` `replay_cash_holding` — upsert uses `with_id(existing_id, …)` /
  `new(…)` per the three-factory convention instead of `restore`.
- Closes the 2026-06-20 techdebt entry.

## Frontend cluster

### T5 What's-new opens on fresh start

- `src/features/whats_new/useWhatsNewDialog.ts` — null-key branch shows the **current**
  version's changelog section (dismiss acknowledges) instead of silent seeding.
- Spec: WNW-030 amendment in `docs/spec/whats-new.md`.
- Tests: hook + mount tests updated. E2E: the wdio pre-suite key-clear now means the dialog
  opens on a fresh E2E launch — extend the pre-suite hook to seed/dismiss deterministically.

### T6 Changelog button in About modal

- `src/features/about/about_modal/AboutModal.tsx` — "What's new" button opening
  `WhatsNewDialog` with the current version's section, **without** touching
  `whats_new_last_seen_version`. New WNW rule; i18n en+fr; stable id; tests.

### T7 Check-for-updates button in About modal

- `AboutModal.tsx` + existing update gateway (`src/lib/update/`) — manual check with
  checking / up-to-date / update-available states (available routes into the existing
  banner flow). i18n en+fr; stable ids; tests.
- Visual proof: one capture for the reworked About modal covers T6+T7.

### T8 Edit buy/sell by total amount

- `src/features/transactions/edit_transaction_modal/EditTransactionModal.tsx` (+ hook) —
  `EntryModeToggle` for Purchase/Sell edits: typed all-in total stored verbatim, unit price
  derived via `deriveUnitPriceMicro`; defaults to price mode; mode not persisted.
- `shared/validateTransaction.ts` reuse; new TRX/SEL rule(s) for the edit path; i18n; tests;
  grep `e2e/` for edit-modal selectors before shipping.

### TD3 Keyboard parity for row-level edit

- `HoldingRow.tsx` + `AssetTable.tsx` — Enter-to-edit on focused rows (AssetTable keeps
  Space = select); F24 a11y labels, F25 stable ids; tests.
- Closes the 2026-05-30 techdebt entry.

### T9 Merge performance features into `features/performance/`

- `git mv` `account_performance/` + `global_performance/` →
  `src/features/performance/{account_view,global_view,value_chart,shared}`; merge the two
  `gateway.ts`; single `index.ts` exporting both pages; `router.tsx` re-points; import paths
  updated everywhere (incl. tests). i18n keys and E2E stable ids unchanged (`idPrefix` stays).
  Mechanical only — zero logic change. Kills every cross-feature import in `src/`
  (per the new CLAUDE.md § Standards rule).

## Meta

### T10 File the F26 rewrite upstream

- `gh issue create` on `phileggel/claude-kit` with the proposed two-axis F26 text from
  `docs/todo.md`; add the issue link to that todo entry (entry stays open until the kit ships).

### Housekeeping commit

- Close shipped `docs/todo.md` entries; remove the three closed techdebt entries;
  `ARCHITECTURE.md` (features/performance/ move); delete this plan doc; memory update.

### Release

- `/dep-audit` → `just test-e2e-headless` green locally → `just merge` → CI E2E + Quality
  green on the merge commit (L-009 hard gate) → `just release -y` → Release workflow green →
  `gh release edit vX.Y.Z --draft=false`.

## Execution order

T1 → T2 → T3 → T4 → TD1 → TD2 (backend, shared files, sequential) → T5 → T6+T7 → T8 → TD3
(frontend) → T9 (mechanical move last) → T10 → housekeeping → release. Commit at each task's
close; reviewer batch + `/review-triage` per task cluster.
