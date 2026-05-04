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

## (e2e) — Fix pre-existing failing E2E tests

The following tests were failing before the helpers refactor (confirmed via baseline run on 2026-05-04) and are unrelated to that change:

### buy_sell — 3 failing (navigation timing after IPC seed)
- **TRX-010, TRX-020, TRX-030** — all fail at `navigateToAccountDetails`: `//button[contains(., "${accountName}")]` not found after IPC-seeding the account and navigating away/back. Root cause unclear — likely the Accounts list doesn't reflect IPC-seeded rows on remount fast enough. Investigate whether the store event (`AccountAdded`) is being emitted by `add_account` IPC and caught by the frontend store, or whether a longer wait / retry loop is needed.

### accounts — 2 failing (same root cause)
- **ACC-002** — edit account: seeded account row (`//tr[.//button[contains(@aria-label, "${ORIGINAL_NAME}")]]`) not found after `forceRefreshToAccounts()`.
- **ACC-003** — delete account: same selector pattern, same failure.
- ACC-001 and ACC-004 pass, confirming `seedAccount` itself works; the issue is the list not reflecting IPC-seeded rows post-navigation.

### assets — 2 failing (navigation + list refresh)
- **"creates an asset manually"** — creates via UI successfully, but `*=E2E Asset Create` not found after navigating away to Accounts and back. The asset was created but the list doesn't reflect it on remount.
- **"unarchiving an asset"** — archives via IPC, navigates, toggles "Show archived", unarchives, then `*=E2E Asset Unarchive` not found in active list after unchecking toggle.

### open_balance — 1 failing (element click intercepted in beforeEach)
- **TRX-042, TRX-046, TRX-043** — the `beforeEach` `navigateToAccountDetails` fails with "element click intercepted" after TRX-055 passes. The `td:first-child span` click is blocked by an overlapping element — likely a modal or overlay not fully dismissed between tests. Adding an explicit wait for any `[role="dialog"]` to disappear before navigation may fix this.

## (e2e) — Review ProjectSF combobox ADRs for applicability to VaultCompass

ProjectSF has two ADRs on combobox handling in tests:

- `\\wsl.localhost\Ubuntu\home\phil\projects\ProjectSF\docs\adr\004-e2e-rtl-test-boundary-combobox.md`
- `\\wsl.localhost\Ubuntu\home\phil\projects\ProjectSF\docs\adr\005-combobox-feasibility-investigation.md`

The `buy_sell.test.ts` E2E test uses `setReactInputValue` on `#buy-trx-asset` (a combobox) then clicks an option via `*=${ASSET_NAME}`. If the combobox behaves differently in WebKit vs jsdom (E2E vs RTL), the same boundary issue may apply here. Review those ADRs and decide if a VaultCompass-specific ADR or test adjustment is needed.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (migrations) — Add missing FK indexes across migrations

SQL reviewer flagged FK columns without standalone indexes in several migrations.
Addressed in `202605040001_add_fk_indexes.sql`: added `idx_assets_category_id`, `idx_holdings_asset_id`, `idx_transactions_asset_id`.
Dropped: `asset_prices.asset_id` (already covered by PK leftmost prefix); `202604040001` IF NOT EXISTS and `202604250002` transaction comment fixes (cannot edit applied migrations — would break SQLx hash checks for existing users).


## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.
