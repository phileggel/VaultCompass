# Ubiquitous Language

The authoritative dictionary of domain terms for this project.

**Rules:**
- All terms MUST be agreed with the user before use in code, tests, or docs.
- The agent MUST NOT invent or assume domain terms — propose and wait for confirmation.
- Terms marked `pending` are proposals awaiting user validation.
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

| Name | Intent | Status |
|---|---|---|
| `buy_holding` | Record a purchase of an asset into the account | confirmed |
| `sell_holding` | Record a sale of an asset from the account | confirmed |
| `correct_transaction` | Correct the fields of an existing transaction (cascades VWAP/P&L recalculation on the affected holding) | confirmed |
| `cancel_transaction` | Delete an existing transaction (cascades VWAP/P&L recalculation or holding removal) | confirmed |

---

## Domain Events

| Name | Raised by | Intent | Status |
|---|---|---|---|
| `AccountUpdated` | Account BC | Any state change in the account or its holdings | confirmed |
| `AssetUpdated` | Asset BC | Any state change in the asset context (asset, price, category) | confirmed |
