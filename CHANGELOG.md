# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-04-26

### Added

- add Settings page with language override
- auto-detect system language and locale-aware number format
- add locale-aware number formatting for display values
- add market price entry and unrealized P&L display
- persist last visited section across sessions
- add closed position history
- guard delete against existing transactions
  DeleteAssetUseCase blocks hard-delete when any transaction references
  the asset. Mirrors ArchiveAssetUseCase pattern. SQLx cache updated.
- add buy-from-holding-row modal
  Buy (+) on holding row opens BuyTransactionModal instead of navigating to /transactions/new.
  Mirrors SellTransactionModal pattern (TRX-041). Modals moved to account_details/ (use-case boundary, fixes F22).
  IconButton gains success/error tonal variants. HoldingRow extracted. try/finally, useMemo, useCallback fixes.
- add currency field to Account entity
  Migration, domain, repository, service, API, bindings, account form,
  transaction modals. Exchange rate field now compares asset.currency vs
  account.currency (TRX-021, SEL-036).
- add archive eligibility guard (OQ-6)
  HoldingRepository.has_active_holdings_for_asset checks quantity > 0 across all accounts.
  ArchiveAssetUseCase guards then delegates to AssetService, keeping contexts isolated.
- implement sell transaction frontend with P&L display
- implement sell transaction backend with realized P&L
- improve account list page UX
- move back button and title into shell header

### Fixed

- reject archived-asset sell on update
- clear DateField parent state on invalid typed input
- replace hardcoded strings with i18n keys
- fix SEL-011 account field and SEL-026 average price retention
  SEL-011: add read-only Account field to SellTransactionModal
  SEL-026: preserve average_price (last known VWAP) when holding quantity reaches zero, per TRX-040

## [0.6.0] - 2026-04-19

### Added

- add /transactions/new page as DDD entry point
- add transaction list page
- add get_asset_ids_for_account command
- add toast notification infrastructure
- reorder nav, translate labels, add About modal
- Reorder drawer: Accounts, Assets, Categories
- Translate nav labels via i18n (nav.\* keys)
- Replace /about route with AboutModal triggered from sidebar
- Add app description and license to About modal
- add TanStack Router with hash history
  ACD-011: replace useState-based nav with URL routing.
  Routes: /assets, /accounts, /accounts/$accountId, /categories, /about.
  Back button and direct linking now work via hash history.
- implement account details view with cost basis
  ACD-010 to ACD-041: holdings list, cost basis, loading/empty/error states
- add purchase transactions with VWAP and server-side total
  Backend: CreateTransactionDTO omits total_amount (TRX-026); orchestrator
  computes it via compute_total. Frontend: computeTotalMicro for display
  preview only. VWAP recalculation on create/update/delete (TRX-030).
  Atomic DB transaction for transaction + holding upsert (TRX-027).

### Fixed

- correct VWAP cost basis calculation
- Fix double MICRO division in VWAP numerator
- Add fees to cost basis via total_amount (TRX-030)
- Align VWAP formula with TRX-026 total_amount
- Update spec TRX-030 to reflect correct formula
- fix asset prefill on buy modal and action.select i18n key
  AddTransactionModal was always mounted so useState init ran once with
  undefined prefillAssetId; fix via key prop on modal (TRX-011).
  fees default changed from "0" to "" to match quantity placeholder.
  action.select i18n key added to fr and en locales.

## [0.5.0] - 2026-04-05

### Added

- remove unused footer and settings

## [0.4.1] - 2026-04-05

### Fixed

- updater issues

## [0.4.0] - 2026-04-05

### Added

- implement account CRUD with full backend and frontend

### Fixed

- stage Cargo.lock in release commit

## [0.3.0] - 2026-04-04

### Added

- add auto-update feature with banner and about page
  Backend: tauri-plugin-updater, use_cases/update_checker (check/download/install
  commands). R18 fix: emit db:migration_error event instead of panicking so the
  frontend error screen is reachable. UpdateState managed before DB init.
  Frontend: update banner state machine (idle→available→downloading→ready/error),
  about page manual check (R25-R27), shell gateway, migration loading/error screens.

## [0.2.0] - 2026-03-29

### Added

- add archive/unarchive, reference validation and UX improvements
- Implement R1-R20: mandatory reference, archive/unarchive, duplicate warning,
  load-error state with retry, isSubmitting guard, showArchived toggle
- Rename factory methods: update_from->with_id, from_storage->restore (B1)
- Store fetches all assets (active+archived); AssetTable filters by showArchived
- Add tests: validateAsset (R9), presenter (R11); fix no-op R9 test in useAddAsset

## [0.1.0] - 2026-03-29

### Added

- Initial release — portfolio management desktop app (Tauri 2 + React 19 + Rust)
- Asset CRUD with categories, currency, risk level, and asset class
- Category management with system-protected default category
- Dashboard scaffold
- i18n support (fr / en)
