# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## ~~(ui) — Locale-aware number formatting in microToDecimal~~ ✅ resolved

Added `microToFormatted` to `src/lib/microUnits.ts` using `Intl.NumberFormat(_displayLocale, ...)`. `_displayLocale` defaults to `"fr"` and is set at startup from `i18n/config.ts` via `setDisplayLocale(i18n.language)`, with a `languageChanged` subscription for runtime switches. `Intl.NumberFormat(undefined)` cannot be used — WebKitGTK on WSL2 resolves `undefined` to en-US, ignoring the OS locale. All display-only values in presenters and hook computed totals use `microToFormatted`. `microToDecimal` (plain `toFixed`) is kept for editable form pre-fill where the browser requires a period decimal separator.

## ~~(ui) — DateField silent stale state when user types invalid text~~ ✅ resolved

`handleInputChange` now always calls `onChange` — passing the valid ISO string when parseable, `""` otherwise. Parent state stays in sync with display value; submit is correctly disabled during partial or invalid input.

## ~~(settings) — User-facing language override (translations + number format)~~ ✅ resolved

`useSettings.ts` exposes `{ currentChoice, setLanguage }` with a `LanguageChoice` type (`"auto" | "en" | "fr"`). `setLanguage` calls `i18n.changeLanguage`, which triggers the `languageChanged` subscription in `i18n/config.ts` to update `setDisplayLocale` automatically. Choice is persisted via `setLanguageOverride`; "auto" falls back to `resolveBrowserLang()`.

## (market-price) — Opt-in: use transaction unit_price as market price

When recording a buy or sell transaction, optionally treat the `unit_price` (excluding fees) as the market price for that date, creating an `AssetPrice` record automatically.

Two possible surfaces (not mutually exclusive):

1. **Global setting** — a toggle in Settings: "Automatically record transaction price as market price". Applies to all future transactions when enabled.
2. **Per-transaction opt-in** — a checkbox in the buy/sell form: "Use this price as today's market price". Gives per-transaction control.

Either surface needs a spec update to MKT before implementation.

## ~~(kit) — Back-fill IPC contracts for all existing domains~~ ✅ resolved

All domain contracts written and reviewed:

1. ~~`asset`~~ ✅ — `docs/contracts/asset-contract.md`
2. ~~`account`~~ ✅ — `docs/contracts/account-contract.md`
3. ~~`transaction`~~ ✅ — `docs/contracts/transaction-contract.md`
4. ~~`account_details`~~ ✅ — `docs/contracts/account_details-contract.md`
5. ~~`record_transaction`~~ ✅ — `docs/contracts/record_transaction-contract.md`
6. ~~`update`~~ ✅ — `docs/contracts/update-contract.md`

Notable findings from contract-reviewer: `ArchivedAssetSell (SEL-037)` guard is missing from `update_transaction` in the orchestrator — the spec mandates it via TRX-033 but the implementation never enforces it on edits.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-03-29): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## ~~(frontend/shell) — Implement Settings page and wire the Settings button in Sidebar~~ ✅ resolved

`src/features/settings/` created with `SettingsPage.tsx` and `useSettings.ts`. Route `/settings` registered in `router.tsx`. `Sidebar.tsx` navigates directly to `/settings` via `navigate({ to: "/settings" })` — no `onSettingsClick` prop needed.

## ~~(frontend/transactions) — TRX-038: implement holdings display~~ ✅ resolved

TRX-038 (holdings refresh on `TransactionUpdated`) is fully satisfied by ACD-039/040 in `useAccountDetails.ts`. The `useTransactionStore` stub was pre-dating Account Details and never wired to any UI; the entire store was dead code. Removed `store.ts`, its export, and the `TransactionUpdated` handler from `src/lib/store.ts`.

## ~~(frontend/shell) — Hardcoded strings in shell components~~ ✅ resolved

`nav.design_system` key added; `shell.sidebar_collapse/expand/version` keys added. `Header.tsx` back button aria-label now uses `t("action.back")`. Both en + fr JSON updated.

## (frontend/accounts) — Extract row handlers into useAccountTable

`AccountTable.tsx` defines inline arrow functions inside the row `.map()`: `onClick`/`onKeyDown` on `<tr>` and `onClick` on action `IconButton`s. Move these handlers into `useAccountTable` to stabilise references and ease testing. See frontend-reviewer warning (account-page task).

## ~~(backend) — Replace string-matching error assertions with structured error types~~ ✅ resolved

Typed error enums introduced across all domains using `thiserror`: `AssetDomainError`, `AssetPriceDomainError`, `CategoryDomainError` in `context/asset/domain/error.rs`; `AccountError` in `context/account/domain/error.rs`; `TransactionError` in `context/transaction/domain/error.rs`; `RecordTransactionError` and `AccountDetailsError` in `use_cases/`. Tests can now match on variants rather than substrings.

## ~~(app) — Add proper application icon~~ ✅ resolved

Source: `.screenshots/vault-compass.png` (1024×1024 RGB). All sizes generated via `cargo tauri icon` — desktop PNGs, `icon.ico`, `icon.icns`, iOS, Android assets.

## ~~(frontend) — Save current view between sessions; start on the accounts page by default~~ ✅ resolved

`lastPath.ts` persists the top-level nav section (`/accounts`, `/assets`, `/categories`) to `localStorage`. `AppShell` saves on every navigation; `indexRoute.beforeLoad` restores on startup. Default is `/accounts`.

## (backend/frontend) — Add new financial asset metadata directly from the web
