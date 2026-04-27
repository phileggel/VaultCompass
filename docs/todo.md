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

MKT-056 and MKT-062 (auto-record DB rollback + tx-form error surfacing) are implemented but their dedicated tests are deferred — see TODO at `src-tauri/src/use_cases/record_transaction/orchestrator.rs`. A repository-level mock seam (e.g. trait-injected `AssetPriceRepository` that can be told to fail after the price write but before commit) would unlock both tests in one shot. Same gap applies to TRX-027 atomicity, which is currently exercised only by happy-path coverage. Pre-existing limitation; not specific to this feature.

## (market-price) — Price-point CRUD page

`AssetPrice` records currently support only same-day overwrite via the existing "Enter price" modal (MKT-025) and have no standalone delete (MKT-042). Once auto-record from transactions ships, wrong-date entries (e.g. user backdates a buy and the auto-recorded price lands on the wrong day) become more likely and the user has no way to correct them.

Add a dedicated price-history view per asset or per holding, listing recorded `AssetPrice` rows with edit and delete affordances. Requires:

- Backend: `delete_asset_price(asset_id, date)` command + repository method.
- Frontend: a "Price history" entry point (likely from the holding row or an asset detail view).
- Spec update: lift MKT-042's "no standalone delete" stance and add new rules covering the list view, delete, and the resulting `AssetPriceUpdated` events.

## (backend/frontend) — Add new financial asset metadata directly from the web
