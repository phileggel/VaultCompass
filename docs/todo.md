# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## ~~(ui) — Locale-aware number formatting in microToDecimal~~ ✅ resolved

Added `microToFormatted` to `src/lib/microUnits.ts` using `Intl.NumberFormat(_displayLocale, ...)`. `_displayLocale` defaults to `"fr"` and is set at startup from `i18n/config.ts` via `setDisplayLocale(i18n.language)`, with a `languageChanged` subscription for runtime switches. `Intl.NumberFormat(undefined)` cannot be used — WebKitGTK on WSL2 resolves `undefined` to en-US, ignoring the OS locale. All display-only values in presenters and hook computed totals use `microToFormatted`. `microToDecimal` (plain `toFixed`) is kept for editable form pre-fill where the browser requires a period decimal separator.

## ~~(ui) — DateField silent stale state when user types invalid text~~ ✅ resolved

`handleInputChange` now always calls `onChange` — passing the valid ISO string when parseable, `""` otherwise. Parent state stays in sync with display value; submit is correctly disabled during partial or invalid input.

## ~~(settings) — User-facing language override (translations + number format)~~ ✅ resolved

`useSettings.ts` exposes `{ currentChoice, setLanguage }` with a `LanguageChoice` type (`"auto" | "en" | "fr"`). `setLanguage` calls `i18n.changeLanguage`, which triggers the `languageChanged` subscription in `i18n/config.ts` to update `setDisplayLocale` automatically. Choice is persisted via `setLanguageOverride`; "auto" falls back to `resolveBrowserLang()`.

## ~~(market-price) — Opt-in: use transaction unit_price as market price~~ ✅ resolved

Both surfaces shipped per MKT-050..062:

- Global toggle in Settings (`features/settings/SettingsPage.tsx`), persisted in `localStorage` via `src/lib/autoRecordPriceStorage.ts`.
- Per-transaction checkbox (`features/transactions/shared/RecordPriceCheckbox.tsx`) wired into buy, sell, add and edit forms; default snapshots the global toggle on create, hardcoded OFF on edit (MKT-052).
- Backend stays stateless on the toggle — `record_price: bool` rides on `CreateTransactionDTO` and the orchestrator upserts `AssetPrice(asset_id, tx.date, tx.unit_price)` inside the same DB transaction (MKT-055/056), with silent overwrite on `(asset_id, date)` collision (MKT-058) and skip when `unit_price = 0` (MKT-061). `AssetPriceUpdated` fires after commit via `AssetService::notify_asset_price_updated()`.

## ~~(kit) — Back-fill IPC contracts for all existing domains~~ ✅ resolved

All domain contracts written and reviewed:

1. ~~`asset`~~ ✅ — `docs/contracts/asset-contract.md`
2. ~~`account`~~ ✅ — `docs/contracts/account-contract.md`
3. ~~`transaction`~~ ✅ — `docs/contracts/transaction-contract.md`
4. ~~`account_details`~~ ✅ — `docs/contracts/account_details-contract.md`
5. ~~`record_transaction`~~ ✅ — `docs/contracts/record_transaction-contract.md`
6. ~~`update`~~ ✅ — `docs/contracts/update-contract.md`

Notable findings from contract-reviewer: `ArchivedAssetSell (SEL-037)` guard is missing from `update_transaction` in the orchestrator — the spec mandates it via TRX-033 but the implementation never enforces it on edits. ✅ Guard was already present; two tests added (`create_sell_rejected_when_asset_archived`, `update_sell_rejected_when_asset_archived`) in `use_cases/record_transaction/orchestrator.rs`.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.

## ~~(frontend/shell) — Implement Settings page and wire the Settings button in Sidebar~~ ✅ resolved

`src/features/settings/` created with `SettingsPage.tsx` and `useSettings.ts`. Route `/settings` registered in `router.tsx`. `Sidebar.tsx` navigates directly to `/settings` via `navigate({ to: "/settings" })` — no `onSettingsClick` prop needed.

## ~~(frontend/transactions) — TRX-038: implement holdings display~~ ✅ resolved

TRX-038 (holdings refresh on `TransactionUpdated`) is fully satisfied by ACD-039/040 in `useAccountDetails.ts`. The `useTransactionStore` stub was pre-dating Account Details and never wired to any UI; the entire store was dead code. Removed `store.ts`, its export, and the `TransactionUpdated` handler from `src/lib/store.ts`.

## ~~(frontend/shell) — Hardcoded strings in shell components~~ ✅ resolved

`nav.design_system` key added; `shell.sidebar_collapse/expand/version` keys added. `Header.tsx` back button aria-label now uses `t("action.back")`. Both en + fr JSON updated.

## ~~(frontend/accounts) — Extract row handlers into useAccountTable~~ ✅ resolved

All substantive row handlers (`handleRowKeyDown`, `handleEditClick`, `handleDeleteClick`) were already in `useAccountTable`. The direct `onClick={() => onAccountClick(account.id)}` on `<tr>` is the correct pattern — a no-op wrapper would add indirection with no logic. No change needed; item closed.

## ~~(backend) — Replace string-matching error assertions with structured error types~~ ✅ resolved

Typed error enums introduced across all domains using `thiserror`: `AssetDomainError`, `AssetPriceDomainError`, `CategoryDomainError` in `context/asset/domain/error.rs`; `AccountError` in `context/account/domain/error.rs`; `TransactionError` in `context/transaction/domain/error.rs`; `RecordTransactionError` and `AccountDetailsError` in `use_cases/`. Tests can now match on variants rather than substrings.

## ~~(app) — Add proper application icon~~ ✅ resolved

Source: `.screenshots/vault-compass.png` (1024×1024 RGB). All sizes generated via `cargo tauri icon` — desktop PNGs, `icon.ico`, `icon.icns`, iOS, Android assets.

## ~~(frontend) — Save current view between sessions; start on the accounts page by default~~ ✅ resolved

`lastPath.ts` persists the top-level nav section (`/accounts`, `/assets`, `/categories`) to `localStorage`. `AppShell` saves on every navigation; `indexRoute.beforeLoad` restores on startup. Default is `/accounts`.

## (testing) — Fault injection seam for orchestrator atomicity tests

MKT-056 and MKT-062 (auto-record price-record failure + tx-form error surfacing) and TRX-027 (buy/sell atomicity) lack dedicated rollback tests. The record_transaction use case was dissolved into `context/account/` (Phase 7); the gap now lives in `src-tauri/src/context/account/service.rs`. A repository-level mock seam (e.g. trait-injected `AssetPriceRepository` that can be told to fail) would unlock all three in one shot.

## ~~(market-price) — Price-point CRUD page~~ ✅ resolved

Shipped per MKT-072–MKT-096 (2026-04-29):

- Backend: `get_asset_prices`, `update_asset_price`, `delete_asset_price` commands + repository methods (`get_all_for_asset`, `get_by_asset_and_date`, `delete`, `replace_atomic`). `AssetPriceDomainError::NotFound` added. `AssetNotFound` propagated from `AssetDomainError` into `AssetPriceCommandError`. Integration tests in `tests/asset_price_crud.rs`.
- Frontend: History (clock) `IconButton` on every active `HoldingRow` opens `PriceHistoryModal` — list with Edit/Delete per row. Edit transitions to `EditPriceForm`; delete requires `ConfirmationDialog`. `usePriceHistory` and `useEditPrice` hooks. Shared `validatePriceForm.ts`. 16 Vitest tests, all green.
- Entry point: History button on holding row in Account Details view.

## ~~(spec/backend) — MKT-043 says AssetNotFound is a specific error but AssetPriceCommandError has no such variant~~ ✅ resolved

`AssetNotFound` added to `AssetPriceCommandError` in `asset-contract.md` (2026-04-29). The backend implementation of `record_asset_price`, `get_asset_prices`, and `update_asset_price` must add this variant to `AssetPriceDomainError` and map it in `to_asset_price_error`.

## (spec) — MKT-055/056 describe backend-atomic auto-record but implementation is frontend-sequential

ADR-006 Note states: "the frontend now calls the two commands independently." ARCHITECTURE.md confirms this. MKT-055 ("orchestrator writes AssetPrice inside the same DB transaction") and MKT-056 ("single database transaction") describe a model that was removed before shipping. The rules should be rewritten to describe the actual frontend-sequential flow (frontend calls `record_asset_price` after a successful buy/sell/correct when `recordPrice` is true and `priceMicro > 0`). Flagged during spec-reviewer pass on the price history CRUD feature (2026-04-29).

## (testing) — Refactor AssetService tests to use MockAssetPriceRepository

`src-tauri/src/context/asset/service.rs` has 24 inline service tests that use real `SqliteAssetRepository`, `SqliteAssetCategoryRepository`, and `SqliteAssetPriceRepository` against an in-memory SQLite database. They test service-layer logic but go all the way down to real SQL, which duplicates coverage already provided by the repository tests and makes failure diagnosis harder (a SQL regression can fail a service test with no useful signal about where the fault is).

`MockAssetPriceRepository` and `MockAssetRepository` are already generated by `#[cfg_attr(test, mockall::automock)]`. Rewriting the service tests to inject mocks would give proper unit-test isolation while keeping the repository tests as the real-DB layer. End-to-end confidence is now covered by `src-tauri/tests/asset_price_crud.rs`.

## (backend/frontend) — Add new financial asset metadata directly from the web
