# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## ~~(ui) — Locale-aware number formatting in microToDecimal~~ ✅ resolved

Added `microToFormatted` to `src/lib/microUnits.ts` using `Intl.NumberFormat(undefined, ...)`. All display-only values in presenters and hook computed totals now use `microToFormatted`. `microToDecimal` (plain `toFixed`) is kept for editable form pre-fill where the browser requires a period decimal separator. Tauri's WebView inherits the OS locale for `Intl` — verify French display in the running app.

## ~~(ui) — DateField silent stale state when user types invalid text~~ ✅ resolved

`handleInputChange` now always calls `onChange` — passing the valid ISO string when parseable, `""` otherwise. Parent state stays in sync with display value; submit is correctly disabled during partial or invalid input.

## (market-price) — Opt-in: use transaction unit_price as market price

When recording a buy or sell transaction, optionally treat the `unit_price` (excluding fees) as the market price for that date, creating an `AssetPrice` record automatically.

Two possible surfaces (not mutually exclusive):

1. **Global setting** — a toggle in Settings: "Automatically record transaction price as market price". Applies to all future transactions when enabled.
2. **Per-transaction opt-in** — a checkbox in the buy/sell form: "Use this price as today's market price". Gives per-transaction control.

Requires the Settings page (see Settings todo) for option 1. Either surface needs a spec update to MKT before implementation.

## (kit) — Back-fill IPC contracts for all existing domains

The kit workflow now requires `docs/contracts/{domain}-contract.md` (derived via `/contract` from the validated spec). Run `/contract` for each domain in order:

1. ~~`asset`~~ ✅ — `docs/contracts/asset-contract.md` (extended by `market-price`)
2. `account` — spec: `docs/spec/account.md`
3. `transaction` — specs: `docs/spec/financial-asset-transaction.md` + `docs/spec/sell-transaction.md`
4. ~~`account_details`~~ ✅ — `docs/contracts/account_details-contract.md` (extended by `market-price`)
5. `record_transaction` — spec: `docs/spec/financial-asset-transaction.md` + `docs/spec/sell-transaction.md`
6. `update` — spec: `docs/spec/update.md`

After each `/contract`, run `contract-reviewer` to validate before moving on.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-03-29): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (frontend/shell) — Implement Settings page and wire the Settings button in Sidebar

The Settings button in `Sidebar.tsx` footer is wired via `onSettingsClick?` but `MainLayout` does not yet pass that handler.
Create the Settings page (feature `settings/`) and pass `onSettingsClick` from `MainLayout`.

## ~~(frontend/transactions) — TRX-038: implement holdings display~~ ✅ resolved

TRX-038 (holdings refresh on `TransactionUpdated`) is fully satisfied by ACD-039/040 in `useAccountDetails.ts`. The `useTransactionStore` stub was pre-dating Account Details and never wired to any UI; the entire store was dead code. Removed `store.ts`, its export, and the `TransactionUpdated` handler from `src/lib/store.ts`.

## ~~(frontend/shell) — Hardcoded strings in shell components~~ ✅ resolved

`nav.design_system` key added; `shell.sidebar_collapse/expand/version` keys added. `Header.tsx` back button aria-label now uses `t("action.back")`. Both en + fr JSON updated.

## (frontend/accounts) — Extract row handlers into useAccountTable

`AccountTable.tsx` defines inline arrow functions inside the row `.map()`: `onClick`/`onKeyDown` on `<tr>` and `onClick` on action `IconButton`s. Move these handlers into `useAccountTable` to stabilise references and ease testing. See frontend-reviewer warning (account-page task).

## (backend) — Replace string-matching error assertions with structured error types

Backend errors are currently `anyhow::Error` strings (e.g. `"Cannot edit an archived asset"`, `"Cannot archive an asset with active holdings"`). Tests assert with `err.to_string().contains(...)` — fragile: wording changes silently break intent, and there is no structural match.
Introduce a domain error enum (e.g. `AssetError`, `TransactionError`) so tests can match on variants rather than substrings, and callers can handle errors programmatically without parsing strings.

## (app) — Add proper application icon

## ~~(frontend) — Save current view between sessions; start on the accounts page by default~~ ✅ resolved

`lastPath.ts` persists the top-level nav section (`/accounts`, `/assets`, `/categories`) to `localStorage`. `AppShell` saves on every navigation; `indexRoute.beforeLoad` restores on startup. Default is `/accounts`.

## (backend/frontend) — Add new financial asset metadata directly from the web
