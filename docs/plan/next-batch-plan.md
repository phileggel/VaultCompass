# Next-batch plan — v0.35.0 → v0.36.0

Branch `next`, one/multiple commits per task, housekeeping closure commit, then CI-gated release.
Deleted by the housekeeping commit once every task below is shipped.

## Batch decisions (locked with user)

- Flow-column merge label: **"In/Out"** / FR **"Entrée/sortie"**.
- Stock split: same asset (never fork), value-neutral rescale; reference Alphabet 20:1 (2022-07-15).
- Holding note: **one note per holding** `(account_id, asset_id)`, threshold watches the **share price**
  (asset currency), **in-app bell only**, stateless/live trigger (no acknowledged state).
- Out of batch: scheduled daily price download (deferred todo stays open).

## Frontend

### T1 — Merge "Cash in/out" + "Asset in/out" into one "In/Out" column

- `docs/spec/account-performance.md` — add **PRF-075** (FE renders `cash_flow + asset_flow` as one
  sign-coloured column; backend terms stay separate), amend PRF-074 display sentence.
- `src/features/performance/shared/presenter.ts` — `PeriodRowViewModel`: replace `cashFlow` +
  `assetFlow` with `externalFlow: PnlCellViewModel`; `presentPeriodRow` sums the two backend fields.
- `src/features/performance/account_view/AccountPerformanceTable.tsx` — merge the two `<th>`/`<td>`
  pairs into one (`${idPrefix}-flow-${rowKey}`).
- `src/i18n/locales/{en,fr}/common.json` — add `account_performance.column_flows` ("In/Out" /
  "Entrée/sortie"); drop dead `column_cash_flow` / `column_asset_flow`.
- Tests: `presenter.test.ts` (combined cell incl. mixed-sign case), integration tests if they
  assert the removed testids. `/visual-proof` the table. No E2E refs exist; no backend change.

## Full-stack features

### T2 — Stock split / reverse split (SPL)

- `docs/spec/stock-split.md` — new SPL-0xx rules: value-neutral rescale (`quantity ×= factor`,
  `average_price ÷= factor`, cost basis + realized P&L unchanged), reverse split = factor < 1,
  non-integer ratios OK, same-asset invariant (rename = separate asset edit), bridge neutrality
  (contributes 0 to cash_flow/asset_flow/pnl), correction/cancel semantics.
- Backend (`src-tauri/src/context/account/`): `TransactionType::Split` (factor stored micro-scaled
  in `quantity`; `unit_price`/`fees`/`total_amount` = 0); `domain/account.rs` `record_split()` +
  replay arm in `recalculate_holding` (+ oversell/chronology interplay) + `correct_transaction`
  Split arm; `domain/transaction.rs` factory + validation (`SplitFactorNotPositive`, reject ×1);
  `service.rs`, `use_cases/holding_transaction/{api,orchestrator}.rs` `record_split` command,
  `core/specta_builder.rs`, `just generate-types`.
- Perf neutrality: verify `use_cases/shared/valuation.rs` flow classifiers ignore Split (no flow,
  no market-valued date) — add windowed + bridge regression tests.
- Frontend: Split action on `HoldingRow.tsx` (+ journal rendering of Split rows in
  `TransactionTable.tsx` / presenters); `features/account_details/split_transaction/` modal —
  ratio input (new:old), resulting qty/price preview, edit via `?modal=` mount if journal-editable;
  i18n; E2E `e2e/split/split.test.ts`; `/visual-proof`.

### T3 — Per-holding note with optional price alarm (HNO)

- `docs/spec/holding-note.md` — new HNO-0xx rules per locked design.
- Migration `migrations/NNN_holding_note.sql` — table `holding_note(account_id, asset_id, text,
threshold_price NULL, threshold_direction NULL, created_at, updated_at, PK(account_id, asset_id),
FKs)`; `just db-migrate` + `just prepare-sqlx`.
- Backend (`context/account/`): `domain/holding_note.rs` (new/with_id/from_storage factories,
  validation: non-empty text, positive threshold, direction requires threshold); repository trait +
  SQLite impl; `service.rs` upsert/delete/get; `api.rs` commands `upsert_holding_note`,
  `delete_holding_note`; note + computed `alarm_triggered` (current_price vs threshold+direction)
  joined into `use_cases/account_details/orchestrator.rs` → `HoldingDetail`; specta registry;
  bindings.
- Frontend: `features/account_details/holding_note/` — `HoldingNoteModal` (textarea + optional
  below/above + amount in asset ccy) + hook + tests; `HoldingRow.tsx` note action button, note text
  under asset name, two-state bell (outline armed / filled+coloured crossed); gateway methods;
  i18n; E2E; `/visual-proof`.

## Backend

### T4 — Audit: OpeningBalance ("add a position") in performance

- Confirm: OB is an in/out flow, pnl-neutral at add (entry-date market value, PRF-086), post-entry
  latent P&L counts; lifetime keeps typed cost (pre-account gain in since-inception — confirm
  desired). Trace one concrete case (cost 100, market 150 at entry, → 180) through
  `use_cases/shared/performance.rs` (`period_bridge` OB arm), `valuation.rs`
  (`position_flows_windowed`, `opening_balance_flow_value`, `compute_current_ytd_pct`), account +
  asset + global scopes. Deliverable: audit table in chat; fix + regression test if a scope
  diverges, else DB-free confirming test(s) + close the todo.

### T5 — Promote BC application services to traits (B34)

- `context/account/service.rs`, `context/asset/service.rs` — extract `#[cfg_attr(test,
mockall::automock)]` traits (`AccountServiceContract`, `AssetServiceContract`) covering the
  methods orchestrators call; impl on the concrete services.
- Orchestrators (`use_cases/{holding_transaction,archive_asset,delete_asset,account_details,…}`)
  inject `Arc<dyn …Contract>`; `lib.rs` wiring updated.
- Rewrite orchestrator inline tests to mockall mocks (drop `setup_pool` + real repos where the test
  is service-level); keep integration tests as-is.

## Techdebt closures

### TD1 — Total-entry derive Rule-of-Three

- `context/account/domain/account.rs` — extract `derive_purchase_from_total()` /
  `derive_sell_from_total()` (validation + `derive_unit_price_from_total`) shared by
  `buy_holding`, `sell_holding`, `correct_transaction`; tests stay green; remove techdebt entry.

### TD2 — Relocate `isCashAsset`

- Move `isCashAsset` from `features/account_details/shared/presenter.ts` to `src/lib/cashAsset.ts`
  (pure helper, same home as `microUnits`/`modalSearch`); update all consumers (account_details,
  performance ×2); kills the last cross-feature import; remove techdebt entry.

### TD3 — `src/lib/update/` → `src/features/update/`

- `git mv` + import-path updates (gold-layout sub-item #1; mechanical). Update the techdebt
  entry's sub-item list (entry stays for the remaining sub-items).

## Housekeeping & release

- Housekeeping: close shipped todos (flow-merge, split, note, OB-audit, services-traits), remove
  TD1/TD2 techdebt entries + TD3 sub-item, `ARCHITECTURE.md` (split flow, holding_note,
  features/update move), delete this plan doc, memory update, `just format`.
- Release: `/dep-audit` → `just test-e2e-headless` green → `just merge` → CI E2E+Quality green on
  merge commit (L-009) → `just release -y` → publish draft (`gh release edit --draft=false`).

## Order

T1 (small FE) → TD2 (unblocks perf-feature purity) → T4 (audit before more perf work) → TD1 → T5
(backend refactor before new BC surface) → T2 (split) → T3 (holding note) → TD3 → housekeeping →
release. Commit at each task's close; reviewers per touched surface + `/review-triage` per batch;
spec docs authored inline (spec-writer shape) with contract-level rules embedded.
