# Business Rules — Financial Asset Transaction (TRX)

## Context

A `Transaction` represents a financial event affecting an asset's quantity and cost basis within a specific account. This feature allows users to record purchases (and later sales) of financial assets (Stocks, ETF, Digital Assets, etc.). These transactions are the source of truth for calculating current holdings (quantity and average purchase price) and historical performance.

As this feature spans multiple bounded contexts (`Account` and `Asset`), the orchestration logic resides in a dedicated Use Case within `src-tauri/src/use_cases/`, ensuring atomic updates across entities.

---

## Entity Definition

### Transaction

Represents a single purchase (or sale) event for an asset in an account.

| Field              | Business meaning                                                      |
| ------------------ | --------------------------------------------------------------------- |
| `id`               | Unique identifier of the transaction.                                 |
| `account_id`       | The account where the transaction occurred.                           |
| `asset_id`         | The financial asset involved in the transaction.                      |
| `transaction_type` | Type of transaction: `Purchase` or `Sell` (future).                   |
| `date`             | Date when the transaction was executed.                               |
| `quantity`         | Number of units acquired (positive, stored in micros: value \* 10^6). |
| `unit_price`       | Price per unit in asset's currency (stored in micros: value \* 10^6). |
| `exchange_rate`    | Exchange rate between asset currency and account currency (micros).   |
| `fees`             | Transaction fees in the account's currency (stored in micros).        |
| `total_amount`     | Total cost in account's currency (incl. fees, stored in micros).      |
| `note`             | Optional user comment.                                                |

---

## Business Rules

### Eligibility and Initiation (010–019)

**TRX-010 — Purchase entry point (frontend)**: A purchase can be initiated from the "Assets" table (contextual action on an asset) or from the "Account Details" view (FAB or "Add Transaction" button).

**TRX-011 — Contextual pre-filling (frontend)**: When initiated from a specific asset, the asset is pre-selected. When initiated from an account view, the account is pre-selected.

### Creation (020–029)

**TRX-020 — Field validation (backend)**: A transaction is valid if: `account_id` and `asset_id` exist, `date` is not in the future, `quantity` is strictly positive, `unit_price` is positive or zero, and `total_amount` is positive.

**TRX-021 — Multi-currency handling (backend)**: The `unit_price` is in the asset's currency. The `exchange_rate` (explicitly stored) is used to convert the cost to the account's currency. `total_amount` must equal `(quantity * unit_price * exchange_rate) + fees` (all in base units before micro-conversion).

**TRX-022 — Holding quantity update (backend)**: Creating a purchase transaction increases the `Holding.quantity` for the specified asset and account.

**TRX-023 — Form default values (frontend)**: The transaction date defaults to the current day. The `transaction_type` defaults to `Purchase`. `exchange_rate` defaults to 1.0.

**TRX-024 — Decimal storage (backend)**: All financial amounts (quantity, price, fees, total) are stored as 64-bit integers (`i64`) using a micro-unit scale (multiplier of 1,000,000) to ensure precision as per [ADR-001](../adr/001-use-i64-for-monetary-amounts.md).

**TRX-025 — Holding cost basis update (backend)**: Creating a purchase transaction updates the `Holding.average_price` using the VWAP method (TRX-030).

### Update and Deletion (030–050)

**TRX-030 — VWAP Calculation (backend)**: Average purchase price for a `Holding` is calculated using the Volume Weighted Average Price (VWAP) method: `Sum(quantity * unit_price * exchange_rate) / Total Quantity`. Only purchase transactions are included in this calculation.

**TRX-031 — Transaction modification (backend)**: Modifying a transaction triggers a full recalculation of the associated `Holding` cost basis and quantity.

**TRX-032 — Modifiable fields (backend)**: All fields of a transaction are modifiable. Changing the `asset_id` or `account_id` is permitted and triggers a recalculation of Holdings for both the old and new associations.

**TRX-040 — Transaction deletion (backend)**: Deleting a transaction triggers a recalculation of the associated `Holding`. If no transactions remain for that asset in the account, the `Holding` is removed.

**TRX-041 — Deletion confirmation (frontend)**: Deleting a transaction requires a user confirmation dialog to prevent accidental data loss.

**TRX-050 — Chronological integrity (backend)**: Recalculations of `Holding` state following a transaction mutation must process all associated transactions for that specific asset and account in chronological order to ensure the cost basis and quantity remain accurate.

### Lifecycle Management (051–059)

**TRX-051 — Zero quantity handling (backend)**: If a `Holding.quantity` reaches zero due to future `Sell` transactions, the `Holding` entity remains in the database to accommodate potential future purchase transactions. The `average_price` is maintained at its last known value until the next purchase transaction initiates a new VWAP calculation.

---

## Workflow

```
[User initiates Purchase]
  → Modal: Add Purchase Transaction
          │
          ├─ [Select Account] (pre-filled if possible)
          ├─ [Select Asset] (pre-filled if possible)
          ├─ [Enter Date] (default: today)
          ├─ [Enter Quantity & Unit Price]
          ├─ [Enter Exchange Rate] (default: 1.0)
          ├─ [Enter Fees & Total Amount]
          ├─ [Enter Note] (optional)
          │
          └─ [Save] → Backend validates (TRX-020, TRX-021)
                     → Persists Transaction with micro-units (TRX-024)
                     → Updates Holding using VWAP chronologically (TRX-022, TRX-025, TRX-030, TRX-050)
                     → Refreshes views
```

---

## UX Draft

### Entry Point

- "Buy" action in the Assets table row.
- "Add Transaction" FAB in the Account Details view.

### Main Component

**FormModal** with the following fields:

- Account (Select)
- Asset (Combobox with fuzzy search)
- Date (Date picker)
- Quantity (Number field)
- Unit Price (Amount field with asset currency suffix)
- Exchange Rate (Number field, visible if asset currency != account currency)
- Fees (Amount field with account currency suffix)
- Total Amount (Amount field with account currency suffix, auto-calculated from inputs)
- Note (Textarea, optional)

### States

- **Empty**: Form fields empty or defaulted.
- **Loading**: Submitting the transaction.
- **Error**: Inline validation errors or backend rejection message.
- **Success**: Modal closes, success notification.

### User Flow

1. User clicks "Buy" on an asset.
2. Form opens with Asset pre-selected.
3. User selects the target Account.
4. User enters Quantity, Unit Price, and Fees.
5. Total Amount is auto-calculated but remains editable for minor adjustments.
6. User clicks "Save".

---

## Open Questions

None — all questions have been resolved.
