# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (backend/test) — Rename existing Rust test functions to follow test\_ prefix convention

`docs/test_convention.md` now requires all test function names to start with `test_`. The following files have pre-existing names that need renaming (no logic change). Also batch-rename `archive_asset_sets_flag` / `unarchive_asset_clears_flag` in `asset/service.rs` — these names claim to verify a flag but only assert delegation; rename to `archive_asset_delegates_to_repo` / `unarchive_asset_delegates_to_repo` or add flag assertions:

- `src-tauri/src/context/account/service.rs` — ~8 mock-based tests in `mod tests`
- `src-tauri/src/context/asset/service.rs` — ~8 mock-based tests in `mod tests`
- `src-tauri/tests/account_service_crud.rs` — ~8 integration tests
- `src-tauri/tests/asset_service_crud.rs` — ~8 integration tests

Straightforward batch rename, no behavioural changes.

## (backend/test) — Review account service delegation tests for B33 compliance

8 mock-based tests added to `src-tauri/src/context/account/service.rs` verify one-line passthrough methods (e.g. `get_all`, `get_by_id`, `delete`). B33 flags tests that only verify "a getter returns what was just passed in". Evaluate whether these should be enriched with assertion content or replaced by the existing integration tests in `tests/account_service_crud.rs` that already cover the same methods end-to-end.

## (test) — Add coverage thresholds to vitest.config.ts

`vitest run --coverage` always exits 0 — no regression floor is enforced. Add a `thresholds` block (e.g. `lines: 60, functions: 60`) once a stable baseline has been measured on `main`. Increment thresholds incrementally rather than setting a tight target immediately.


## (e2e) — accounts.test.ts confirm-delete button selector too broad

`$('//button[normalize-space()="Delete"]')` at line 184 matches any visible Delete button. Scope to the dialog: `$('[role="dialog"] .//button[normalize-space()="Delete"]')` to prevent matching the wrong element if other Delete buttons exist in the DOM simultaneously.

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

## (backend) — Default tracing log level is "debug" in lib.rs

`initialize_tracing` falls back to `EnvFilter::new("debug")` on any parse error. This floods logs during normal debug runs. Change default to `"info"`; keep `"debug"` opt-in via `RUST_LOG=debug`.

## (e2e) — Review ProjectSF combobox ADRs for applicability to VaultCompass

ProjectSF has two ADRs on combobox handling in tests:

- `\\wsl.localhost\Ubuntu\home\phil\projects\ProjectSF\docs\adr\004-e2e-rtl-test-boundary-combobox.md`
- `\\wsl.localhost\Ubuntu\home\phil\projects\ProjectSF\docs\adr\005-combobox-feasibility-investigation.md`

The `buy_sell.test.ts` E2E test uses `setReactInputValue` on `#buy-trx-asset` (a combobox) then clicks an option via `*=${ASSET_NAME}`. If the combobox behaves differently in WebKit vs jsdom (E2E vs RTL), the same boundary issue may apply here. Review those ADRs and decide if a VaultCompass-specific ADR or test adjustment is needed.

## (e2e) — Extract shared E2E helpers to a common module

`setReactInputValue`, `isoToDisplayDate`, and IPC seed helpers (`seedCategory`, `seedAccount`, `seedAsset`, `seedBuy`) are copy-pasted verbatim across every spec file (`accounts`, `assets`, `buy_sell`, `open_balance`, `asset_web_lookup`). Extract to `e2e/helpers/` (e.g. `react.ts`, `seed.ts`, `date.ts`) and import from there. Reduces duplication and keeps cross-cutting fixes in one place.


## (backend) — String-sentinel "account not found" pattern in open_holding/api.rs

`open_holding/api.rs:89` (and `context/account/api.rs:302`) maps `AccountService::open_holding` errors by matching the string `"account not found"`. This is fragile. Fix: introduce a typed `AccountNotFoundError` in the service layer and downcast it in `to_open_holding_error` as is done for the other variants.

## (i18n) — Hardcoded numeric placeholders in buy/sell transaction modals

`BuyTransactionModal.tsx` and `SellTransactionModal.tsx` use hardcoded `"0.000000"` / `"0.000"` placeholder strings instead of i18n keys. Fixed in `OpenBalanceModal` (keys `open_balance.form_quantity_placeholder` / `open_balance.form_total_cost_placeholder`). Buy and sell modals should be updated consistently.

## (account) — Consolidate account domain contracts into account-contract.md

Convention: one contract file per bounded context domain (not per service or use case). Three files currently cover the account domain and must be merged into `docs/contracts/account-contract.md`:

- `docs/contracts/record_transaction-contract.md` — buy/sell/open_holding/correct/cancel commands
- `docs/contracts/transaction-contract.md` — `get_asset_ids_for_account`
- `docs/contracts/account-contract.md` — CRUD + deletion summary (keep this one, merge the others in)

After merging: delete the two source files and update all references (plan docs, reviewer reports, ARCHITECTURE.md).

## (e2e) — assets.test.ts Archive/Unarchive button selectors not scoped to row

`$('button[aria-label="Archive"]')` and `$('button[aria-label="Unarchive"]')` match the first matching button in the table regardless of which asset row is targeted. If multiple archive buttons are visible simultaneously (e.g. after seeding several assets), the selector will act on the wrong row. Scope to the asset row by its name or position, similarly to the fix needed for the confirm-delete button in `accounts.test.ts`.

## (backend/test) — Extract `fn micro` test helper to avoid duplication

`fn micro(v: i64) -> i64` is defined verbatim in both `src-tauri/src/context/account/service.rs` (inside `#[cfg(test)]`) and `src-tauri/tests/account_service_crud.rs`. Extract to a shared test utility module (e.g. `tests/helpers.rs`) and import from both sites.


## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.
