# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## ~~(backend/assets) — Delete eligibility guard~~ ✅ resolved

`DeleteAssetUseCase` blocks hard-delete when any transaction references the asset. Mirrors `ArchiveAssetUseCase` pattern. `delete_asset` Tauri command now routes through the use case. Two tests added.

## (kit) — Back-fill IPC contracts for all existing domains

The kit workflow now requires `docs/contracts/{domain}.md` (derived via `/contract` from the validated spec). No contracts exist yet. Run `/contract` for each domain in order:

1. `asset` — spec: `docs/spec/asset.md`
2. `account` — spec: `docs/spec/account.md`
3. `transaction` — specs: `docs/spec/financial-asset-transaction.md` + `docs/spec/sell-transaction.md`
4. `account_details` — spec: `docs/spec/account-details.md`
5. `record_transaction` — spec: `docs/spec/financial-asset-transaction.md` + `docs/spec/sell-transaction.md`
6. `update` — spec: `docs/spec/update.md`

After each `/contract`, run `contract-reviewer` to validate before moving on.

## ~~(frontend/assets) — F22: AssetTable imports AddTransactionModal (cross-feature)~~ ✅ resolved

Import fixed to `@/features/transactions` (public index).

## ~~(frontend/assets) — SortIcon defined inside AssetTable body (remounts on every render)~~ ✅ resolved

Replaced with `<SortIcon>` from `@/ui/components/SortIcon` (shared component).

## ~~(frontend/account_details) — F22: AccountDetailsView imports AddTransactionModal (cross-feature)~~ ✅ resolved

Import fixed to `@/features/transactions` (public index).

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-03-29): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## ~~(frontend/ui) — Migrate StatCard.tsx to M3 tokens~~ ✅ resolved

Replaced `bg-emerald-100/text-emerald-700` with `m3-success-container/on-success-container` and `bg-rose-100/text-rose-700` with `m3-error-container/on-error-container`.

## ~~(frontend/ui) — Remove structural borders from ManagerLayout and ManagerHeader~~ ✅ resolved

Removed `border-b border-m3-outline/5` from `ManagerHeader`. `ManagerLayout` already used `shadow-elevation-1`.

## ~~(frontend/ui) — Create TextareaField component~~ ✅ resolved

`TextareaField` created at `ui/components/field/TextareaField.tsx`; replaces raw `<textarea>` in `AddTransactionModal`, `EditTransactionModal`, `AddTransactionPage`, `BuyTransactionModal`, and `SellTransactionModal`.

## ~~(frontend/transactions) — Success snackbar feedback (transactions)~~ ✅ resolved

`useAddTransaction` and `useEditTransactionModal` both call `showSnackbar(t("transaction.success_created/updated"), "success")`. Snackbar infra (`snackbarStore.ts` + `Snackbar.tsx`) is in place and mounted in `MainLayout`.

## ~~(frontend/assets) — Success snackbar feedback~~ ✅ resolved

All asset mutations (create, update, archive, unarchive, delete) call `showSnackbar` in `useAssets.ts`.

## (frontend/shell) — Implement Settings page and wire the Settings button in Sidebar

The Settings button in `Sidebar.tsx` footer is wired via `onSettingsClick?` but `MainLayout` does not yet pass that handler.
Create the Settings page (feature `settings/`) and pass `onSettingsClick` from `MainLayout`.

## ~~(frontend/transactions) — Buy button disabled for archived assets~~ ✅ resolved by TRX-029

The "Buy" button stays active for archived assets: TRX-029 (auto-unarchive confirmation dialog) is the chosen behaviour. Disabling the button would contradict that flow.

## ~~(frontend/transactions) — TRX-010: "Add Transaction" button in Account Details view~~ ✅ resolved

`IconButton` per row in `AccountDetailsView` navigates to `/accounts/$accountId/transactions/$assetId` (TXL-010, ACD-042).

## ~~(frontend/transactions) — TRX-035: delete transaction confirmation dialog~~ ✅ resolved

`TransactionListPage` exposes the delete action via `ConfirmationDialog` (TXL-040, TXL-041). Existing i18n keys used.

## ~~(spec/account) — Add `currency` field to the Account entity~~ ✅ resolved

`Account` domain, migration, repository, service, API, bindings, account form, and both transaction modals updated. Exchange rate field now compares `asset.currency !== account.currency` (TRX-021 fixed).

## ~~(sell/SEL-036) — Exchange Rate field visibility in SellTransactionModal~~ ✅ resolved

`AccountDetailsView.tsx` now uses `account.currency` instead of `"EUR"` in `showExchangeRate` (resolved together with TRX-021).

## (frontend/transactions) — TRX-038: implement holdings display

`useTransactionStore.refreshHoldings()` is a stub: the backend has a `holdings` table (populated by `RecordTransactionUseCase`) but there is no `getHoldings` Tauri command.
Create `get_holdings(account_id) -> Vec<Holding>` in `use_cases/record_transaction/api.rs` (or `context/account/api.rs`) and use it in `store.ts` to display positions per account.

## ~~(frontend/shell) — Rename useSidebar.ts to navItems.ts~~ ✅ resolved

Renamed to `navItems.ts`; updated imports in `Sidebar.tsx` and `useHeaderConfig.ts`.

## ~~(frontend/shell) — Add missing mount logs (F13)~~ ✅ resolved

Added `useEffect` + `logger.info` in `Sidebar.tsx` and `DesignSystemPage.tsx`.

## ~~(frontend) — i18n for navigation labels and shell~~ ✅ resolved

`NAV_ITEMS` labels and app name migrated to i18n (`nav.*`). App renamed VaultCompass.

## (frontend/shell) — Hardcoded strings in shell components (pre-existing i18n debt)

`useSidebar.ts` — label values "Assets", "Accounts", "Categories", "About", "Design System" are hardcoded English strings rendered in three places: sidebar nav text, aria-label on nav buttons, and the `<h1>` page title via `Header.tsx`. Should be i18n keys resolved with `t()`.

`Header.tsx` — `resolveTitle()` returns raw `item.label` strings, so the page title is hardcoded English. Fix is coupled to the `useSidebar.ts` fix above.

`Sidebar.tsx` — "Version: " prefix (expanded sidebar) is a hardcoded English string; should be `t("shell.sidebar_version", { version: appVersion })`.

## (frontend/accounts) — Extract row handlers into useAccountTable

`AccountTable.tsx` defines inline arrow functions inside the row `.map()`: `onClick`/`onKeyDown` on `<tr>` and `onClick` on action `IconButton`s. Move these handlers into `useAccountTable` to stabilise references and ease testing. See frontend-reviewer warning (account-page task).

## ~~(backend/assets) — Implement archive eligibility guard (OQ-6): block archiving with active positions~~ ✅ resolved

Implemented via `ArchiveAssetUseCase` (cross-context use case) that checks `HoldingRepository::has_active_holdings_for_asset()` before delegating to `AssetService::archive_asset()`.

## (backend) — Replace string-matching error assertions with structured error types

Backend errors are currently `anyhow::Error` strings (e.g. `"Cannot edit an archived asset"`, `"Cannot archive an asset with active holdings"`). Tests assert with `err.to_string().contains(...)` — fragile: wording changes silently break intent, and there is no structural match.
Introduce a domain error enum (e.g. `AssetError`, `TransactionError`) so tests can match on variants rather than substrings, and callers can handle errors programmatically without parsing strings.

## (app) — Add proper application icon

## (frontend) - Save current view between session. Start on the account page by default.

## (backend/frontend) - Be able to add a new financial asset metadata directly from the web