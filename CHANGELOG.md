# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.33.1] - 2026-07-04

### Fixed

- overlay the fetch progress bar to avoid layout shift
  In-flow mount reflowed the page when a fetch started, making CI WebDriver
  clicks land on moved elements (auto_fetch intercept).

## [0.33.0] - 2026-07-04

### Added

- interest recording and editing UI
  INT-010/020/030/040: header entry, percent-or-quantity modal (cash line
  included), journal placeholders, URL-driven edit mount.
- add Interest transaction type (backend)
  INT spec: capitalized zero-cost quantity credit (FreeShares mechanics) on a
  fund line or the cash line; percent-of-holding or direct amount entry.
- shell price-fetch progress bar + coalesced view refresh
  MKT-180/181: dispatcher emits done/total progress; views skip per-asset
  re-fetches during a bulk fetch and reload once on completion (kills the
  3N-round-trip flicker).
- account forms toggle + hidden fee UI when fees disabled
  FEE-075/076: opt-in checkbox on add/edit; disabled accounts hide the fee
  button, Manage-fee action, Management Fees column and header total.
- add account-level management-fees gate (backend)
  FEE-075/077/078: new accounts default off, migration backfills existing on;
  record/create reject when disabled; catch-up pauses without cursor advance.
- show active fee-schedule rate on the holding line
- show net cash input since inception in account header
- add holding weight % column to account details

### Fixed

- upgrade quick-xml to 0.41 (RUSTSEC-2026-0194/0195)
  Direct dep bumped (unescape_value -> normalized_value); plist 1.9->1.10
  clears the last vulnerable transitive copy.
- interest review round (cash percent base, XOR gate UX)
  Addresses reviewer-backend/arch/frontend: cash-line percent interest now
  bases on the cash replay (was 2x-overstating past Purchases); submit gate
  keeps InterestAmountInvalid reachable; UL entry; visual proofs.

## [0.32.0] - 2026-07-01

### Added

- add management fee UI (modals, fees column, catch-up)
  One-off fee + recurring schedule modals consume the new fee surface;
  a Management Fees column and header total make deductions visible.
  The recurring catch-up fires once on app mount (FEE-040).
- add management fee deduction via quantity reduction
  Fees deducted as a periodic share-quantity reduction with no cash leg;
  cost basis is preserved so the average price concentrates (FEE-023).
  Recurring schedules apply lazily via catch-up generation on app open.

### Fixed

- close management-fee spec gaps (edit modal, TXL, reactivity)
  spec-checker flagged FEE-055/063/064 unimplemented on the FE: TXL fee
  rows showed 0.00 instead of —, editing a fee opened the generic money
  editor, and the view ignored FeeScheduleUpdated. Adds the dedicated
  edit-management-fee modal and backfills tests covering all 40 FEE rules.
- emit typed UpdateError on update:error event
  download/do*download now return Result<*, UpdateError> instead of anyhow (B31);
  each cause is logged server-side and the event carries the typed variant, not the
  raw error chain. The banner drops its raw-string field for the generic message.
- drop hardcoded French createLabel default in ComboboxField
  The createLabel default rendered an untranslated '+ Créer' to the DOM (F16). Drop
  the default and render the create entry only when both onCreateNew and a caller-
  supplied label are present. No visual impact — every live consumer passes both.

## [0.31.0] - 2026-06-29

### Added

- rework account details header into icon-button toolbar
  Replace the labelled buttons + Record dropdown with six big square icon
  buttons (tooltips name each action). The as-of date field leads, label-less
  and width-constrained, showing a "Today" placeholder in the live view.
  Header info trimmed to Global Value only — the now-unrendered summary fields
  and their presenter mapping/tests were removed as dead code.

## [0.30.0] - 2026-06-29

### Added

- add per-year annualized yield (CAGR) on performance page
  Each year row now shows the cash-flow-adjusted cumulative return annualized
  over the elapsed years (CAGR) — the equivalent constant annual rate, for
  comparing against a fixed-interest plan. Reuses the since-inception metric;
  sub-1-year periods report the cumulative as-is (no extrapolation).
- view live account details as of a past date
  A header date selector (default today) reconstructs the full view as of a past
  date (holdings, prices, FX, realized P&L, dividends) in read-only mode with a
  back-to-today banner. Retires the v0.29.0 holdings-as-of modal +
  get_account_holdings_as_of, folded into get_account_details.

### Fixed

- revert DateField to committed value on blur
  A half-typed entry emits "" and left stale partial text — indistinguishable
  from an external reset to "" (which the echo-skip guard can't re-sync). On
  blur, re-sync the display to the last committed value so no stale partial
  lingers.

## [0.29.0] - 2026-06-28

### Added

- add account value-over-time chart
  Plots the account's period-end Global Value over time on the performance
  page, reusing the existing performance data (no new backend command). Adds
  recharts; the line chart is themed with M3 CSS variables so it tracks
  light/dark. Chart dataset derived in a colocated hook (F10).
- view account holdings as of a past date
  New read-only get_account_holdings_as_of command reconstructs per-asset
  quantity + VWAP, price, and value as they stood on a chosen past date
  (reusing holding_snapshot_as_of + the performance price loader + as-of FX).
  Surfaced via a holdings-as-of modal launched from the account header.

### Fixed

- re-seed cash modal date on each open
  Deposit/withdrawal modals were always mounted, so their once-only date
  initializer read the stored last-operation date just at page load and went
  stale. Mount them only while open (like the buy/sell/dividend modals) so the
  hook re-seeds on every open. No visual change — modals render identically.
- persist price-modal date as last-operation date
  record() seeded the date field from the stored last-operation date but
  never wrote it back on success, unlike every other operation hook. Now
  calls setLastOperationDate on a successful record.
- open market-price asset combobox on focus
  The asset selector felt readonly: useFuzzySearch returns nothing below 2
  chars and the options panel was gated on that, so focusing showed no list.
  Now opens on focus with the full list (HeadlessUI immediate) and a chevron
  affordance. Drops the hook's unused displayValue/idKey/selectedId.

## [0.28.0] - 2026-06-27

### Added

- accept inline arithmetic in transactions number fields
  Swap numeric TextField → CalcField in the add-transaction page + modal, edit-transaction modal, and the account-journal amount filters (A3).
- accept inline arithmetic in account-details number fields
  Swap numeric TextField → CalcField in the buy, sell, dividend, price, deposit, withdrawal, free-shares, open-balance and edit-price forms (A3).
- add inline-arithmetic CalcField primitive
  evaluateArithmetic parses + - \* / and parentheses (no eval); CalcField shows a live '= result' hint and commits the result on blur, reporting the evaluated value while plain numbers pass through. Wired into fields in follow-up commits.
- price dialog fuzzy asset search + 'save & add another'
  The price modal's asset becomes a fuzzy combobox over the account's priceable holdings (pre-selected to the launched holding, switchable); the date seeds from the stored last-operation date; a 'Save & add another' button records and keeps the modal open. Amends MKT-011/012/013 + adds MKT-014. Visual-proof deferred to the pre-release todo.
- add "record & add another" to the dividend dialog
  A secondary button records the dividend, refreshes via a refresh-only onRecorded callback, clears amount + note, and keeps the modal open for the next entry (DIV-010). Visual-proof deferred to the pre-release todo.
- show as-of-date avg cost and potential P&L on trade dialogs
  Buy/sell dialogs show the holding's avg cost as of the trade date (TDI-020); the sell dialog also shows the typed sell's potential P&L (TDI-030). Visual-proof of the new modal info lines is a pre-release todo (interactive + needs IPC mocking).
- add as-of-date holding snapshot query
  get_holding_snapshot_as_of replays a (account, asset) pair's transactions up to a date to reconstruct quantity + VWAP cost (TDI-010). Read-only; powers the trade-dialog insights. Also corrects the contract's stale correct/cancel arg rows (now DTO-only).

### Fixed

- T8 review follow-ups — float formatting + colocate arithmetic
  CalcField commits formatResult() (not raw String) so float artifacts like 5.000000000000001 render and store as 5. Move arithmetic.ts next to its only consumer (gold for the new file).
- T5 review follow-ups — snapshot error pass-through + coverage
  useHoldingSnapshotAsOf surfaces the typed AccountError instead of dropping it (F27). Defensive qty clamp + FreeShares/Withdrawal/DatabaseError/cross-currency-P&L tests. Regenerate bindings so HoldingSnapshot.average_price reads account currency (was stale).

## [0.27.0] - 2026-06-23

### Added

- remember performance view mode per account
- per-period global value bridge on performance view

## [0.26.0] - 2026-06-23

### Added

- fuzzy-search combobox for dividend asset selector
- add dividends, P&L and cash columns to performance view
- remember closed-positions fold state per account

### Fixed

- order same-date journal rows by created_at

## [0.25.0] - 2026-06-22

### Added

- foldable closed positions, dividend cols, date price fetch
- Closed Positions section is collapsible (toggle + chevron).
- Closed positions table adds Dividends + Total Revenues (realized P&L + dividends).
- New "Prices at date" button/modal fetches each holding's Yahoo close at a picked date (carry-back), stored under that date via an isolated fetch path.

## [0.24.0] - 2026-06-21

### Added

- split performance value and % into separate columns

## [0.23.0] - 2026-06-21

### Added

- bank-statement cash columns + back nav in journal
  The Balance column is the true full-history cash balance at each row,
  shown even when filters hide cash-moving transactions — so it can jump.

## [0.22.0] - 2026-06-21

### Added

- account-wide transaction journal with filters
- expose get_all_transactions_for_account command
- prefill operation date from last entry per account

### Fixed

- render transaction journal dates in locale-numeric format
- stop DateField wiping input; add +/- day stepping
- replay holdings chronologically and refresh journal on edit
  recalculate_holding trusted callers to pass transactions in date
  order, so the oversell guard was order-dependent: a sell could be
  moved before its buy, then a reload flipped physical order and
  permanently blocked further edits. Sort the replay internally; the
  journal now refreshes on TransactionUpdated instead of on reload.

## [0.21.0] - 2026-06-20

### Added

- always-visible cash row; remove cash menu items
- eager cash line at account creation
- group holdings by class (cash, stocks, other)
- allow zero-cost positions in open balance
- open add-transaction in a modal, not a page
- replace add-transaction buttons with a FAB
- secondary sort by name on every column
- account-wide P&L and YTD on the overview
- current-value column replaces cost basis
- manual fill for unupdated prices

## [0.20.0] - 2026-06-15

### Added

- replace Stooq with keyless Yahoo Finance price source

## [0.19.0] - 2026-06-12

### Added

- free-shares modal + transaction-list rendering
  Record-menu entry + dedicated modal (no money inputs, asset locked on edit
  via the URL-driven shell mount) so a zero-cost distribution is recorded as
  quantity only. The TXL row dashes the money columns since no cash moved.
- optional keyless Stooq fetch mode
  Stooq's daily-download serves anonymously (PoW only) on some networks but
  is key-gated or IP-blocked on others, so neither mode fits everyone. A
  Settings toggle (default keyed) lets the user pick; the mode travels with
  each fetch request. ADR-016 supersedes ADR-015's keyed-only decision.
- record free-share distributions
  Bonus shares added at zero cost: quantity rises, cost basis is unchanged
  so the average price dilutes. No cash leg. Fully reversible via the
  transaction-log replay (FSD-028) — delete restores the holding exactly.

## [0.18.0] - 2026-06-11

### Added

- Connections dialog + price-refresh key gating
  URL-driven Connections dialog (?modal=connections) for BYOK key entry,
  test, and removal; the refresh buttons gate on a stored key (KEY-040) and
  launch auto-fetch skips silently without one (KEY-041). The KEY-012
  plaintext opt-in surfaces only when a save falls back to session memory.
- provider API-key storage + Stooq keyed fetch
  Stooq's key-less light-quote endpoint 404'd (L-006); a live probe showed
  the apikey does not bypass the proof-of-work gate, so the fetch path
  retains PoW and adds the key on q/d/l (ADR-015, supersedes ADR-008).
  New connection BC stores BYOK keys via the OS-keychain ladder (ADR-011).

### Fixed

- surface price-fetch outcome with a failure snackbar
  Price fetches failed silently — the user only learned from logs (worse
  now that Stooq's free endpoint 404s, L-006). The fetch task now emits
  AssetPriceFetchCompleted{ok,skipped} (MKT-119); the frontend shows a
  snackbar when any asset was skipped, staying silent on full success so
  launch auto-fetch stays quiet on the happy path (MKT-145).

## [0.17.4] - 2026-06-07

### Fixed

- normalize class-share slash for Stooq symbols
  OpenFIGI spells class shares with a slash (BRK/B), but Stooq resolves
  only the hyphen form (BRK-B.US) and returns N/D for the slash. The
  Stooq symbol derivation now translates / to -; the stored reference
  stays the canonical OpenFIGI ticker.

## [0.17.3] - 2026-06-07

### Fixed

- date auto-fetched prices by their quote date
  Auto-fetch stamped every price with today's date, so a weekend sync of
  Friday's close read "Updated today". The fetch now uses Stooq's quote
  date (MKT-117/118), falling back to today when it is absent, malformed,
  or in the future. Repeat non-trading-day syncs become idempotent.

## [0.17.2] - 2026-06-06

### Fixed

- solve Stooq proof-of-work anti-bot challenge
  Stooq replaced its User-Agent gate (L-003) with a JavaScript
  proof-of-work challenge served to all clients. Solve it, POST
  /\_\_verify for the auth cookie, retry once (cookie reused per launch).
  Bounded difficulty/token guard the solver against hostile input.
  Refs: #73
- make asset error translation exhaustive
  translate*asset_application_error used a `* =>` arm that would silently
  map any future AssetApplicationError variant to DatabaseError. An
  exhaustive match makes a new variant a compile error instead. Behavior
  unchanged today (AssetError exposes only DatabaseError).

## [0.17.1] - 2026-06-05

### Fixed

- set Stooq User-Agent to bypass anti-bot challenge
  Stooq serves a JS anti-bot challenge page (HTTP 200, text/csv) to
  clients without a browser User-Agent, which the CSV parser misread as
  a "close not numeric" error so prices silently stopped updating. Also
  guard against non-CSV bodies and document the failure mode (L-003).
  Refs: #69

## [0.17.0] - 2026-06-05

### Added

- wire live FX-staleness label (FXR-090)
  Completes the lone deferred FXR rule — the FX-staleness label was render-ready
  but always null because the valuation lift didn't surface the resolved rate's
  date. resolve*rate now carries the date through HoldingDetail.fx_rate_date to
  the account_details presenter (F26-safe: currency.rate_staleness*\* are i18n
  key strings, not a cross-feature import). spec-checker: FXR-090 PASS, 46/46.
- Currency Rates view + holding-row FX shortcut
  Frontend for the FXR feature — consumes the currency BC + valuation lift
  already on main. New currency feature (gateway/presenter/view/record+declare+
  delete modals); the account_details holding-row FX shortcut reaches the
  record-rate modal via URL params + a shell mount (no cross-feature import, F26);
  views subscribe to CurrencyRateUpdated; validateRateForm wired as inline hints.
- FX provider fetch (Frankfurter + ECB)
  Auto-fetches current FX rates for persisted pairs via the ADR-009 chain
  (Frankfurter JSON → ECB XML), computing non-EUR pairs by EUR cross-rate
  (FXR-080-083). The refresh piggybacks the existing asset price-fetch task
  and its in-flight guard (FXR-075/076). Provider responses are hardened
  against non-finite/negative values and i64 overflow before storage.
- multi-currency valuation lift
  Foreign-currency holdings are now valued live in account currency across
  account_details/summary/performance. Adds latest_rate_on_or_before (FXR-035)
  and CurrencyService::resolve_rate_micros, injected into the three orchestrators
  to lift the asset.currency==account.currency guards (FXR-030-035/040-042).
  Works on manual rates; provider fetch lands in PR2b.
- currency bounded context + manual rate CRUD
  First PR of the FX-rate feature (FXR). New `currency` BC in full gold
  layout (application/domain/infrastructure) with CurrencyPair + CurrencyRate
  aggregates, six manual-CRUD commands, latest-write-wins upsert (ADR-012),
  i64 micros (ADR-001), and the CurrencyRateUpdated event. Provider fetch and
  the valuation lift land in later PRs.
- view + edit cash transactions (CSH-110/111)
  Cash rows gain a View-transactions inspect action (CSH-110) reaching the
  existing per-asset list. Editing a Deposit/Withdrawal there reuses the
  dedicated cash modals in edit mode via correct_transaction, opened through a
  URL-driven shell mount so the transactions feature stays import-free of
  account-details (CSH-111). Delete already worked via cancel_transaction.

### Fixed

- the E2E critical path
  (declare/record/edit/delete) + docs closure (spec-checker PASS). The E2E
  surfaced two real defects fixed here — the pair-row drill-in was a mouse-only
  non-keyboard-accessible <tr> (now role/tabIndex/onKeyDown like AccountTable),
  and record-rate date used a raw text field, not the shared DateField.

## [0.16.0] - 2026-05-31

### Added

- cash dividend frontend (DIV)
  Adds the dividend modal (paying-asset picker + conditional FX) reached from a
  new consolidated header "Add" menu that absorbs Deposit / Withdraw / Open
  balance (DIV-012). Surfaces per-holding dividends + total return and a
  per-account dividend total. Registers the new UL terms in the dictionary.
- add cash dividend transaction backend (DIV)
  Dividend credits the account cash holding like Sell but leaves the paying
  asset's quantity and cost-basis untouched (DIV-024) and records no
  AssetPrice (DIV-027). Adds per-holding dividends_received / total_return_pct
  and per-account total_dividends_received to the account-details read model.
- price-refresh lock toggle on holding row
  Adds a Lock/LockOpen IconButton on each non-cash holding row (MKT-153)
  that calls the block/unblock command via the gateway, refetches assets
  (mirroring archive/unarchive), and confirms via snackbar. Icon state
  reads asset.price_refresh_blocked from the store. Stable id
  action-toggle-price-refresh-${assetId} for E2E. Adds i18n + tests.
- aggregate
  block/unblock methods, repo + service + two Tauri commands, and the
  build_scope exclusion (MKT-151) so a locked asset is never fetched.
  Regenerates bindings; existing FE Asset fixtures gain the new field.
- add price_refresh_blocked column (MKT-150)
  Adds the per-asset price-refresh lock flag (ADR-014). Bare ALTER matching
  the existing is_archived / isin migration style; defaults to not-locked.
- router-driven edit modal + row double-click
  The edit button and a new row double-click now open the shell-mounted
  Edit Asset modal via URL params, so it overlays in the asset-view
  context; drops the table's local modal mount and state. Double-click is
  a no-op on archived rows; the action-edit-asset id is preserved for E2E.
- edit asset on holding row double-click
  Double-clicking a holding opens the router-driven Edit Asset modal so it
  overlays in the account-details context. No-op on archived assets and
  cash rows, matching where editing is otherwise disallowed.
- trim trailing zeros on holding quantity
  Whole-share holdings rendered as "12.000000" are noisy; show "12" for
  whole numbers and only the significant fractional digits otherwise.
  Cash balances stay at fixed 2 decimals (a currency amount).
- show 3 price decimals below 10, 2 otherwise
  Sub-10 prices (penny stocks, low-value units) lose meaningful precision
  at 2 decimals; larger prices don't need a noisy third digit. Applies to
  average price, current price, and the price-history list.

## [0.15.0] - 2026-05-29

### Added

- performance page UI (PRF)
  Consumes the get_account_performance bindings merged in #53; no FE caching —
  the page just renders the recomputed series (ADR-013).
  Container/presentational split (AccountPerformanceTable) mirrors account_details
  for visual-proofability; errors flow through the I18nMessage F27 pipeline.
  Also drops the N+1 tech-debt note (triaged to non-issue: one-time O(assets) lookup).
- account performance backend (PRF)
  Per-account value-over-time computed on read (ADR-013) — no snapshot table,
  so backdated prices/transactions can never leave stale rows.
  Performance is net of deposits/withdrawals via Simple Dietz (PRF-030/032);
  foreign-currency holdings contribute 0 until FX ships.
  Bindings regenerated but not yet consumed (BE-first of a 3-PR split).
- widen keyword lookup to ETP, bonds, funds, crypto, REIT
  OpenFIGI /v3/search accepts only a single securityType string, so the
  old "Common Stock" filter blocked ETPs, bonds, mutual funds, REITs, and
  crypto from keyword results. Dropped the request-time filter; relies on
  post-classification (WEB-023) + priority sort (WEB-048) for relevance.
  Also corrected map_security_type vocabulary to OpenFIGI's actual values.
- click-to-edit on missing-ticker diagnostic
  MKT-032 Interactivity: "Missing ticker" becomes a stable-id button
  (action-edit-missing-ticker-{assetId}) that fires URL search params.
  A shell-level AssetEditModalMount reads modal/editAssetId/focusField
  from the URL and overlays EditAssetModal with the reference input
  focused — no cross-feature import from account_details to assets.
- typed price-missing states (MKT-032)
  Replaces unannotated "—" in the holdings price cell with typed
  diagnostics: "Missing ticker" when asset_reference is empty,
  "No price available" otherwise. Informational-only (non-interactive);
  click-to-edit deferred pending the URL-driven modal pattern.
  Amends MKT-032; reconciles MKT-140 cell-composition.

### Fixed

- stable ids on nav, FABs, row actions, and header buttons
  Sweeps locale-coupled aria-label/XPath selectors across 9 e2e files,
  replacing them with stable id selectors per E1. FE additions:
  nav-{path} on Sidebar items, fab-add-{asset,account} on FABs,
  per-row action-{verb}-{entity}-{id} on HoldingRow, AssetTable, and
  AccountTable. Resolves the 2026-05-18 + 2026-05-25 E4 techdebt entries.
- enforce AST-006 archive guard on price-mutation commands
  record_asset_price, update_asset_price, and delete_asset_price now
  reject archived assets with the typed Archived variant. Reads
  (get_asset_prices) stay available — archive blocks mutations only.
  Resolves the long-parked AST-006 enforcement decision via the
  shipping path (new helper ensure_asset_writable_for_price).
- IE country prefix covers UCITS ETF venues
  ISIN_COUNTRY_TO_PRIMARY_VENUES["IE"] extended from ["ID"] to
  ["ID", "LO", "NA", "GY"]. Dublin still wins for Irish equities
  (Ryanair, CRH, Kerry Group); UCITS ETFs (domiciled in Ireland but
  trading on LSE / Amsterdam / Xetra) now get a coherent fallthrough
  instead of landing on GLOBAL_VENUE_PRIORITY's Amsterdam-first default.
- stable id selectors + pre-release E2E gate guardrail
  The E2E was broken on main since two earlier renames (Open Balance → Add a
  position; two-field web lookup) but stayed green at PR time because the
  workflow only fires on `main` push. Patching the text-XPath would just rerun
  the same fragility class on the next rename; stable ids fix it durably.
  CLAUDE.md update ensures the release tag doesn't hide future breakage.

## [0.14.0] - 2026-05-24

### Added

- add optional ISIN field alongside ticker reference
- two-field web lookup with ISIN format gate
  SearchPanel now exposes one input + submit per path (ISIN and
  Keyword). Loading spinner and inline error anchor to the field
  that triggered the action so the other path stays usable; new
  presenter maps InvalidIsinFormat to a field-local copy key.
- explicit ISIN/keyword lookup mode + format validator
  Auto-routing by query shape is replaced by an explicit
  `mode: LookupMode { Isin, Keyword }` so the FE can drive path
  selection. ISIN path adds ISO 6166 validation (WEB-016) and a
  tighter 3-venue cap per share class (WEB-050e). FE two-field UI
  follows in a subsequent commit.
- show per-account global value on accounts list
  ACC-021 surfaces CSH-094's per-account economic value (cash + same-
  currency priced holdings) on the Accounts list, alongside the name +
  frequency columns. New use_cases/account_summary/ wraps Account with
  total_global_value via a dedicated IPC, keeping the existing
  get_accounts cheap for dropdown callers that don't need the value.

### Fixed

- i18n ModalContainer close-button aria-label (F24)
- thread i18n locale through formatIsoDate + compact price cell
- rename Open Balance CTA to Add a position
- guard account_details_view hooks with try/catch fallback
  usePriceModal.handleSubmit and useAccountDetails.fetchDetails called
  the gateway without try/catch — a throw would leave isSubmitting /
  isLoading stuck true and never set an error. Both now match the
  canonical pattern used by the sibling hooks (UNKNOWN_ERROR fallback,
  flag cleared in finally). Closes two 2026-05-23 techdebt entries.
- clip Dialog to viewport so tall modals scroll inside
  E2E in CI's 800x600 viewport reported the AddAsset submit button
  as "not interactable" — Dialog had no max-height and content
  overflowed the screen, with the footer pushed below the viewport.
  max-h-[calc(100vh-2rem)] + flex-1/min-h-0 lets the content scroll
  within bounds; matches the FormModal pattern.

## [0.13.0] - 2026-05-23

### Added

- add ETP class for OpenFIGI umbrella securityType
  ETFs like Amundi PEA MSCI World (FR001400U5Q4) come back from OpenFIGI
  with securityType=ETP, which our previous mapping didn't know — user
  saw "Unknown type". OpenFIGI uses ETP as umbrella for ETF/ETN/ETC and
  doesn't expose the distinction, so we surface the same umbrella. Users
  can edit the class manually for finer granularity.
- raise lookup caps to 10/share-class, 30 total
  Previously the per-share-class cap of 3 hid the primary venue for
  single-company queries (e.g. ASML keyword showed London/Xetra/Milan
  but never Amsterdam — its actual primary). Bumping per-class to 10
  and total to 30 lets the user scroll a longer list and pick the
  right venue. Spec WEB-022 updated to match.

### Fixed

- strip diacritics from OpenFIGI lookup query
  OpenFIGI's name index is unaccented — "Société Générale" returns 0
  hits while "Societe Generale" returns 100. NFD-normalize the query
  before WEB-014 routing so accented inputs find their matches. Handles
  all Latin diacritics (é/è/ê/ç/à/ô/ñ/ü/…) via combining-mark removal.
  Spec adds WEB-015 alongside existing WEB-014 routing rule.
- map LO and SQ OpenFIGI codes to XLON/XMAD
  OpenFIGI exposes some venues under two short codes — LSE as both LN
  (lit primary) and LO (consolidated), Madrid as both SM (Continuous)
  and SQ (BME composite). Mapping only one in each pair left priority-
  walker picks of LO/SQ unlabelled (e.g. ENGIE 0LD0, SANTANDER SAN).
- F27 fetch-price presenter for refresh hooks
  useRefreshGlobalPrices + useRefreshAccountPrices were switching on
  result.error.code directly, bypassing F27. Adds fetchPriceErrorToI18n
  covering FetchAccountAssetPricesError + FetchAllAssetPricesError, plus
  a SnackbarMessage type at src/ui/format/i18n.ts for hooks that dispatch
  via snackbar rather than rendered error state.
- F27 pipeline → account_details + transactions
  Hooks in account_details + transactions still stringified typed errors,
  losing Oversell.available, InsufficientCash.current_balance_micros,
  and InvalidDateFormat.date payloads at the hook layer. Adds two per-BC
  presenters covering HoldingTransactionError + AssetPriceError unions
  with payload interpolation in en/fr; boyscout-fixes useEditPrice.
- restore F27 typed-error pipeline across 3 features
  Hooks were stringifying typed errors to `error.${code}` before the F27
  presenter ever ran, dropping per-variant payloads (InvalidExchange,
  InvalidCurrency). Adds three per-BC presenters + canonical I18nMessage
  at src/ui/format/i18n.ts, payload interpolation in en/fr, and
  exhaustive variant tests across assets, categories, accounts.
- pin sqlx-cli to 0.8.6 to match sqlx 0.8
  taiki-e/install-action with `tool: sqlx-cli` (no version) drifted to
  0.9.0, which requires DATABASE_URL for `prepare --check` even with
  SQLX_OFFLINE=true. Match the runtime dep version explicitly.

L-001 (Tauri bundler walks src/bin/) and L-002 (unpinned install-action
tools drift) codified in docs/lessons.md.

## [0.12.1] - 2026-05-22

### Fixed

- stop Tauri bundler probing for phantom .exe
  Tauri's NSIS bundler walks src-tauri/src/bin/ on disk and expects
  every entry to produce a bundled .exe — required-features gating
  (b89b343) didn't help, breaking v0.11.0 and v0.12.0 Windows builds.
  Moving the dev tool to src-tauri/dev/ keeps it buildable while
  hiding it from the bundler. Mirrors PatientManager d79a245.

## [0.12.0] - 2026-05-20

### Added

- exchange picker in Add/Edit forms + i18n
  Picker is feature-scoped Zustand cache, session-static — curated list
  loads once via gateway on first mount. Boyscout: modals now `t()` the
  error string so InvalidExchange (and pre-existing variants) display.
- add Exchange value object + BE wiring
  Exchange is a first-class domain field, decoupled from any web provider.
  Inbound `openfigi_mic_to_exchange` and outbound `exchange_to_stooq_suffix`
  mappers form the anti-corruption layer; providers stay swappable.
- MKT auto-fetch frontend UI + settings toggle (#30)
- MKT auto-fetch backend (Stooq) + source field (#29)
- surface primary listing per share class
  OpenFIGI's keyword search never returns the primary venue for a name
  search; it floods results with OTC and trade-reporting duplicates of the
  same share class. Adds a second /v3/mapping call to recover the primary
  listing, and isolates the opinionated venue-priority and exchange-name
  tables in a dedicated processor module for auditability.

### Fixed

- surface OpenFIGI 429 as typed RateLimited error
  Reversed WEB-025's "intentionally closed" decision: 429 deserved
  distinct user-facing copy ("wait and retry") instead of the opaque
  generic network error that hid the actual fix from the user.
- honour Asset.exchange in Stooq symbol derivation
  The price-fetch orchestrator was still calling the bare-ticker helper,
  so the picker had no production effect for non-US assets. Wired to
  derive_stooq_symbol_with_exchange + integration test guards the path.
- small fixes bundle (CVE + domain hardening) (#25)
- add error.DatabaseError key for new gold infra wire shape
  Frontend reviewer caught the missing key. Without it, the new
  { code: "DatabaseError" } wire shape from PR 6's Category CRUD
  migration would have rendered the raw key string to users on any DB
  failure. Surgical fix tied directly to the wire-shape change.
- preserve typed AccountNotFound in ensure_cash_for
  PR 3 routed buy/sell/correct/cancel through ensure_cash_for_typed,
  which opaqued every error to Infrastructure(Unknown) — including the
  in-account AccountNotFound that buy_holding etc. previously surfaced
  as typed AccountNotFound { account_id }. Promote ensure_cash_for to
  typed Result; opaque only true cross-BC asset-side failures.

## [0.11.0] - 2026-05-08

### Added

- cash + priced
  same-currency holdings, no FX.
- `OpeningBalance` rejected on the Cash Asset (CSH-061).
- Migration `202605060001` is documentation-only —
  `transaction_type` is TEXT.

Frontend

- New Deposit / Withdrawal modals + hooks mirroring Buy/Sell, with a
  shared `validateCashForm` helper (CSH-021/031). Withdraw + Buy/Edit
  surface `InsufficientCash` inline with localised balance + currency
  (CSH-081); `useTransactions` formats the payload-bearing variant
  centrally.
- Account Details: Deposit (always) and Withdraw (gated on cash > 0)
  buttons in the header, Global Value tile, and a no-cash banner.
- Cash row variant in the active holdings table — sorted to the top
  (CSH-092), no cost-basis / avg-price / realized-pnl cells, with
  inline Deposit / Withdraw actions (CSH-091).
- Suppress system Cash Assets in the Asset Manager and the Add / Edit
  / Open-Balance asset selectors (CSH-015/018/061), and the Cash
  Category from category lookups (CSH-017).
- Transaction list renders Deposit / Withdrawal type labels (CSH-101).
- New `useAccountDetailsView(accountId)` hook absorbs modal state +
  handlers + derived flags so `AccountDetailsView.tsx` is pure JSX +
  one hook call.
- New cash, validation, and `error.*` i18n keys in EN + FR.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>

### Fixed

- stop shipping generate_bindings dev tool in installer
  The dev-only generate_bindings bin was packaged into Windows NSIS
  installers since at least v0.8.x. Gate it behind a Cargo feature so
  cargo build skips it; recipes and scripts opt in via
  --features generate-bindings. Release workflows also strip any stale
  binary restored from rust-cache before tauri-action runs.
- reject mutations on system Cash Asset (CSH-016)
  update_asset / archive_asset / unarchive_asset / delete_asset now reject
  inputs whose target asset has class == Cash with CashAssetNotEditable.
  archive_asset and delete_asset additionally surface NotFound for unknown
  ids (the new guard loads the asset). FE filtering (CSH-015/018) already
  prevents these calls in practice; this closes the direct-IPC gap.
- the scrim covers the whole viewport at 50%
  black + blur, and in a standalone preview there is no real content
  behind it, so dark-mode shots looked near-black and misrepresented
  the component. The panel-only pattern lets dark mode show the proper
  m3-surface-container tone behind the dialog.
- Documented the panel-only pattern in CLAUDE.md (project-specific
  visual-proof note).
- Kit-level update tracked separately on phileggel/claude-kit.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>

- wire afterTest screenshot hook in wdio.conf.ts
  Captures a PNG to screenshots/e2e-failures/ on every failed test for
  post-mortem diagnosis. Filename: {suite}-{test}-{ISO-timestamp}.png.
  Also fixes pre-existing bug where SIGINT handler set undeclared `exit`
  instead of `cleanShutdown`, causing spurious "exited unexpectedly" logs.
- make E2E suite green for WebKit/HeadlessUI
  Replace flaky `*=` selectors with XPath, redesign tests around the
  HeadlessUI ComboboxField boundary (no automation in WebKit), and add
  data-testid + aria-label to modal close buttons. New ADR-007 documents
  the combobox limitation. All 18 E2E tests pass.
- remove spin buttons from numeric transaction inputs
- replace hardcoded placeholders in remaining modals
- replace hardcoded placeholders in buy and sell modals
- replace string-sentinel with typed AccountNotFound error

## [0.10.0] - 2026-05-03

### Added

- improve web lookup with Derivatives class and exchange
  Add Derivatives AssetClass (default_risk=5); maps Warrant/Option/Future/Rights (WEB-023)
  Add exchange field to AssetLookupResult; resolved from exchCode via static table (WEB-049)
  Sort results by AssetClass priority before 10-item truncation (WEB-048)
  Two-line result row in SearchPanel: code+name / type·exchange (WEB-031)
  Add formatAssetClass() presenter for human-readable class labels

### Fixed

- address PR review comments on exchange_code and i18n

## [0.9.1] - 2026-05-03

### Fixed

- use constant default for created_at ALTER TABLE
  SQLite rejects non-constant expressions (e.g. datetime('now')) in
  ALTER TABLE ADD COLUMN. This caused all app launches to fail with
  "Database initialization failed" since 2026-04-26.

Existing rows get the epoch sentinel '1970-01-01T00:00:00Z', which
sorts before any real timestamp and is correct for ORDER BY date, created_at.

## [0.9.0] - 2026-05-03

### Added

- isolate E2E tests with an ephemeral SQLite database
- implement frontend for TRX-042–058
  OpenBalanceModal + useOpenBalance hook; openHolding in gateway;
  Open Balance button in AccountDetailsView (TRX-055);
  OpeningBalance edit support in EditTransactionModal (TRX-051);
  i18n en+fr; TRX-046 future-date guard; presenter + edit modal
  unit tests (TRX-051–054)
- implement opening-balance transaction type

### Fixed

- change default tracing log level from debug to info
- harden wdio.conf.ts and align buy_sell note field
- Append Date.now() suffix to E2E_DATA_DIR to prevent collision on concurrent runs
- Wrap mkdirSync in try/catch with actionable error message
- Rename `exit` variable to `cleanShutdown` to avoid shadowing process.exit
- Change note: null to note: "" in seedBuy for Tauri 2 null-deserialization safety
- apply reviewer findings across frontend and backend
- apply reviewer findings and update docs
  Reviewer findings: Math.floor (TRX-047), TRX-055 button always visible,
  TRX-046 edit date guard, TRX-058 snackbar test, mount log removed,
  gateway.test.ts moved to feature root, i18n placeholder keys.
  Docs: plan ticked, roadmap row, todo tech-debt items, UL confirmed.
- use platform native root CAs for OpenFIGI HTTPS on Windows
- resolve error.undefined on account creation and window visibility

## [0.8.1] - 2026-05-01

### Fixed

- show empty state before error in account/asset/category tables

## [0.8.0] - 2026-05-01

### Added

- add web lookup dialog before Add Asset form
  Introduces WebLookupModal, SearchPanel, useWebLookupSearch, useWebLookupModal hooks.
  Extends useAddAsset with prefill?: AssetLookupResult (WEB-041/042).
  Wires AssetManager to open WebLookupModal instead of AddAssetModal.
  Adds i18n keys (en + fr) for the web_lookup block.
- add search_asset_web command and OpenFIGI client
- add price history CRUD commands and modal
- add pre-deletion summary dialog for non-empty accounts
  ACC-019: show holding+tx counts before deleting a non-empty account
  ACC-020: new get_account_deletion_summary Tauri command
  Fix ConfirmationDialog to not auto-close after async onConfirm (R13)
  Use tokio::try_join! for parallel count queries in service
- add UoW infrastructure foundation (Phase 5)
- add auto-record price checkbox and settings toggle
  Settings page gains a global auto-record toggle persisted in
  localStorage. Buy/sell/edit forms gain a RecordPriceCheckbox
  whose default snapshots the global toggle on create (always OFF
  on edit, MKT-052). 18 new tests; 6 new i18n keys (en + fr).
- auto-record asset price from transaction
  CreateTransactionDTO gains record_price: bool. RecordTransactionUseCase
  wires Arc<AssetService>; orchestrator upserts AssetPrice in the same
  DB tx and notifies after commit (MKT-055..062). 9 new tests.
  Frontend hooks default to record_price=false (UI wiring lands next).
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

- abort E2E run when tauri build fails in onPrepare
- address reviewer-infra findings on E2E infrastructure
- guard isSubmitting reset in finally blocks
- reject archived-asset sell on update
- clear DateField parent state on invalid typed input
- replace hardcoded strings with i18n keys
- fix SEL-011 account field and SEL-026 average price retention
  SEL-011: add read-only Account field to SellTransactionModal
  SEL-026: preserve average_price (last known VWAP) when holding quantity reaches zero, per TRX-040

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
