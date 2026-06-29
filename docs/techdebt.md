# Tech Debt

Observations of code smells, brittle patterns, or pre-existing issues surfaced
during work that don't warrant immediate action. Format produced by the kit's
`/techdebt` skill — see `.claude/kit-tools.md`.

Entries are observations, not commitments. Triaged by `/whats-next` alongside
`docs/todo.md`. Remove an entry once it has been resolved or moved to
`docs/todo.md` for active work.

---

## 2026-06-29 — Carry-forward price lookup duplicated in the valuation engine

- Found by: reviewer-backend (v0.30.0 T6 review)
- Where: `src-tauri/src/use_cases/shared/valuation.rs` — `end_value_as_of` and `free_shares_value` each inline `prices.iter().rev().find(|p| parse_date(&p.date).is_some_and(|d| d <= period_end))`
- Severity: 🔵
- Observation: The "latest price on or before a date" carry-forward search is written twice. It briefly existed as `PricedAsset::price_as_of` (v0.29.0) but was removed as dead code when `account_holdings_as_of` was deleted (v0.30.0 T1); T6 then materialised both copies in the shared module. Reintroduce a `PricedAsset::price_as_of(date)` accessor and route both callers through it — that also lets `PricedAsset::prices` drop back from `pub(crate)` to private. Trivial, deferred to keep T6 a pure move.

## 2026-05-24 — Rust test functions missing `test_` prefix project-wide

- Found by: reviewer-backend (during ISIN-lookup-split review)
- Where: src-tauri/src/ (project-wide — 315 of 391 test functions use descriptive naming without the `test_` prefix)
- Context: branch `feat/explicit-isin-lookup` @ `30ec513`
- Severity: 🔵
- Observation: `docs/test_convention.md` mandates the `test_<subject>_<condition>_<expected_outcome>` naming pattern, but the codebase has organically settled on descriptive names without the `test_` prefix (e.g. `validates_ishares_sp500_isin`, `rejects_empty_string_as_wrong_length`). Only ~20% of test functions (76 of 391) carry the prefix. The reviewer surfaced 26 new tests in the ISIN-lookup feature that follow the existing local convention but diverge from the doc. Resolution direction (project-wide rename to align with the doc OR doc update to codify the de facto pattern) is a separate decision and a separate MR; either path is mechanical but spans every Rust test file in the repo.

---

## 2026-05-16 — ADR status vocabulary lacks an "amends" relationship

- Found by: adr-reviewer (during review of ADRs 008/009/010/011)
- Where: docs/adr/003-cross-context-use-case-orchestration.md, docs/adr/005-account-details-inject-transaction-service.md, docs/adr/README.md
- Context: branch `docs/adr-asset-valuation` @ `4d706dd`
- Severity: 🔵
- Observation: ADR-003 carries `Status: Accepted — amended by ADR-005` and ADR-005 carries `Status: Accepted — amends ADR-003`. The kit's `adr-writer` skill permits only three status values (`Accepted`, `Accepted — supersedes ADR-{NNN}`, `Superseded by ADR-{NNN}`). The "amends / amended by" relationship — capturing "this ADR refines another without superseding it" — has no permitted encoding, so the local files use an unsanctioned vocabulary that won't pass strict reviewer checks. The kit gap (no "amends" relationship class) is the upstream cause; the local files reflect that gap. Not fixed in the ADR-asset-valuation branch because resolving it requires either an upstream kit decision (add an "amended by" vocabulary) or a deliberate local decision to convert ADR-003 → Superseded by ADR-005 (loses the "still partly valid" nuance) — both are out of scope for an in-place ADR edit.

---

## 2026-05-10 — Migrate to FE gold layout (per kit proposals #21–#23)

- Found by: manual (post-FE-architecture delta scan)
- Where: src/ (top-level structure + features/account_details cross-imports)
- Context: branch `main` @ `114cb79`
- Severity: 🟡
- Observation: Three FE layout/coupling deltas surfaced by mirroring the BE architecture revisit on the frontend. The current shape works but encodes implicit conventions that diverge from the kit gold layout (now codified as F26/F27/F28 in `docs/frontend-rules.md` since kit v4.6+; the original kit issues phileggel/claude-kit#21/#22/#23 are effectively ratified). Migration is bit-by-bit per `CLAUDE.md` § Gold Standards & Bit-by-Bit Trajectory — apply gold to new code; defer existing-code reshape unless it fits the 50-LOC + locality + mechanical gates.
  1. **`src/lib/update/` is a feature, mislocated.** It has full feature shape — `gateway.ts` + sub-feature folder (`update_banner/`) + hook + test — but lives under `src/lib/`. Per kit proposal #23, `lib/` (renamed `infra/`) hosts platform adapters only; features must live in `src/features/`. Move to `src/features/update/`. Mechanical rename + import-path update.

  2. **`features/account_details/{buy,sell}_transaction/` cross-imports from `features/transactions/`.** Today the imports are `RecordPriceCheckbox` (component), `TransactionFormData` (type), `validateTransactionForm` / `validateSellForm` (pure functions), and `useTransactions` (hook with state). Per the F23 reframing in kit proposal #21, the first three (primitives) become fine; the fourth (behavior coupling via a hook) remains a code smell. Either `account_details` owns its own thin wrapper around the gateway calls it needs, or the two features consolidate. Worth deciding _with_ the consolidation question (delta #3) rather than fixing the hook coupling alone.

  3. **`account_details` sub-feature bloat (8 sub-features).** Half of them — `buy_transaction`, `sell_transaction`, `deposit_transaction`, `withdrawal_transaction` — are conceptually transaction-recording flows and overlap with the `transactions/` feature. Two reasonable shapes: (a) consolidate the four into `transactions/` and let `account_details` stay focused on the holdings view, or (b) formalize the split — `account_details` owns "modals invoked from the holding row," `transactions/` owns "the transaction list page and its CRUD." Pick (b) as the lighter move; (a) is a bigger refactor.

  4. **`src/lib/*Storage.ts` adapters belong in `src/infra/settings/`.** The browser-`localStorage` UI-preference adapters (`autoFetchStorage.ts`, `autoRecordPriceStorage.ts`, `lastOperationDateStorage.ts`, `closedSectionStorage.ts`) are platform adapters per F28's Store-kinds table and should move to `src/infra/settings/`. New ones keep landing in `src/lib/` to stay consistent with their siblings (a partial move would orphan one file mid-migration). Mechanical folder move + import-path update; fold into the same `lib/ → infra/` rename PR.

  Migration is mechanical for #1/#4 (folder move + import sites) and conventional for #2/#3 (depends on the consolidation decision). Cleanest as one or two dedicated PRs after the kit proposals land (so the project mirrors the kit-ratified spec).

## 2026-06-20 — `replay_cash_holding` uses `Holding::restore` for fresh/upserted holdings

- Found by: reviewer-arch (eager-cash-line review)
- Where: `src-tauri/src/context/account/domain/account.rs` (`replay_cash_holding`, the cash-holding upsert)
- Context: branch `feat/eager-cash-line` @ HEAD
- Severity: 🟡
- Observation: The cash-holding upsert in `replay_cash_holding` builds the holding via `Holding::restore` (the "reconstruct from DB, no validation" factory) for both the preserve-existing-id case and the freshly-generated-id case. Per the three-factory convention it should use `Holding::with_id(existing_id, …)` when preserving an id and `Holding::new(…)` when generating one (both validate). Pre-dates this branch; deferred because the fix needs a new/with*id branch split (design, not a mechanical swap). `seed_cash_holding` (added this branch) already uses `Holding::new` correctly — align `replay_cash_holding` when next touched. Update 2026-06-27 (v0.28.0 T1): re-attempted and confirmed blocked — `Holding::new`/`with_id` return `anyhow::Result`, not `Result<*, AccountError>`, so calling them from `replay_cash_holding`(which returns`AccountError`) needs an anyhow→`AccountError`downcast or migrating the`Holding` factories to typed errors first. The factory migration is the real prerequisite.

## 2026-05-09 — Migrate to gold DDD layout (per kit proposals #17–#19)

- Found by: manual (post-PR-#12 design discussion)
- Where: src-tauri/src/ (top-level structure)
- Context: branch `main` @ `eb4e180`
- Severity: 🟡
- Observation: Three layout deltas from the kit gold target (now codified as B0/B37–B43 in `docs/backend-rules.md` since kit v4.4+; the original kit issues phileggel/claude-kit#17/#18/#19 are effectively ratified). The current shape works but documents the architecture imperfectly to newcomers. Migration is bit-by-bit per `CLAUDE.md` § Gold Standards & Bit-by-Bit Trajectory — apply gold to new code; defer existing-code reshape unless it fits the 50-LOC + locality + mechanical gates.
  1. **`service.rs` lives at the BC root, not in `application/`.** Inconsistent with `domain/` and `repository/` (which ARE folders). Migrate `service.rs` → `application/service.rs` per BC. Note (2026-06-27, v0.28.0 T1): the `account/` BC no longer has an `application/` folder at all — moving `{BC}Error` to the BC root (`error.rs`, per error-model gold) emptied it, so it was removed rather than left as an empty marker. The `application/` folder returns for `account/` when this `service.rs → application/service.rs` migration lands. reviewer-arch flagged the missing folder as a B38 gap; deferred here (an empty layer folder while the BC is otherwise old-layout — `service.rs` at root, `repository/` not `infrastructure/` — would be speculative scaffolding).

  2. **`repository/` should be `infrastructure/`** (DDD layer name). `repository/` is one TYPE of infrastructure; renaming protects against the day a BC adds an external API client, cache adapter, or message-queue subscriber (avoids proliferating peer folders). Today the folder only contains repository impls — stay flat (`infrastructure/{aggregate}.rs`) until non-repo infra arrives, then add siblings without nesting.

  3. **`core/` should be `shared/`**, restructured into the three DDD layer folders. `core/` overpromises ("central business logic" — but BCs ARE the business). Target shape:

     ```
     shared/
     ├── application/error.rs        ← shared InfrastructureError
     ├── domain/cash.rs              ← shared kernel (system_cash_asset_id)
     └── infrastructure/{db, event_bus, logger, specta_*, uow}
     ```

     `InfrastructureError` reclassifies as application-layer (it's the typed application translation of opaque infra failures, per the DDD doc's travel rule — the NAME describes the source, the LAYER is application).

  Migration is mechanical (folder moves + module-path updates, ~50–100 import sites total). Cleanest as a single dedicated chore PR after the kit proposals land (so the project mirrors the kit-ratified spec).

---

## 2026-05-24 — Dialog dialog/close-button lack stable F25 ids

- Found by: reviewer-frontend (during Dialog viewport-clip fix review)
- Where: `src/ui/components/modal/Dialog.tsx:54` (role="dialog" surface), `src/ui/components/modal/Dialog.tsx:69` (close button uses `data-testid="modal-close-btn"`)
- Context: branch `fix/e2e-navigate-click-intercept` @ `148aed1`
- Severity: 🔵
- Observation: F25 mandates stable `id` attributes on dialog containers and prefers `id` over `data-testid` for E2E selectors. Dialog's `role="dialog"` surface has no `id`, and the close button is selectable only via `data-testid="modal-close-btn"`. Migration would touch every Dialog consumer (8+ feature modals) plus the `dismissLeftoverModal` E2E helper which queries the testid — multi-file fanout outside any single fix's scope.

## 2026-05-30 — Row-level "edit asset" affordance is mouse-only

- Found by: reviewer-frontend
- Where: `src/features/account_details/account_details_view/HoldingRow.tsx:131` (holding row), `src/features/assets/asset_table/AssetTable.tsx:182` (asset row)
- Context: branch `feat/ui-tweaks-account-asset` @ `8d904a1`
- Severity: 🟡
- Observation: The row-level "edit asset" affordance (double-click on a holding row, and the analogous AssetTable row) is mouse-only; there is no keyboard equivalent. Designing keyboard parity (e.g. Enter-to-edit) needs a consistent decision across both tables, since AssetTable's Enter/Space currently selects the row rather than opening edit.

## 2026-06-16 — YTD summary helper over-fetches the FX rate map

- Found by: reviewer-backend (accounts-overview-metrics review)
- Where: `src-tauri/src/use_cases/account_performance/orchestrator.rs` (`compute_current_ytd_pct` → `load_rate_map(month_view_available=true)`)
- Context: branch `refactor/ux-improvements` @ HEAD
- Severity: 🟡
- Observation: `compute_current_ytd_pct` (called once per account by `get_account_summaries`) pre-resolves FX rates for every monthly/yearly period-end from the account's earliest date to today, but the YTD computation only consumes two dates (today + prior 31 Dec). For a long-lived account with foreign holdings this is O(months × foreign currencies) unnecessary rate lookups per summary row, multiplied across all accounts on the list. Correctness is unaffected (the two needed dates are always in the set). A targeted `load_rate_map_for_dates(&[today, prior_dec_31])` would bound it. Accepted at current scale per the ACC-024 dependency note; revisit if account/transaction volume grows.

## 2026-06-21 — Inconsistent date display style across the app (fr/us)

- Found by: manual
- Where: `src` — date-rendering surfaces; confirmed sites: `features/transactions/transaction_list/TransactionListPage.tsx:226` (locale-numeric), `features/account_details/price_history/PriceHistoryModal.tsx:144` + `features/account_details/account_details_view/ClosedHoldingRow.tsx:39` (short-month via `account_details/shared/formatDate.ts`), `features/currency/currency_rates_view/CurrencyRatesView.tsx:159` (raw ISO `{rate.date}`)
- Context: branch `fix/datefield-input-typing` @ `6d1682b`
- Severity: 🟡
- Observation: Three different date-display styles coexist with no single audited convention — locale-numeric (14/06/2026) in the transaction journal, short-month (14 juin 2026) in price-history/closed-holdings, and raw ISO (2026-06-14, US-looking on a fr machine) in the currency rates table. The two helpers (`ui/format/date.ts` numeric vs `account_details/shared/formatDate.ts` short-month) also live in different buckets, and some surfaces bypass both.

## 2026-06-21 — Semantic M3 color tokens repurposed for financial polarity

- Found by: reviewer-frontend (journal-bank-statement review)
- Where: `src/features/transactions/transaction_list/TransactionTable.tsx` (cash-out/cash-in cells use `text-m3-error`/`text-m3-success`; realized-P&L cell uses the same tokens)
- Context: branch `feat/journal-bank-statement` @ HEAD
- Severity: 🔵
- Observation: The M3 `error`/`success` semantic color tokens are reused to express financial debit/credit (and gain/loss) polarity — cash out = `text-m3-error` (red), cash in = `text-m3-success` (green). Visually conventional and consistent with the pre-existing P&L sign-coloring, but it overloads tokens whose semantics are failure/confirmation, which a high-contrast or screen-reader-driven theme may interpret differently from "money out / money in". A dedicated `text-m3-debit`/`text-m3-credit` (and `-gain`/`-loss`) alias mapping to the same palette entries — or an ADR ratifying the reuse — would carry the correct intent. Cross-cutting (affects the P&L column too), so larger than one PR.

## 2026-06-28 — `download` emits the raw anyhow error string on the `update:error` event

- Found by: reviewer-backend / reviewer-security / reviewer-arch (v0.29.0 T8 review)
- Where: `src-tauri/src/use_cases/update_checker/service.rs` (`download` → `app_handle.emit("update:error", e.to_string())`)
- Severity: 🔵
- Observation: T8 closed the `Result<_, String>` leak on the three update commands, but the `download` flow reports failure via a one-way `update:error` event whose payload is `e.to_string()` — the full anyhow chain. The current `.context(...)` strings are developer-authored literals (no OS paths/URLs today), and events carry no Specta binding, so the exposure is low. But it is the same anti-pattern T8 removed from the command surface: a future `.context` omission or a system error whose `Display` includes a path would leak silently. Follow-up: migrate the event payload to a typed `UpdateError` shape. `download`/`do_download` also still return `anyhow::Result` (B31) — fold both together.

## 2026-06-28 — ComboboxField `createLabel` default is a hardcoded French string

- Found by: reviewer-frontend (v0.29.0 T3 combobox-open-on-focus review)
- Where: `src/ui/components/field/ComboboxField.tsx` (`createLabel = "+ Créer"` default prop; JSDoc example `"+ Créer un patient"`)
- Severity: 🔵
- Observation: The `createLabel` prop defaults to a hardcoded French string rendered straight to the DOM (F16). A consumer that passes `onCreateNew` without an explicit `createLabel` ships untranslated text. No current consumer hits the default (`AddTransactionModal` passes `t("asset.create_new")`), so it is latent. Fixing it properly needs the leaf to either require `createLabel` (drop the default) or accept an i18n key — a small API decision on a shared primitive, hence deferred rather than folded into T3. Pre-dates v0.29.0.
