# TODO

<!-- Add new backlog items here. Format: ## #NNN — (domain) — Short title -->
<!-- #NNN is a permanent reference: never renumbered, never reused. A new entry takes the -->
<!-- next free number wherever it is placed. Next free: #018. -->
<!-- Every entry ends with four lines: **User value:**, **Done when:**, **Design:** and -->
<!-- **Open questions:**. Design is `none` until the agent proposes one (it does so before -->
<!-- touching anything the user sees), then `proposed (screenshots/design/NNN-*.png)`, then -->
<!-- `validated` once the human has looked. Open questions are `none` or a `- [ ]` list; an -->
<!-- entry with an open question is not ready. Either side may add a question; the human -->
<!-- answers by editing the entry. -->
<!-- Ordered by user value: entries that change what the user experiences first, -->
<!-- entries with no direct user value after the separator. -->

## Next

<!-- The human's queue: references (#NNN or TD-NNN) in the order to work them. The agent -->
<!-- takes the first ready one, never edits this list, and stops when it is empty. -->

Nothing queued.

## #001 — (frontend) — Keep the asset column in view while scrolling the holdings table sideways

The account view's holdings table (`AccountDetailsView`) has grown past the window width — quantity, average price, price, value, P&L, performance, fees, In/Out — and scrolls horizontally inside its `overflow-auto` container. The header row already sticks to the top; the first column (Asset) does not stick to the left, so once the user scrolls right to read the outer columns they lose which line they are reading. Same shape for the closed-positions table beneath it.

**User value:** The asset name stays on screen while reading the columns to its right, so a row can be read end to end without scrolling back.
**Done when:** The Asset column of the active and closed holdings tables stays pinned at the left edge while the table scrolls horizontally, in light and dark mode, with the row background behind it so scrolled columns do not show through; screenshots committed.
**Design:** none
**Open questions:** none

## #003 — (fullstack) — Show the amount each account moved in the price movement report

The report (`PriceMovementDialog`) gives each account its value before, its value after and the change as a percentage (PMV-024); the amount itself — after minus before, in the account's own currency — is missing, and it is the figure most people look for first. The total row has the same gap in the reference currency. The backend computes and carries it (the dialog renders, never derives — PMV-023's "one valuation" rule), and an unmoved entry shows no amount, as it already shows no percentage (PMV-031).

**User value:** The report states how much each account gained or lost, not only by what proportion.
**Done when:** `PriceMovementRow` and the report's total carry the signed movement amount from the backend, a PMV rule states it (per entry in account currency, total in the reference currency, absent when unmoved), the dialog shows it in a column beside the percentage with the gain/loss polarity, the Rust builder and the dialog tests cover moved / unmoved / total, and the screenshots are recaptured. — merged, unreleased (PR #136).
**Design:** validated (in chat, 2026-09-14)
**Open questions:** none

## #004 — (frontend) — Shorten the price-age label on the holding row to the age alone

The holding row's price cell says how old the price is with a full sentence — `Mis à jour il y a 2 j` / `Updated 2d ago` (`mkt.staleness_days_ago`) and `Mis à jour aujourd'hui` / `Updated today` (`mkt.staleness_today`). In a dense row the words add nothing the position does not already say; the age alone reads faster: `2 j` / `2d`. Copy-only — the formatter and the tests key on the i18n identifiers, not the words. The label only has a today and an N-days form; there is no hours form to keep.

**User value:** The price age reads at a glance as `2 j` instead of a sentence, in a row already short of width.
**Done when:** Both languages' `mkt.staleness_days_ago` and `mkt.staleness_today` carry the age alone, the same-day wording and the FX label question are decided, and the holding-row screenshot is recaptured. — merged, unreleased (PR #124).
**Design:** validated (in chat, 2026-09-13)
**Open questions:**

- [x] Same-day wording once the prefix is gone: `aujourd'hui` / `today`, or `0 j` / `0d` to line up with the others? → `0 j` / `0d`.
- [x] Shorten the FX counterpart on the same row (`currency.rate_staleness_*`, "Rate as of today", FXR-090) the same way? → yes, the same way.

## #005 — (frontend) — Move the holding row's actions next to the asset name, on two rows

The holding row's actions — buy, sell, split, note, fee, price history, transactions, deposit / withdrawal on the cash line, the refresh lock, up to twelve `action-*-{assetId}` buttons — sit in the twelfth and last column, past a horizontal scroll on most windows. They belong beside the thing they act on: the Actions column moves to second place, right after Asset, and its buttons wrap onto two rows so the column stays narrow. Every button keeps its stable id, so the E2E suite is untouched by the move. With the Asset column becoming sticky (see the sticky-column entry above), Asset and Actions together form the pinned left edge.

**User value:** A holding's actions are reachable without scrolling right, next to the name they apply to.
**Done when:** The Actions column is the second column of the active and closed holdings tables, its buttons lay out on two rows, the header and row cells keep their ids, the Vitest row tests still pass unchanged, and the holding-row screenshots are recaptured in both themes.
**Design:** none
**Open questions:** none

## #006 — (frontend) — Use one icon for "open the transactions" on the account header and the holding row

The account header's journal button (`account-details-journal`, `ScrollText`) opens the account's transaction journal; the holding row's loupe (`action-view-transactions-{assetId}`, `Search`) opens the same kind of list for one asset. Same action at two scopes, two unrelated icons — and a loupe says "search", which neither does. One icon, the same on both. Recommendation: `ScrollText`, the ledger, on both — it names what opens, and it is where the loupe is heading anyway once the per-asset page folds into the journal (see the TXL-merge entry below).

**User value:** The same picture means the same thing everywhere: a transaction list, for the account or for one holding.
**Done when:** Both buttons render the same icon, their ids and labels are unchanged, and the header and holding-row screenshots are recaptured. — merged, unreleased (PR #134).
**Design:** validated (in chat, 2026-09-14)
**Open questions:**

- [x] The shared icon: `ScrollText` (recommended, it names what opens), or another? → `ScrollText`.

## #007 — (fullstack) — Show a total row on the accounts list

The accounts list (`AccountTable`) shows each account's Global Value and Unrealized P&L in the account's own currency and stops there: no line says what the portfolio is worth. A total needs one currency, so it is the backend's to compute — every account converted to the reference currency (EUR, GPF-011) with the same rate resolution the performance page and the price-movement total already use, and the same degradation when a rate is missing (FXR-034): that account contributes zero and the total is marked partial, as PMV-042/043 do. The list renders the figure; it never sums mixed currencies itself. The YTD column has no meaningful sum and stays blank on the total row — the portfolio-level figure lives on the performance page.

**User value:** The accounts list answers "what is it all worth" without opening the performance page.
**Done when:** The backend returns the portfolio's total Global Value and total Unrealized P&L in the reference currency, flagged partial when any account could not be converted; an ACC (or GPF) rule states it; the list shows a total row with both figures, the currency, and the partial marker; the Rust computation and the table rendering are tested; screenshots recaptured.
**Design:** none
**Open questions:** none

## #008 — (fullstack) — Backfill one asset's price history over the period an account held it

An account's history and as-of views value a holding at the price recorded for each date; assets bought before the scheduled download existed, or held while it was off, have gaps, and today the only way to fill them is by hand, one price at a time. Add a holding-row action that fills them: for that account and asset, every trading day from the first date the account held the asset to the last (today while still held) that has no recorded price gets its daily close. Dates that already carry a price are left exactly as they are — never overwritten, whatever their source — so a run over a complete history is a no-op that says so. Failure is a reported outcome, never a crash: an unresolvable ticker, a locked asset (MKT-151), or an unreachable provider ends the action with a snackbar and nothing written.

The provider must not be hammered: the daily-close series comes from the ranged request the scheduled download already uses (SPF-030/031, one `period1…period2` call per asset), never one request per date, with the window split into a few large ranges if the provider caps a single one. Shape and precedent: `backfill_currency_rate_history` (FXR-110–114) for exchange rates — same use-case layout, same "written / skipped" feedback.

**User value:** One click gives a holding its full price history for the time the account held it, so past valuations and performance stop showing gaps.
**Done when:** A holding-row action backfills the asset's missing daily closes over the held period through one ranged request per asset (chunked only when the provider caps the window), leaves existing prices untouched, reports written and skipped counts (or that nothing was missing), degrades every failure to feedback with no partial surprise, respects the refresh lock, is specified as MKT rules with a contract entry, and is covered by a Rust integration test on a seeded gap and an E2E scenario on an unresolvable ticker.
**Design:** none
**Open questions:** none

## #009 — (fullstack) — A per-account analysis view: target price, horizon and reasoning on each holding

Now that prices and rates arrive on their own, what is missing is a place to think. A new view, opened from the account header, lists the account's active holdings with the figures the holdings table already computes — quantity, current value, YTD performance — and adds three fields that are the user's own judgement, edited inline per row: a target price (in the asset's currency), a horizon (short / medium / long term), and free text. This is not the holding note (HNO): the note carries an alarm the application acts on; this carries an opinion the application only stores. One derived figure belongs in the row: how far the current price stands from the target, computed by the backend.

Model: a `HoldingAnalysis` entity keyed by (account, asset) in the account bounded context, all three fields optional, removed when all are cleared. It is user data, so it takes the full sync ceremony — change capture, rank, apply path, tombstone, its own event — which is the cost driver: size it like the holding-note feature (backend, frontend and E2E PRs), not like a screen. Route through `/spec-writer` when scheduled; questions for the interview: whether closed positions appear (probably not — "assets owned"), and whether the row shows the note's alarm threshold read-only beside the target so the two are visibly different things.

**User value:** A working sheet per account where each holding carries the user's target, horizon and reasoning next to its live figures, kept in sync across devices like everything else.
**Done when:** The view lists active holdings with their computed figures and the three editable analysis fields plus distance to target; the entity syncs (captured, applied, tombstoned, announced) with CFR outcomes stated; the trigram is registered and every rule is covered by tests; screenshots and an E2E scenario are committed.
**Design:** none
**Open questions:** none

## #010 — (frontend) — Give multi-device sync its own view and rework its UI

Sync ships as one `SyncSection` inside the settings page (`src/features/settings/sync/`, ~12 KB of TSX). That section now carries the whole feature: the status block (enabled/paused, device name, folder, last sync), the roster of other computers, held-back counts, failures, conflict notices, inconsistent holdings, six actions (Sync now, Pause, Rename, Change folder, Leave, Start over), two modals, and a single-field prompt shared between rename and change-folder. It has outgrown a settings section.

Observed friction (2026-08-28, first real two-computer setup):

- The six actions render as one flat row of buttons with no grouping by consequence — "Sync now" sits beside "Start over", which discards every published file.
- The shared rename/change-folder prompt forced a conditional Browse button, since only one of the two takes a path. Splitting them into purpose-built dialogs removes the conditional.
- `InstallationHoldsUserData` states that joining requires a fresh installation but gives no route forward; the user has to be told out-of-band which directory to clear. The error should explain the remedy, and ideally offer it.
- Status is a bare `<dl>` — no sense of health at a glance, and nothing shows whether the other computer has published yet, which is the first thing a user checks when a join fails.

Proposal: promote sync to its own route (`/sync`, following the `/performance` precedent in `src/router.tsx`), leaving settings with a link and, at most, the enabled/paused summary. Group the actions by consequence (routine / device / destructive), split the shared prompt, and give the status block a health-oriented layout.

Carry-over risk: `e2e/sync/sync.test.ts` selects on `sync-*` stable ids throughout (`#sync-now`, `#sync-pause`, `#sync-leave`, `#sync-change-folder`, `#sync-enable-*`, `#sync-indicator`). Moving or regrouping those elements breaks the suite silently — `just check` does not type-check `e2e/`. Rewrite the specs in the same PR.

**User value:** Sync health is readable at a glance, destructive actions sit apart from routine ones, and a refused join states how to fix it.
**Done when:** Sync lives at `/sync` with a link from settings, actions are grouped by consequence, rename and change-folder have separate dialogs, `InstallationHoldsUserData` names the remedy, and `e2e/sync/sync.test.ts` passes on the new ids.
**Design:** none
**Open questions:** none

## #011 — (fullstack) — Monitored assets, price bars, and indicator primitives

Prerequisite work for the private advice module — design in [`advice-module-design.md`](advice-module-design.md) (draft, hook not yet ratified). Two public-side steps, both useful on their own: (1) a `monitored` asset flag plus an `asset_daily_bars` table (OHLCV, separate from `asset_prices` so the latest-write-wins price semantics stay untouched), fetched as one ranged request per monitored asset at the minimum window the enabled indicators need — one year of daily bars covers every requirement including SMA(200), and its month-end closes feed the monthly algorithms without a second call (25 KB / 256 bars measured); afterwards only the missing tail is topped up by the scheduled fetch. (2) Indicator primitives (SMA/EMA, MACD, ATR, RSI, Bollinger, Donchian, monthly closes, drawdown) as pure tested functions plus an indicator panel — readings only, no verdicts. Verdicts and levels stay in the private module. Route through /spec-writer when scheduled; the doc's open questions (target weights for 5/25 drift, SMA(200) inclusion) should be closed first.

**User value:** The user reads technical indicators (SMA, EMA, MACD, ATR, RSI, Bollinger, Donchian, drawdown) for the assets they mark as monitored.
**Done when:** The `monitored` flag and `asset_daily_bars` ship with the ranged fetch, the indicator functions are unit-tested, and the panel renders readings for a monitored asset.
**Design:** none
**Open questions:** none

## #012 — (fullstack) — Explain suppressed lifetime performance metrics instead of a bare "—"

When the since-inception % and annualized-yield columns are suppressed by the Dietz guard (denominator ≤ 0), the performance view shows "—" with no cause, which reads as a bug. Real-world trigger (CTO account, 2026-07-27): opening balances typed with unit price 0 (employee free shares) plus early withdrawals make the lifetime denominator negative forever, while the windowed Perf % column computes fine — the user cannot tell the data is fixable. Proposal: the response carries a degradation reason for suppressed lifetime metrics (e.g. zero-valued opening balance vs. genuinely undefined), and the view surfaces a persistent contextual hint (info icon/tooltip on the suppressed cells — not a snackbar, which is transient and re-fires) telling the user which transaction to correct. Needs a PRF spec rule + contract field + both layers; route through /spec-writer when scheduled. Companion guardrail at the entry side (user decision 2026-07-27: warn, don't block — a truly worthless position is legitimate): when an opening-balance form is submitted with Total Cost 0, show an inline warning that zero declares no starting capital and suppresses lifetime performance, suggesting the entry-date market value instead.

**User value:** When lifetime performance shows “—”, the user learns why and which transaction to correct.
**Done when:** The response carries a degradation reason, suppressed cells show a persistent hint naming the cause, and an opening balance submitted with Total Cost 0 warns inline.
**Design:** none
**Open questions:** none

## #013 — (frontend) — Merge TXL per-asset page into the account journal (deferred)

The per-asset transaction page (`transaction_list/TransactionListPage.tsx`, route `/accounts/$accountId/transactions/$assetId`, the holdings-row loupe target) predates the account journal and is now a strict subset of it — both already share `TransactionTable`, `EditTransactionModal`, delete flow, and `routeEditTransaction`. Consolidate: the loupe navigates to the journal with the asset filter prepopulated (`/accounts/$accountId/journal?asset=<assetId>`); delete the TXL page/hook/route. Decided 2026-07-06: cash-statement columns (Cash out / Cash in / Balance) render only in the unfiltered (global) journal view; with an asset filter active the table shows plain Total Amount — a running balance over a filtered subset is misleading.

Must carry over before deleting TXL: (1) add-transaction CTA + `AddTransactionModal` with prefill from the active filter; (2) the `pendingTransactionAssetId` deep-link round-trip — re-target its senders (`HoldingRow`, `ClosedHoldingRow`, `AssetManager` `returnPath` create-asset flow) to the journal route; (3) fold TXL-0xx spec rules into the journal spec. TXL's in-place account switcher is intentionally dropped. E2E: the suite uses `txl-*` stable ids throughout — rewrite those specs in the same PR (selector-removal trap).

**User value:** One transaction view instead of two near-identical ones; the holdings loupe opens the journal filtered to that asset.
**Done when:** The loupe navigates to `/accounts/$accountId/journal?asset=…`, the TXL page/hook/route are deleted, add-transaction prefill and the `pendingTransactionAssetId` deep link work from the journal, and the `txl-*` specs are rewritten.
**Design:** none
**Open questions:** none

---

<!-- Below: no direct user value — test infrastructure, conventions, dependency currency. -->

## #014 — (e2e) — Drive a second device in the E2E suite

The multi-device sync E2E covers the single-device critical path only (plan § Halt Artifact H1): `wdio.conf.ts` launches one binary with one `VAULT_COMPASS_E2E_DATA_DIR` and `maxInstances: 1`, so joining a folder another device created (SYN-014/036) is proven by the two-database integration test `src-tauri/tests/sync_two_devices.rs`, not through the UI. A real two-device E2E needs an `e2e/helpers/second_device.ts` that launches a second binary against its own data directory plus a wdio multi-remote configuration — a separate, pre-requisite task before any join scenario is written.

**User value:** None directly — test infrastructure.
**Done when:** A wdio multi-remote config and `e2e/helpers/second_device.ts` launch a second binary on its own data directory, and a join scenario (SYN-014/036) passes through the UI.
**Design:** none
**Open questions:** none

## #015 — (deps) — Upgrade specta / tauri-specta / specta-typescript past rc.22

Pinned at `specta 2.0.0-rc.22`, `tauri-specta 2.0.0-rc.21`, `specta-typescript 0.0.9`. The lockstep bump to rc.25 / rc.25 / 0.0.12 was attempted on 2026-09-12 and reverted: `specta-typescript 0.0.12` removed the global `Typescript::bigint(BigIntExportBehavior::Number)` switch this project relies on, and now refuses to export any unannotated `i64` / `u64` (`Error::bigint_forbidden`). The replacement is a per-field `#[specta(type = specta_typescript::Number)]` override (or a wrapper type) on every 64-bit integer that crosses the wire — which, under ADR-001, is every monetary amount, quantity, rate and percentage in every DTO and command signature. That is an annotation sweep across the whole wire surface, not a dependency bump, and a single missed field fails bindings generation.

**User value:** None — dependency currency.
**Done when:** every wire-visible 64-bit integer carries the `Number` override (or a shared newtype does), `just generate-types` produces a bindings diff that is cosmetic only, and the three crates sit on a current release together.
**Design:** none
**Open questions:** none

## #016 — (deps) — Accepted risk: WebdriverIO 9 transitive advisories (extract-zip)

`npm audit` reports 14 advisories (2 low, 12 high), all in `devDependencies`. The twelve highs share one root: `extract-zip`, affected in every published version and reached only through `@puppeteer/browsers 2.13.2`, which WebdriverIO 9.31 pins for downloading browser binaries the suite never uses (it drives the Tauri binary through tauri-driver). npm's only proposed fix is a downgrade to WebdriverIO 8.14.6. The two lows are mocha's bundled `diff`. The `deepmerge-ts` and `js-yaml` advisories were cleared by the in-range bump of 2026-09-12 (`ff986f1`). Nothing from these packages enters the application bundle or the Tauri binary, and CI's `npm audit --omit=dev` gate is green. Re-run `npm audit` at each release.

**User value:** None — devDependency advisories; nothing from them enters the shipped bundle.
**Done when:** `@puppeteer/browsers` ships a patched `extract-zip` (or WebdriverIO drops it), `npm audit` is clean, and this entry is deleted.
**Design:** none
**Open questions:** none

## #017 — (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.

**User value:** None — the vulnerable RSA path is unreachable in a SQLite-only build.
**Done when:** sqlx stops compiling `sqlx-mysql` for sqlite-only builds or `rsa` publishes a fix, `cargo audit` is clean, and this entry is deleted.
**Design:** none
**Open questions:** none
