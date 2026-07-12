# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (infra) — Scheduled daily automatic price download, app-closed (deferred)

Deferred — not for the current batch. Goal: download market prices **once per day after a user-set time** (e.g. every day at 19:00 French time, Europe/Paris) **even if the main app is not running**. Today's auto-fetch only fires on app cold-start (MKT), so it never runs when the app is closed.

The hard part is execution outside the app's lifetime — Tauri code doesn't run when the window is closed. Options to weigh at planning time: (a) register an **OS-level scheduled task** on first enable — cron (Linux) / Task Scheduler (Windows) / launchd (macOS) — that launches the app binary in a **headless `--fetch-prices` CLI mode** which opens the existing SQLite DB, runs the current `use_cases/asset_price_fetch` pipeline (Yahoo keyless), writes prices, and exits; (b) a persistent **tray/background mode** + autostart (`tauri-plugin-autostart`) that keeps a lightweight process resident to fire an in-process timer; (c) a companion background service. (a) is the most faithful to "app not started" but needs per-OS scheduler registration + the headless binary to resolve the app's DB path and config. Config surface: enable/disable, time-of-day + timezone (default Europe/Paris, handle DST), **once-per-day dedupe** (skip if a successful fetch already ran on/after today's trigger). Reuse the existing fetch pipeline and `AssetPriceFetchCompleted` eventing so the running app (if later opened) reflects the background fetch. Cross-platform packaging + permissions are the main risk; size it as its own batch.

## (kit) — Upstream: changelog entries should be commit titles only

`scripts/release.py` is kit-owned (kit-manifest) and deliberately (`re.DOTALL`) copies the full commit message — title + body — into each `CHANGELOG.md` bullet. Since the changelog is end-user-facing (the What's-new dialog renders it in-app), bodies leak developer detail to users. Patched locally 2026-07-12 (`_build_changelog_entry` takes `description.splitlines()[0]` in the Added/Fixed loops); a non-force `just sync-kit` will conflict on it — re-apply until upstream ships. File a kit issue proposing titles-only (or a config flag) like the F26 rewrite (#85); this entry closes when the kit ships it.

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

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.

## (deps) — Accepted risk: RUSTSEC-2026-0185 (quinn-proto, not compiled)

`cargo audit` flags `quinn-proto 0.11.14` (RUSTSEC-2026-0185, remote memory exhaustion via unbounded out-of-order stream reassembly, 7.5 high, fixed in ≥0.11.15). It is only an **optional** dependency of `reqwest 0.13.3` behind the `http3` feature, which is **not enabled** — `cargo tree -i quinn-proto` is empty, confirming it is not compiled into the shipped binary. Non-applicable; flagged at the v0.28.0 release. The v0.29.0 T6 reqwest 0.13 upgrade did **not** prune it (the earlier expectation was wrong): `quinn` is reqwest 0.13's own optional `http3` dependency, resolved into `Cargo.lock` regardless of activation but never compiled. It will only clear if reqwest drops the optional `quinn` entry upstream. Re-evaluate if `http3` is ever enabled.
