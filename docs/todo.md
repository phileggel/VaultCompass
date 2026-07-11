# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (frontend) — Merge "Cash in/out" + "Asset in/out" into one performance column

The performance bridge table (shared `AccountPerformanceTable`, rendered on both the account-detail `/accounts/$id/performance` and global `/performance` pages) shows two adjacent flow columns: "Cash in/out" (`cash_flow`, PRF-070) and "Asset in/out" (`asset_flow`, PRF-071). Merge them into a single sign-coloured **"In/Out"** column (FR "Entrée/sortie"). FE-only display change — the backend keeps `cash_flow` and `asset_flow` separate (needed for the PRF-074 bridge identity and GPF EUR conversion). Presenter: replace `cashFlow` + `assetFlow` in `PeriodRowViewModel` with `externalFlow: PnlCellViewModel = toPnlCell(period.cash_flow + period.asset_flow)`; table: merge the two `<th>`/`<td>` (`${idPrefix}-flow-${rowKey}`); i18n: add `account_performance.column_flows`, drop dead `column_cash_flow`/`column_asset_flow`. Spec: add PRF-075 (combined FE display) + amend PRF-074's display sentence (GPF inherits via the shared table). Update `presenter.test.ts` (incl. the mixed-sign colour case). No E2E refs, no backend change; visual-proof the table.

## (full-stack) — Stock split / reverse split corporate action

Support share splits on a held position. A split is **not** a new asset — a value-neutral corporate action that rescales the position: `quantity ×= factor`, `average_price ÷= factor`, total cost basis + realized P&L unchanged. Example: 10 shares, 5:1 split → 50 shares at 1/5 the price. Reverse splits use `factor < 1` (e.g. 1:10 → ×0.1). Non-integer ratios allowed (3:2 → ×1.5); micro-unit quantities already handle fractional results.

Real reference: Alphabet's **20-for-1** split, effective after close 2022-07-15 (board 2022-02-01, record 2022-07-01), applied to both GOOGL (Class A) and GOOG (Class C) — price ~$2,235 → ~$108, holdings unchanged, no new ticker. Confirms large integer factors and per-share-class application are realistic.

Note on identity changes: even when a corporate action renames/re-tickers the security (e.g. the 2015 **Google → Alphabet** rename, ticker stayed `GOOG`/`GOOGL`, same holding), it stays the **same asset** — the user edits the existing asset's name/reference/ISIN via the current edit-asset flow, keeping the holding and cost basis continuous. Creating a new asset (forking history + cost basis) is never the answer for a split or a rename. The split operation here only rescales quantity/price; an accompanying rename is a separate, already-supported asset edit (planning may optionally let the split modal also update the name/ticker in one step).

Proposed model: a new `TransactionType::Split` carrying the split factor, applied during holding replay at the split date (scale the running position forward; prior per-share prices are not retro-edited — the factor lives on the split transaction). Mirror the FreeShares/Interest corporate-action shape: domain method on `Account` + `Holding`, `record_split` command, orchestrator, bindings, a "Split" affordance on the holding row + modal with a ratio input and a resulting-quantity/price preview, i18n, E2E. Spec: new SPL-0xx rules. Performance bridge: a split is value-neutral, so it must contribute 0 to `cash_flow`/`asset_flow`/`pnl` (quantity changes, value doesn't) — verify it doesn't distort the bridge or the per-line windowed flows. Open question for planning: how the factor is represented (rational new:old vs micro-scaled multiplier) and precision of `average_price ÷ factor`.

## (full-stack) — Per-holding note with optional price alarm

Let the user attach a **note to a line they hold** (a position), distinct from the existing per-transaction `note`. Example: on Air Liquide, note "acheter 7 actions si moins de 150€". Decided design: **one note per holding** `(account_id, asset_id)`; the note is `text` + an **optional alarm** = `threshold_price` (in the **asset's currency**) + `direction` (below/above). Alarm is **stateless & live** — "triggered" is computed on read from `current_price` vs threshold+direction (no persisted acknowledged state); the bell reflects the current condition and re-arms when the price moves back. **In-app only** (no OS notification).

Backend (account BC): holdings are derived by transaction replay, so the note needs its own persisted table `holding_note(account_id, asset_id, text, threshold_price NULL, threshold_direction NULL, timestamps)` + migration; commands `upsert_holding_note` / `delete_holding_note`; the note (text + threshold + computed `triggered`, using the same `current_price` the details read already resolves) rides back on `AccountDetailsResponse`/`HoldingDetail`. Frontend: a **"Note" action button on the holding row** → small modal (textarea + optional "alert me when price is [below/above] [amount] [asset ccy]"); the row renders the **note text under the asset name** plus a **two-state bell** (outline = alarm armed, filled+coloured = threshold currently crossed). Scope: the account-details holding rows (owned lines) for v1. New trigram (e.g. position-note `PNO`/`HNO`) with spec rules; i18n, E2E, visual-proof. Possible later extension noted but out of scope: OS/desktop notification on crossing, and surfacing the note on the global/asset views.

## (backend) — Audit: "add a position" (OpeningBalance) handling in performance

Verify the just-shipped PRF-086 (v0.35.0, T4) actually matches the intended model: an "Add position" (OpeningBalance) must be treated as an **in/out flow**, value-neutral at the moment of add — adding a position should NOT itself manufacture performance (%). But **latent P&L after entry** (market movement once the position is held) SHOULD count in perf — confirm that is the case. Current design to confirm: windowed metrics (period rows, per-line YTD/1y/…) value the OB flow at **entry-date market value** (`opening_balance_flow_value`, PRF-086, fallback typed cost) so the entry is pnl-neutral and performance counts from entry onward; lifetime/since-inception (PRF-035) keeps **typed cost**, so the market-vs-cost delta at entry lands in lifetime gain (the pre-account gain) — is that the desired treatment, or should lifetime also be entry-neutral? Audit scope: `performance.rs` period_bridge OB arm, `valuation.rs` `position_flows_windowed`/`opening_balance_flow_value`, account + asset + global scope, and the live view (`compute_current_ytd_pct`). Trace a concrete case (add 10 sh cost 100, market 150 at entry, then to 180) through cash_flow/asset_flow/pnl/%/since-inception and confirm each figure. If a scope diverges from "flow-neutral at add, movement-after counts," fix + regression test; else close as confirmed. Interacts with the closed-position freeze (PRF-085) and the flow-column merge todo above.

## (infra) — Scheduled daily automatic price download, app-closed (deferred)

Deferred — not for the current batch. Goal: download market prices **once per day after a user-set time** (e.g. every day at 19:00 French time, Europe/Paris) **even if the main app is not running**. Today's auto-fetch only fires on app cold-start (MKT), so it never runs when the app is closed.

The hard part is execution outside the app's lifetime — Tauri code doesn't run when the window is closed. Options to weigh at planning time: (a) register an **OS-level scheduled task** on first enable — cron (Linux) / Task Scheduler (Windows) / launchd (macOS) — that launches the app binary in a **headless `--fetch-prices` CLI mode** which opens the existing SQLite DB, runs the current `use_cases/asset_price_fetch` pipeline (Yahoo keyless), writes prices, and exits; (b) a persistent **tray/background mode** + autostart (`tauri-plugin-autostart`) that keeps a lightweight process resident to fire an in-process timer; (c) a companion background service. (a) is the most faithful to "app not started" but needs per-OS scheduler registration + the headless binary to resolve the app's DB path and config. Config surface: enable/disable, time-of-day + timezone (default Europe/Paris, handle DST), **once-per-day dedupe** (skip if a successful fetch already ran on/after today's trigger). Reuse the existing fetch pipeline and `AssetPriceFetchCompleted` eventing so the running app (if later opened) reflects the background fetch. Cross-platform packaging + permissions are the main risk; size it as its own batch.

## (docs) — Rewrite F26: domain axis missing from the cross-feature import rule

F26 evaluates crossings only by behaviour (hooks/stores forbidden, presentational allowed) and misses the domain axis — a domain-flavored dumb component crossing features is the same wrong-boundary smell (bit us: the performance table/chart, which prompted the account+global performance merge into `features/performance/`, shipped 2026-07-06). Proposed rewrite:

> **F26** — Feature folders are domain boundaries. Cross-feature imports are evaluated on two axes: behaviour AND domain.
>
> - Views/pages are NEVER imported across features — routing is the only entry to another feature's surface.
> - Hooks, stores, and gateways NEVER cross — behaviour coupling signals a wrong feature boundary.
> - Generic primitives (Button-grade components, pure formatters, generic types) MUST NOT live in a feature at all — promote to `ui/` and import from there.
> - Domain-flavored artifacts (view models, presenters, domain tables/charts) needed by more than one view mean those views are ONE feature — merge or re-cut the feature instead of importing across.
>
> Net effect: no import path in `src/features/` may reference a sibling feature's folder.

`docs/frontend-rules.md` is kit-managed (read-only for project content) — filed upstream as [phileggel/claude-kit#85](https://github.com/phileggel/claude-kit/issues/85) (2026-07-06); until the kit ships it, the project-side rule lives in CLAUDE.md § Standards. This entry closes when a kit sync delivers the rewritten F26.

## (frontend) — Merge TXL per-asset page into the account journal (deferred)

The per-asset transaction page (`transaction_list/TransactionListPage.tsx`, route `/accounts/$accountId/transactions/$assetId`, the holdings-row loupe target) predates the account journal and is now a strict subset of it — both already share `TransactionTable`, `EditTransactionModal`, delete flow, and `routeEditTransaction`. Consolidate: the loupe navigates to the journal with the asset filter prepopulated (`/accounts/$accountId/journal?asset=<assetId>`); delete the TXL page/hook/route. Decided 2026-07-06: cash-statement columns (Cash out / Cash in / Balance) render only in the unfiltered (global) journal view; with an asset filter active the table shows plain Total Amount — a running balance over a filtered subset is misleading.

Must carry over before deleting TXL: (1) add-transaction CTA + `AddTransactionModal` with prefill from the active filter; (2) the `pendingTransactionAssetId` deep-link round-trip — re-target its senders (`HoldingRow`, `ClosedHoldingRow`, `AssetManager` `returnPath` create-asset flow) to the journal route; (3) fold TXL-0xx spec rules into the journal spec. TXL's in-place account switcher is intentionally dropped. E2E: the suite uses `txl-*` stable ids throughout — rewrite those specs in the same PR (selector-removal trap).

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
