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

An internal entity of `Asset`. A price observation for an asset on a given date.

> Status: confirmed

---

## Aggregate Root Methods (Account)

| Name                  | Intent                                                                                                  | Status    |
| --------------------- | ------------------------------------------------------------------------------------------------------- | --------- |
| `buy_holding`         | Record a purchase of an asset into the account                                                          | confirmed |
| `sell_holding`        | Record a sale of an asset from the account                                                              | confirmed |
| `correct_transaction` | Correct the fields of an existing transaction (cascades VWAP/P&L recalculation on the affected holding) | confirmed |
| `cancel_transaction`  | Delete an existing transaction (cascades VWAP/P&L recalculation or holding removal)                     | confirmed |

---

## Domain Events

| Name                 | Raised by  | Intent                                                      | Status    |
| -------------------- | ---------- | ----------------------------------------------------------- | --------- |
| `AccountUpdated`     | Account BC | Any state change in the account or its holdings             | confirmed |
| `TransactionUpdated` | Account BC | A holding or transaction was created, updated, or cancelled | confirmed |
| `AssetUpdated`       | Asset BC   | Any state change in an asset or category                    | confirmed |
| `CategoryUpdated`    | Asset BC   | Any state change in a category                              | confirmed |
| `AssetPriceUpdated`  | Asset BC   | An AssetPrice record was created, updated, or deleted       | confirmed |

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

## Asset Price Service Operations

| Name                 | Intent                                                                                                                                                                                | Status    |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `record_asset_price` | Create or overwrite the price for an asset on a given date (upsert by `(asset_id, date)`). Errors: `AssetNotFound`, `NotPositive`, `NonFinite`, `DateInFuture`, `Unknown`             | confirmed |
| `get_asset_prices`   | Return all recorded prices for an asset, ordered by date descending. Errors: `AssetNotFound`, `Unknown`                                                                               | confirmed |
| `update_asset_price` | Change the date and/or price of an existing price record; atomic delete-old + upsert-new when date changes. Errors: `NotFound`, `NotPositive`, `NonFinite`, `DateInFuture`, `Unknown` | confirmed |
| `delete_asset_price` | Remove a specific price record by `(asset_id, date)`. Errors: `NotFound`, `Unknown`                                                                                                   | confirmed |
