# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (frontend) — What's-new: show current version's changelog on fresh start

WNW-030 seeds `whats_new_last_seen_version` silently when no key exists, so the upgrade TO the version that introduced the dialog (0.34.0) showed nothing — indistinguishable from a fresh install (observed on the user's machine, 2026-07-06). Change: on fresh start (null key), open the dialog with the **current version's** changelog section only, then acknowledge. Requires amending WNW-030 in `docs/spec/whats-new.md` and the seeding branch in `src/features/whats_new/useWhatsNewDialog.ts`.

## (frontend) — Merge account_performance + global_performance into one performance feature

`GlobalPerformancePage` imports `AccountPerformanceTable`, `AccountValueChart`, and presenter types across the feature boundary. F26 permits presentational crossings, but these are domain-flavored (performance view models), not generic — the domain was cut in half. Merge into `src/features/performance/` (`account_view/`, `global_view/`, `value_chart/`, `shared/` with table + presenters, one `gateway.ts` for both commands); `router.tsx` stays the only consumer of the two pages (`/accounts/$accountId/performance`, `/performance`). Mechanical: `git mv` + import rewrites, no logic/i18n/E2E-id changes. Kills every cross-feature import in `src/`.

## (backend) — 🔴 Sign-flipped Dietz % when the denominator goes negative (account scope)

`metric_for_span_over_flows` (`valuation.rs:543`) guards the Dietz denominator with `== 0`; the asset-scoped variant (`valuation.rs:671`) correctly guards `<= 0`. A negative denominator is reachable at account level — sell holdings and withdraw the proceeds early in a period (prev value 10,000, sell for 12,000, withdraw on Jan 5 → denominator ≈ −1,836 → a +2,000 gain renders as ≈ −109%), or lifetime weighted withdrawals exceeding weighted deposits. Propagates to period_over_period, YTD, since-inception, live-view `compute_current_ytd_pct`, and global performance (same function). Fix: `== 0` → `<= 0` + regression test; align the PRF-032 doc comments ("denominator is 0" → "not positive"). Found by perf-calculation audit 2026-07-06.

## (backend) — 🟡 Closed-position % drifts forever after a full sell (asset scope)

After a position is fully sold, its gain freezes correctly but since-inception/YTD percentages keep moving every period: the Dietz window keeps growing while the sell flow's weight creeps toward 1. Verified numerically (buy 10k → sell all 12k in 2024): since-inception 35% → 75% → 147% → 320% → 1307% → None from 2029 (denominator turns negative); loss case drifts toward −100%; YTD in the sale year inflates 28% (Apr) → 208% (Dec) with the position untouched. `annualized_yield` inherits the drift via since-inception. Formula-correct per PRF-035 but semantically wrong — needs a spec decision: freeze a closed position's % at its close date, or use weighted inflows only (invested capital) as denominator. Interacts with the opening-balance windowed/all-time policy entry below — decide both together. Found by perf-calculation audit 2026-07-06.

## (backend) — 🔵 Disposed zero-cost credits valued at period-end price

Free shares / non-cash interest granted AND fully sold within the same period are still valued at the period-END price in `asset_flow` (`zero_cost_credit_value`, PRF-071 pattern). If the price moved after the sale, pnl and asset_flow carry equal-and-opposite phantom offsets. The bridge identity still balances and no percentage is affected — cosmetic decomposition only. Option: value the credit at its grant/disposal date when the position is closed before period end. Found by perf-calculation audit 2026-07-06.

## (backend) — Opening balance: neutral in windowed performance, cost-based in all-time

An "Add position" (OpeningBalance) is valued at typed cost in the period bridge (PRF-071, `performance.rs` `period_bridge`) and in the per-line windowed flows (`valuation.rs` `position_flows`), so the market-vs-cost delta of a transferred-in position shows up as pnl/% in the period containing the add — performance the account never earned in that window (we don't know when the value was actually gained). Policy decided 2026-07-06:

- **Windowed metrics** (year/month rows, per-line YTD/1y/2y/5y/10y): value the opening-balance flow at **market value** — period-end for the bridge (mirror the FreeShares PRF-071 pattern) and as-of the transaction date for per-line flows (`holding_end_value_as_of` machinery exists). Entry becomes pnl-neutral; performance counts from entry onward. Fallback to typed cost when no market price exists near the entry date.
- **All-time metrics** (since-inception PRF-035, per-line "since start"): keep typed cost — the pre-account gain stays in lifetime performance. No change.

Consequence to codify as a PRF rule: sum of period pnls ≠ since-inception pnl; the difference is the pre-account gain, attributable to no tracked period — intentional, don't "fix" it back. Spec-amendment task: PRF + ACD rules, the two valuation sites, tests. No schema/FE changes.

## (docs) — Rewrite F26: domain axis missing from the cross-feature import rule

F26 evaluates crossings only by behaviour (hooks/stores forbidden, presentational allowed) and misses the domain axis — a domain-flavored dumb component crossing features is the same wrong-boundary smell (bit us: performance table/chart, see merge entry above). Proposed rewrite:

> **F26** — Feature folders are domain boundaries. Cross-feature imports are evaluated on two axes: behaviour AND domain.
>
> - Views/pages are NEVER imported across features — routing is the only entry to another feature's surface.
> - Hooks, stores, and gateways NEVER cross — behaviour coupling signals a wrong feature boundary.
> - Generic primitives (Button-grade components, pure formatters, generic types) MUST NOT live in a feature at all — promote to `ui/` and import from there.
> - Domain-flavored artifacts (view models, presenters, domain tables/charts) needed by more than one view mean those views are ONE feature — merge or re-cut the feature instead of importing across.
>
> Net effect: no import path in `src/features/` may reference a sibling feature's folder.

`docs/frontend-rules.md` is kit-managed (read-only for project content) — route this upstream to the kit; until then the project-side rule lives in CLAUDE.md § Standards (added 2026-07-06).

## (frontend) — Changelog button in the About modal to re-read what's new

Once dismissed (or silently seeded), the changelog is unreachable in-app. Add a "What's new" button to `src/features/about/about_modal/AboutModal.tsx` that opens `WhatsNewDialog` on demand (showing the current version's section, without touching `whats_new_last_seen_version`). Likely rides the `?modal=…` URL-param mount pattern via `src/features/shell/WhatsNewDialogMount.tsx`; new WNW rule(s) in `docs/spec/whats-new.md` when picked up.

## (frontend) — Merge TXL per-asset page into the account journal (deferred)

The per-asset transaction page (`transaction_list/TransactionListPage.tsx`, route `/accounts/$accountId/transactions/$assetId`, the holdings-row loupe target) predates the account journal and is now a strict subset of it — both already share `TransactionTable`, `EditTransactionModal`, delete flow, and `routeEditTransaction`. Consolidate: the loupe navigates to the journal with the asset filter prepopulated (`/accounts/$accountId/journal?asset=<assetId>`); delete the TXL page/hook/route. Decided 2026-07-06: cash-statement columns (Cash out / Cash in / Balance) render only in the unfiltered (global) journal view; with an asset filter active the table shows plain Total Amount — a running balance over a filtered subset is misleading.

Must carry over before deleting TXL: (1) add-transaction CTA + `AddTransactionModal` with prefill from the active filter; (2) the `pendingTransactionAssetId` deep-link round-trip — re-target its senders (`HoldingRow`, `ClosedHoldingRow`, `AssetManager` `returnPath` create-asset flow) to the journal route; (3) fold TXL-0xx spec rules into the journal spec. TXL's in-place account switcher is intentionally dropped. E2E: the suite uses `txl-*` stable ids throughout — rewrite those specs in the same PR (selector-removal trap).

## (frontend) — Edit buy/sell by total amount in the edit-transaction modal

The total-entry mode shipped in v0.34.0 (TRX-060/SEL-050: type qty + all-in total, unit price derived) exists only in the ADD buy/sell modals (`EntryModeToggle` in `BuyTransactionModal`/`SellTransactionModal`). `EditTransactionModal` — opened from the account journal and the per-asset transaction list — only edits price/qty/fees and shows the total read-only. Add the same entry-mode toggle for Purchase/Sell edits: typed total stored verbatim, unit price derived via `deriveUnitPriceMicro` (entry mode is not persisted, so the edit form defaults to price mode and recomputes on switch). Likely new TRX/SEL rule(s) for the edit path when picked up.

## (spec) — PFD (Portfolio Dashboard) unblocked, no spec written

`docs/spec-index.md` lists PFD as `planning — paused — blocked on cash-tracking spec`. Cash-tracking shipped on 2026-05-06, so the blocker is lifted, but no `docs/spec/portfolio-dashboard.md` has been written yet. Next step when picked up: run `/spec-writer portfolio-dashboard` to author the cross-account aggregate-view spec (KPIs + per-account list, per the registry description), then the standard `/contract` → `feature-planner` flow. Update `docs/spec-index.md` to drop the "paused — blocked on cash-tracking spec" suffix at the same time.

## (backend) — Promote BC application services to traits, mock with mockall

`AccountService` and `AssetService` are concrete structs, so cross-BC orchestrators (`HoldingTransactionUseCase`, `ArchiveAssetUseCase`, `DeleteAssetUseCase`, `AccountDetailsUseCase`, …) cannot mockall-mock them and instead test against real services + in-memory SQLite. That's against the spirit of `docs/backend-rules.md` B34 ("Tests for services and orchestrators SHOULD mock external dependencies using mockall-generated mocks") — repositories already follow B34 via `#[cfg_attr(test, mockall::automock)]` on each domain.rs trait, but the service layer above them does not.

Extract a trait per service (e.g. `AccountServiceContract`, `AssetServiceContract`) listing the methods orchestrators call, annotate with `#[cfg_attr(test, mockall::automock)]`, and have orchestrators inject `Arc<dyn AccountServiceContract>` / `Arc<dyn AssetServiceContract>`. Then rewrite the orchestrator inline tests to use the generated `MockAccountService` / `MockAssetService` instead of `setup_pool` + real repositories — true unit isolation, faster, no DB dependency. Surfaced during PR #4 review (2026-05-06).

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.

## (deps) — Accepted risk: RUSTSEC-2026-0185 (quinn-proto, not compiled)

`cargo audit` flags `quinn-proto 0.11.14` (RUSTSEC-2026-0185, remote memory exhaustion via unbounded out-of-order stream reassembly, 7.5 high, fixed in ≥0.11.15). It is only an **optional** dependency of `reqwest 0.13.3` behind the `http3` feature, which is **not enabled** — `cargo tree -i quinn-proto` is empty, confirming it is not compiled into the shipped binary. Non-applicable; flagged at the v0.28.0 release. The v0.29.0 T6 reqwest 0.13 upgrade did **not** prune it (the earlier expectation was wrong): `quinn` is reqwest 0.13's own optional `http3` dependency, resolved into `Cargo.lock` regardless of activation but never compiled. It will only clear if reqwest drops the optional `quinn` entry upstream. Re-evaluate if `http3` is ever enabled.
