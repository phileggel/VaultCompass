# Ubiquitous Language

The authoritative dictionary of domain terms for this project.

> ⚠️ **Every addition or change to this document MUST be individually reviewed and validated
> by the user before it is considered confirmed.** Agents may propose additions (using status
> `confirmed`) but MUST NOT mark any term or entry as `confirmed` without explicit user approval.

**Rules:**

- All terms MUST be agreed with the user before use in code, tests, or docs.
- The agent MUST NOT invent or assume domain terms — propose and wait for confirmation.
- Terms marked `confirmed` are proposals awaiting user validation.
- Once confirmed, the term MUST be used consistently everywhere (code, comments, specs, UI).

---

## Account Context

### Account

The top-level aggregate root. Represents a financial account (e.g. brokerage, savings).
Owns all holdings and their transaction history.

> Status: confirmed

### Holding

An internal entity of `Account`. Represents the current position in a given asset within
an account — quantity held, average price (VWAP), and realized P&L.

> Status: confirmed

### Transaction

An internal entity of `Account`. A single financial event — a purchase or a sale — with
date, quantity, price, fees, and exchange rate. Owned directly by `Account` alongside
`Holding`; a transaction affects its corresponding holding but is not nested inside it.

> Status: confirmed

---

## Asset Context

### Asset

The aggregate root of the asset context. Represents a financial instrument
(stock, ETF, bond, etc.) with a currency, class, category, and risk level.

> Status: confirmed

### AssetPrice

An internal entity of `Asset`. A price observation for an asset on a given date, with a `source` field (see `AssetPriceSource`) qualifying its provenance.

> Status: confirmed

### Observation date

The date a price is _for_ — the trading day its value reflects — as distinct from when the price was fetched or recorded. An auto-fetched price uses the provider's reported date (e.g. Friday's close still seen on a Sunday); a manually entered price uses the date the user picks.

> Status: confirmed

### AssetPriceSource

A value-object enum qualifying the provenance of an `AssetPrice` record. Variants: `Manual` (user-entered via manual entry or transaction auto-record) and `YahooFinance` (keyless auto-fetch, ADR-017). Metadata for traceability per ADR-012 — does not influence read/write precedence (latest-write-wins).

> Status: confirmed

### Exchange

A canonical reference to a trading venue, independent of any market-data provider. Carries an ISO 10383 Market Identifier Code (MIC) as `code` (e.g. `XPAR`, `XNAS`) and a human-readable `label`. Optional field on `Asset`. Auto-filled by the OpenFIGI lookup path (WEB-049) or selected by the user via a curated picker on the Add/Edit Asset form (AST-021). Used by the auto-fetch task to resolve the Yahoo Finance provider symbol (MKT-110). Provider symbols (Yahoo venue suffixes, OpenFIGI exchange codes) are NOT stored on `Exchange` — they are resolved by per-provider mappers at the boundary.

> Status: confirmed

---

## Aggregate Root Methods (Account)

| Name                  | Intent                                                                                                  | Status    |
| --------------------- | ------------------------------------------------------------------------------------------------------- | --------- |
| `buy_holding`         | Record a purchase of an asset into the account                                                          | confirmed |
| `sell_holding`        | Record a sale of an asset from the account                                                              | confirmed |
| `correct_transaction` | Correct the fields of an existing transaction (cascades VWAP/P&L recalculation on the affected holding) | confirmed |
| `cancel_transaction`  | Delete an existing transaction (cascades VWAP/P&L recalculation or holding removal)                     | confirmed |
| `open_holding`        | Seed an existing position with a quantity and total cost at a given date, without full purchase history | confirmed |

## Transaction Types

| Name             | Intent                                                                                                   | Status    |
| ---------------- | -------------------------------------------------------------------------------------------------------- | --------- |
| `Purchase`       | A regular buy transaction — quantity, unit price, exchange rate, fees                                    | confirmed |
| `Sell`           | A regular sell transaction — quantity, unit price, exchange rate, fees, realized P&L                     | confirmed |
| `OpeningBalance` | A position seed entry — quantity and total cost paid directly, no fee breakdown                          | confirmed |
| `Deposit`        | A cash inflow from outside the application's tracked world (CSH-022)                                     | confirmed |
| `Withdrawal`     | A cash outflow to outside the application's tracked world (CSH-032)                                      | confirmed |
| `Dividend`       | Cash income paid by a held asset; credits cash, attributed to the paying asset (DIV-023)                 | confirmed |
| `FreeShares`     | Shares of a held asset received at no cost (bonus issue); quantity rises, cost basis unchanged (FSD-022) | confirmed |

## Cash Domain Concepts (introduced by CSH spec)

### Cash Asset

> Status: confirmed

A system-seeded `Asset` of `class = AssetClass::Cash`, one per ISO currency, with deterministic id `system-cash-{ccy}`. Acts as the asset reference for cash positions. Not user-editable, not user-creatable, not displayed in the asset catalog.

### Cash Holding

> Status: confirmed

A `Holding` whose asset is a Cash Asset. Represents the cash balance held in the account in the account's reference currency. Exactly one Cash Holding per account, created together with the account at a zero balance and kept for the account's lifetime — when the account holds no cash it stays at zero rather than disappearing.

### Global Value

> Status: confirmed

The full economic value of an account: cash balance + Σ (market value of non-cash active holdings). Surfaced as `AccountDetailsResponse.total_global_value` (CSH-094). Used as the canonical "what is this account worth right now?" metric across the Account Details header and (later) the portfolio dashboard.

## Dividend Domain Concepts (introduced by DIV spec)

### Dividends Received

> Status: confirmed

Cumulative cash dividend income attributed to a holding, in the account's reference currency. Per-holding (`HoldingDetail.dividends_received`, Σ of `Dividend` total_amount for the `(account, asset)` pair, DIV-070) and per-account (`AccountDetailsResponse.total_dividends_received`, Σ across all dividends, DIV-073). Reported separately from realized P&L — a dividend is income, not a capital gain.

### Total Return

> Status: confirmed

A holding's combined return from price appreciation and dividend income, expressed as a percentage of cost basis: `(unrealized_pnl + dividends_received) / cost_basis`. Surfaced as `HoldingDetail.total_return_pct` (DIV-071); `null` under the same conditions as `performance_pct` (no recorded price, or zero cost basis — MKT-034/035).

---

## Currency Context (introduced by FXR spec)

### CurrencyPair

A directed currency pair the system follows, e.g. USD → EUR. Used to value a holding whose currency differs from its account's. Created when first needed and kept thereafter.

> Status: confirmed

### CurrencyRate

What one unit of a currency is worth in another on a given day. Used to convert a foreign holding's current value into the account's currency. Distinct from a transaction's `exchange rate`, which is fixed at trade time for cost basis — a currency rate is current and changes over time.

> Status: confirmed

### CurrencyRateSource

Where a currency rate came from: `Manual` (entered by the user), or `Frankfurter` / `Ecb` (fetched from a provider). Informational only — it does not change which rate applies.

> Status: confirmed

---

## Connection Context (introduced by KEY spec)

---

## Domain Events

| Name                       | Raised by              | Intent                                                                                                                                                       | Status    |
| -------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- |
| `AccountUpdated`           | Account BC             | Any state change in the account or its holdings                                                                                                              | confirmed |
| `TransactionUpdated`       | Account BC             | A holding or transaction was created, updated, or cancelled                                                                                                  | confirmed |
| `AssetUpdated`             | Asset BC               | Any state change in an asset or category                                                                                                                     | confirmed |
| `CategoryUpdated`          | Asset BC               | Any state change in a category                                                                                                                               | confirmed |
| `AssetPriceUpdated`        | Asset BC               | An AssetPrice record was created, updated, or deleted                                                                                                        | confirmed |
| `AssetPriceFetchCompleted` | Asset price fetch task | A fetch task finished; carries `ok` / `skipped` counts plus the list of assets it could not price so the UI can summarize the outcome and offer manual entry | confirmed |
| `CurrencyRateUpdated`      | Currency BC            | A currency rate was recorded, updated, or deleted                                                                                                            | confirmed |

---

## Asset Web Lookup

### OpenFIGI Lookup

The outbound HTTP search that, given a name, ticker, or ISIN, queries the OpenFIGI API and
returns up to 10 candidate `AssetLookupResult` values. 12-character alphanumeric inputs route
to the ISIN mapping endpoint; all others route to the keyword search endpoint.

> Status: confirmed

### AssetLookupResult

A transient value object returned by the OpenFIGI lookup. Never persisted. Carries the name,
reference (ISIN or ticker), currency, and asset class of a candidate instrument — used solely
to pre-fill the Add Asset form.

> Status: confirmed

## Asset Web Lookup Command

| Name           | Intent                                                                                                             | Status    |
| -------------- | ------------------------------------------------------------------------------------------------------------------ | --------- |
| `lookup_asset` | Query OpenFIGI with a name, ticker, or ISIN and return up to 10 `AssetLookupResult` values. Errors: `NetworkError` | confirmed |

---

## Asset Price Fetch Tasks

### Fetch task

A backend job that retrieves current prices from an external provider and upserts `AssetPrice` records. Umbrella term for the three named instances below.

> Status: confirmed

### Quote

A single price reading returned by a price provider during a fetch: the market price together with its observation date. Transient — it becomes an `AssetPrice` before being stored.

> Status: confirmed

### Auto-fetch

A fetch task triggered automatically at application launch when the user has enabled the auto-fetch setting. Scope: all active holdings across all accounts.

> Status: confirmed

### Global refresh

A fetch task triggered manually by the user from the global dashboard. Scope: all active holdings across all accounts. Shares the same backend entry point as auto-fetch.

> Status: confirmed

### Account refresh

A fetch task triggered manually by the user from an account detail page. Scope: active holdings of the specified account.

> Status: confirmed

### External provider

A third-party HTTP service that returns current asset prices. Currently Yahoo Finance — keyless, no credential required (ADR-017). "provider" in prose means an External provider.

> Status: confirmed

---

## Asset Price Service Operations

| Name                 | Intent                                                                                                                                                                                | Status    |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `record_asset_price` | Create or overwrite the price for an asset on a given date (upsert by `(asset_id, date)`). Errors: `AssetNotFound`, `NotPositive`, `NonFinite`, `DateInFuture`, `Unknown`             | confirmed |
| `get_asset_prices`   | Return all recorded prices for an asset, ordered by date descending. Errors: `AssetNotFound`, `Unknown`                                                                               | confirmed |
| `update_asset_price` | Change the date and/or price of an existing price record; atomic delete-old + upsert-new when date changes. Errors: `NotFound`, `NotPositive`, `NonFinite`, `DateInFuture`, `Unknown` | confirmed |
| `delete_asset_price` | Remove a specific price record by `(asset_id, date)`. Errors: `NotFound`, `Unknown`                                                                                                   | confirmed |
