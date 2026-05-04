# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (backend/test) — Replace string-sentinel assertion in service.rs test_buy_holding_returns_error_when_save_fails

`src-tauri/src/context/account/service.rs:759` still asserts via `.to_string().contains("simulated DB failure")` — the same string-sentinel antipattern eliminated elsewhere in this file. Replace with a typed downcast assertion, consistent with the rest of the test suite.

## (backend/test) — Review account service delegation tests for B33 compliance

8 mock-based tests added to `src-tauri/src/context/account/service.rs` verify one-line passthrough methods (e.g. `get_all`, `get_by_id`, `delete`). B33 flags tests that only verify "a getter returns what was just passed in". Evaluate whether these should be enriched with assertion content or replaced by the existing integration tests in `tests/account_service_crud.rs` that already cover the same methods end-to-end.

## (test) — Add coverage thresholds to vitest.config.ts

`vitest run --coverage` always exits 0 — no regression floor is enforced. Add a `thresholds` block (e.g. `lines: 60, functions: 60`) once a stable baseline has been measured on `main`. Increment thresholds incrementally rather than setting a tight target immediately.

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

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

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (migrations) — Add missing FK indexes across migrations

SQL reviewer flagged FK columns without standalone indexes in several migrations:

- `init.sql`: `asset_accounts.account_id/asset_id`, `assets.category_id`
- `202604120001`: `holdings.account_id/asset_id`
- `202604120002`: `transactions.account_id/asset_id` (composite index exists but not standalone)
- `202604260001`: `asset_prices.asset_id` (covered by PK composite but standalone preferred)

Also: `202604040001` `CREATE UNIQUE INDEX` missing `IF NOT EXISTS`; `202604250002` DDL + multi-DML should have an explicit SQLx transaction comment.

Not a correctness issue today (single-user, SQLite). Address as a dedicated `chore(migrations): add FK indexes` migration before the schema grows further.

## (ci) — Release workflow warnings from infra reviewer

Flagged in `release-windows.yml` and `release-manual.yml` (not introduced by coverage CI work):

- **`prefix-key` embeds lock-file hash** — both workflows use `prefix-key: *-rust-${{ hashFiles('Cargo.lock') }}`. This defeats `Swatinem/rust-cache`'s partial-match restore because a changed lock file creates a brand-new prefix bucket with no fallback. Change to a static prefix (e.g. `windows-rust-`) and let the action handle lock-file hashing internally.
- **`releaseDraft: true` + live updater endpoint** — `tauri.conf.json` endpoints point to `/releases/latest/download/latest.json`, which GitHub only serves from _published_ (non-draft) releases. Until a draft is manually published, the auto-updater 404s for all users. Either set `releaseDraft: false` or change the updater endpoint to a tag-specific URL.
- **`gh cache delete --all` blast radius** — both release workflows delete _all_ repo caches on failure, including caches from the coverage workflow. Scope to the key created by the run (e.g. `gh cache delete --pattern "windows-rust-"`).
- **Fragile `sleep 10` in asset verification** — replace with a short retry loop (3 attempts, 10 s apart) so the check survives slow GitHub asset uploads.
- **`release-manual.yml` missing "tag is on main" guard** — unlike `release-windows.yml`, the manual workflow does not verify the tag is an ancestor of `main`. Add the same `git merge-base --is-ancestor` check.
- **`release-manual.yml` three-way ternary duplication** — `runs-on`, `targets`, and `tauri-action args` all repeat the same platform→value mapping. Extract to a prior step output to keep changes in one place.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.
