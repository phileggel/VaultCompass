# Roadmap — Financial Asset Lifecycle

This document maps every operation needed to cover the full lifecycle of a financial asset in VaultCompass, from catalog entry to position closure. It is a planning reference, not a spec — each phase links to the relevant spec(s) and todo items when they exist.

---

## Lifecycle Overview

```
[1. Catalog]  →  [2. Acquisition]  →  [3. Monitoring]  →  [4. Corporate Events]
                                              ↓
                                      [5. Disposition]  →  [6. Closure]  →  [7. Archive]
```

---

## Phase 1 — Asset Catalog

Manage the list of tradable instruments the user can reference in transactions.

| Operation                 | Status     | Notes                                                                                                                                                                                                                                     |
| ------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Create asset              | ✅ Done    | Name, ticker, class, category, currency, risk level                                                                                                                                                                                       |
| Edit asset                | ✅ Done    |                                                                                                                                                                                                                                           |
| Archive asset             | ✅ Done    | Soft delete; preserves history                                                                                                                                                                                                            |
| Unarchive asset           | ✅ Done    |                                                                                                                                                                                                                                           |
| Delete asset              | ✅ Done    | Hard delete; only safe when no holdings exist                                                                                                                                                                                             |
| Archive eligibility guard | 🔲 Planned | Block archiving if `Holding.quantity > 0` (spec TRX OQ-6)                                                                                                                                                                                 |
| Delete eligibility guard  | 🔲 Planned | Block hard delete if any transaction exists for the asset                                                                                                                                                                                 |
| Inline asset creation     | ✅ Done    | From `/transactions/new`, `ComboboxField` `onCreateNew` navigates to `/assets?createNew=<query>&returnPath=...`; on success returns to `/transactions/new` with new asset pre-filled |

Spec: `docs/spec/asset.md`

---

## Phase 2 — Acquisition (Buy)

Record a purchase that opens or increases a position in an account.

| Operation                     | Status     | Notes                                                          |
| ----------------------------- | ---------- | -------------------------------------------------------------- |
| Add purchase transaction      | ✅ Done    | Quantity, unit price, exchange rate, fees, note                |
| Total amount auto-computation | ✅ Done    | Backend formula: `floor(qty×price/M)×rate/M + fees` (TRX-026)  |
| Multi-currency support        | ✅ Done    | Exchange rate stored per transaction (TRX-021)                 |
| VWAP cost basis update        | ✅ Done    | `avg_price = Σ total_amount / Σ quantity` (TRX-030)            |
| Atomic holding update         | ✅ Done    | Transaction + holding in one DB transaction (TRX-027)          |
| Archived asset auto-unarchive | ✅ Done    | With frontend confirmation dialog (TRX-028, TRX-029)           |
| Edit purchase transaction     | ✅ Done    | Full recalculation of holding on save (TRX-031)                |
| Delete transaction            | ✅ Done    | Full flow: backend + confirmation dialog + snackbar (TRX-035)  |
| Delete confirmation dialog    | ✅ Done    | ConfirmationDialog in TransactionListPage (TRX-035)            |
| Transaction list view         | ✅ Done    | Per-account/asset filter, sort, edit/delete actions (TXL spec) |
| Account currency field        | 🔲 Planned | Exchange rate visibility currently hardcoded vs EUR (todo)     |

Spec: `docs/spec/financial-asset-transaction.md`

---

## Phase 3 — Position Monitoring

Display the current state of positions and their performance.

| Operation                     | Status     | Notes                                                                |
| ----------------------------- | ---------- | -------------------------------------------------------------------- |
| Holdings computed from buys   | ✅ Done    | Quantity + VWAP average price per (account, asset) pair              |
| Account Details view          | ✅ Done    | Holdings list + total cost basis per account (ACD spec)              |
| Holdings reactivity           | ✅ Done    | Re-fetch on `TransactionUpdated` event (ACD-039)                     |
| Current market price          | 🔲 Planned | Requires a price data source (manual entry or feed)                  |
| Unrealized P&L                | 🔲 Planned | `(current_price − average_price) × quantity`; depends on phase above |
| Performance %                 | 🔲 Planned | `unrealized_pnl / cost_basis × 100`                                  |
| Portfolio summary / dashboard | 🔲 Planned | Aggregate view across all accounts                                   |

Spec: `docs/spec/account-details.md`

---

## Phase 4 — Corporate Events

Non-trade events that alter quantity or value without a cash exchange at the position level.

| Operation            | Status     | Notes                                                               |
| -------------------- | ---------- | ------------------------------------------------------------------- |
| Dividend             | 🔲 Planned | Cash income; does not change quantity; new transaction type needed  |
| Stock split          | 🔲 Planned | Multiplies quantity, divides price; requires dedicated operation    |
| Reverse split        | 🔲 Planned | Divides quantity, multiplies price                                  |
| Merger / acquisition | 🔲 Planned | Asset substitution; complex; out of scope until sell is implemented |

> None of these are specced yet. They require a new `transaction_type` enum and dedicated backend logic distinct from purchases and sales.

---

## Phase 5 — Disposition (Sell)

Record a sale that reduces or closes a position.

| Operation                    | Status     | Notes                                                                  |
| ---------------------------- | ---------- | ---------------------------------------------------------------------- |
| Sell transaction type        | 🔲 Planned | `transaction_type: Sell` defined in entity (TRX-026), not implemented  |
| Sell form                    | 🔲 Planned | Same fields as buy; quantity must not exceed current holding           |
| Holding quantity decrease    | 🔲 Planned | On sell, `Holding.quantity -= sold_quantity`                           |
| Sell validation              | 🔲 Planned | Cannot sell more than currently held (oversell guard)                  |
| Realized P&L computation     | 🔲 Planned | `(sell_price − average_price) × quantity`; crystallized at sell time   |
| Realized P&L display         | 🔲 Planned | Shown per sell transaction and cumulated per holding                   |
| Partial sell                 | 🔲 Planned | Holding remains open; VWAP unchanged for remaining quantity            |
| Edit/delete sell transaction | 🔲 Planned | Full recalculation required; same recalc engine as purchases (TRX-031) |

> Prerequisite: sell transaction type must be introduced before corporate events (phase 4) can be fully modelled.

---

## Phase 6 — Position Closure

A position is closed when `Holding.quantity` reaches zero.

| Operation                       | Status     | Notes                                                                     |
| ------------------------------- | ---------- | ------------------------------------------------------------------------- |
| Zero-quantity holding retention | ✅ Specced | Holding kept in DB at `quantity = 0`; `average_price` preserved (TRX-040) |
| Closed position exclusion       | ✅ Done    | `quantity > 0` filter in Account Details (ACD-020)                        |
| "All positions closed" state    | ✅ Done    | Distinct empty-state message in Account Details (ACD-034)                 |
| Closed position history         | 🔲 Planned | View past closed positions with realized P&L per position                 |
| Cumulative realized P&L         | 🔲 Planned | Total gains/losses across all closed positions in an account              |

---

## Phase 7 — Archive

An asset can be archived once it has no active positions, preserving its historical data.

| Operation                  | Status     | Notes                                                                    |
| -------------------------- | ---------- | ------------------------------------------------------------------------ |
| Archive action             | ✅ Done    | Available from asset table                                               |
| Auto-unarchive on new buy  | ✅ Done    | Transparent to user, with confirmation dialog (TRX-028, TRX-029)         |
| Archive eligibility guard  | 🔲 Planned | Block archiving if any `Holding.quantity > 0` across all accounts (OQ-6) |
| Archived asset in holdings | ✅ Done    | Included in Account Details as long as `quantity > 0` (ACD-021)          |

---

## Implementation Order (Recommended)

Based on value and dependency chain:

1. ~~**Delete confirmation UI**~~ — ✅ done
2. ~~**Transaction list view**~~ — ✅ done
3. ~~**Inline asset creation from Account Details**~~ — ✅ done
4. **Sell transaction** — unlocks phases 5 & 6; requires new spec
5. **Realized P&L** — follows sell; display per transaction and per position
6. **Archive eligibility guard** — depends on phase 5 (need sell to reach `qty = 0`)
7. **Current market price** — data source decision needed (manual vs. feed); standalone spec
8. **Unrealized P&L + performance %** — depends on market price
9. **Corporate events (dividends, splits)** — standalone spec; relatively independent
10. **Portfolio dashboard** — aggregation of phases 3–6; last to implement
