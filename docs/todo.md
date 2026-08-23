# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (e2e) — Drive a second device in the E2E suite

The multi-device sync E2E covers the single-device critical path only (plan § Halt Artifact H1): `wdio.conf.ts` launches one binary with one `VAULT_COMPASS_E2E_DATA_DIR` and `maxInstances: 1`, so joining a folder another device created (SYN-014/036) is proven by the two-database integration test `src-tauri/tests/sync_two_devices.rs`, not through the UI. A real two-device E2E needs an `e2e/helpers/second_device.ts` that launches a second binary against its own data directory plus a wdio multi-remote configuration — a separate, pre-requisite task before any join scenario is written.

## (fullstack) — Monitored assets, price bars, and indicator primitives

Prerequisite work for the private advice module — design in [`advice-module-design.md`](advice-module-design.md) (draft, hook not yet ratified). Two public-side steps, both useful on their own: (1) a `monitored` asset flag plus an `asset_daily_bars` table (OHLCV, separate from `asset_prices` so the latest-write-wins price semantics stay untouched), fetched as one ranged request per monitored asset at the minimum window the enabled indicators need — one year of daily bars covers every requirement including SMA(200), and its month-end closes feed the monthly algorithms without a second call (25 KB / 256 bars measured); afterwards only the missing tail is topped up by the scheduled fetch. (2) Indicator primitives (SMA/EMA, MACD, ATR, RSI, Bollinger, Donchian, monthly closes, drawdown) as pure tested functions plus an indicator panel — readings only, no verdicts. Verdicts and levels stay in the private module. Route through /spec-writer when scheduled; the doc's open questions (target weights for 5/25 drift, SMA(200) inclusion) should be closed first.

## (fullstack) — Explain suppressed lifetime performance metrics instead of a bare "—"

When the since-inception % and annualized-yield columns are suppressed by the Dietz guard (denominator ≤ 0), the performance view shows "—" with no cause, which reads as a bug. Real-world trigger (CTO account, 2026-07-27): opening balances typed with unit price 0 (employee free shares) plus early withdrawals make the lifetime denominator negative forever, while the windowed Perf % column computes fine — the user cannot tell the data is fixable. Proposal: the response carries a degradation reason for suppressed lifetime metrics (e.g. zero-valued opening balance vs. genuinely undefined), and the view surfaces a persistent contextual hint (info icon/tooltip on the suppressed cells — not a snackbar, which is transient and re-fires) telling the user which transaction to correct. Needs a PRF spec rule + contract field + both layers; route through /spec-writer when scheduled. Companion guardrail at the entry side (user decision 2026-07-27: warn, don't block — a truly worthless position is legitimate): when an opening-balance form is submitted with Total Cost 0, show an inline warning that zero declares no starting capital and suppresses lifetime performance, suggesting the entry-date market value instead.

## (frontend) — Merge TXL per-asset page into the account journal (deferred)

The per-asset transaction page (`transaction_list/TransactionListPage.tsx`, route `/accounts/$accountId/transactions/$assetId`, the holdings-row loupe target) predates the account journal and is now a strict subset of it — both already share `TransactionTable`, `EditTransactionModal`, delete flow, and `routeEditTransaction`. Consolidate: the loupe navigates to the journal with the asset filter prepopulated (`/accounts/$accountId/journal?asset=<assetId>`); delete the TXL page/hook/route. Decided 2026-07-06: cash-statement columns (Cash out / Cash in / Balance) render only in the unfiltered (global) journal view; with an asset filter active the table shows plain Total Amount — a running balance over a filtered subset is misleading.

Must carry over before deleting TXL: (1) add-transaction CTA + `AddTransactionModal` with prefill from the active filter; (2) the `pendingTransactionAssetId` deep-link round-trip — re-target its senders (`HoldingRow`, `ClosedHoldingRow`, `AssetManager` `returnPath` create-asset flow) to the journal route; (3) fold TXL-0xx spec rules into the journal spec. TXL's in-place account switcher is intentionally dropped. E2E: the suite uses `txl-*` stable ids throughout — rewrite those specs in the same PR (selector-removal trap).

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.
