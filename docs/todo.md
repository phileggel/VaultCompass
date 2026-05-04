# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (backend/test) — Review account service delegation tests for B33 compliance

8 mock-based tests added to `src-tauri/src/context/account/service.rs` verify one-line passthrough methods (e.g. `get_all`, `get_by_id`, `delete`). B33 flags tests that only verify "a getter returns what was just passed in". Evaluate whether these should be enriched with assertion content or replaced by the existing integration tests in `tests/account_service_crud.rs` that already cover the same methods end-to-end.

## (test) — Add coverage thresholds to vitest.config.ts

`vitest run --coverage` always exits 0 — no regression floor is enforced. Add a `thresholds` block (e.g. `lines: 60, functions: 60`) once a stable baseline has been measured on `main`. Increment thresholds incrementally rather than setting a tight target immediately.

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

## (e2e) — Wire failure screenshot hook in wdio.conf.ts

Add an `afterTest` hook to `wdio.conf.ts` that saves a screenshot to `screenshots/e2e-failures/` whenever a test fails. WebdriverIO supports `browser.saveScreenshot(path)` in hooks; the filename should encode the suite + test name + timestamp so multiple failures don't overwrite each other. This would make the pre-existing flaky navigation failures much easier to diagnose.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (migrations) — Add missing FK indexes across migrations

SQL reviewer flagged FK columns without standalone indexes in several migrations.
Addressed in `202605040001_add_fk_indexes.sql`: added `idx_assets_category_id`, `idx_holdings_asset_id`, `idx_transactions_asset_id`.
Dropped: `asset_prices.asset_id` (already covered by PK leftmost prefix); `202604040001` IF NOT EXISTS and `202604250002` transaction comment fixes (cannot edit applied migrations — would break SQLx hash checks for existing users).

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.
