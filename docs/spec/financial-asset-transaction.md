# Business Rules — Financial Asset Transaction (TRX)

## Context

A `Transaction` represents a financial event affecting an asset's quantity and cost basis within a specific account. This feature allows users to record purchases (and later sales) of financial assets (Stocks, ETF, Digital Assets, etc.). These transactions are the source of truth for calculating current holdings (quantity and average purchase price) and historical performance.

The `Transaction` entity owns its own bounded context (`context/transaction/`). As this feature spans multiple bounded contexts (`transaction/`, `account/`, and `asset/`), the orchestration logic resides in a dedicated Use Case within `src-tauri/src/use_cases/`, ensuring atomic updates across entities. The use case calls `TransactionService` (persists the transaction, publishes `TransactionUpdated`), then calls `AccountService` (updates the Holding); it queries `asset/` only for existence and archive-status checks.

### Holding entity

The `Holding` entity represents the current state of a financial position: an asset held within an account. It is owned by the `account/` bounded context and replaces the former `AssetAccount` entity (see [ADR-002](../adr/002-replace-asset-account-with-holding.md)). All financial fields are stored as `i64` micro-units per [ADR-001](../adr/001-use-i64-for-monetary-amounts.md).

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

### Holding

Represents the current state of a position (asset held within an account). Computed from transactions.

| Field           | Business meaning                                                         |
| --------------- | ------------------------------------------------------------------------ |
| `id`            | Unique identifier of the holding.                                        |
| `account_id`    | The account holding the asset.                                           |
| `asset_id`      | The financial asset held.                                                |
| `quantity`      | Current number of units held (i64 micros).                               |
| `average_price` | Volume-weighted average purchase price in account currency (i64 micros). |

---

## Business Rules

### Eligibility and Initiation

**TRX-010 — Purchase entry point (frontend)**: A purchase can be initiated from the "Assets" table (contextual action on an asset) or from the "Account Details" view (FAB or "Add Transaction" button).

**TRX-011 — Contextual pre-filling (frontend)**: When initiated from a specific asset, the asset is pre-selected. When initiated from an account view, the account is pre-selected. When both contexts are available (e.g., entry from a specific holding row), both are pre-selected.

### Creation

**TRX-020 — Field validation (backend)**: A transaction is valid if: `account_id` and `asset_id` exist, `date` is not in the future and not before `1900-01-01`, `quantity` is strictly positive, `unit_price` is positive or zero, `exchange_rate` is strictly positive, and the backend-computed `total_amount` is positive.

**TRX-021 — Multi-currency semantics (backend)**: The `unit_price` is stored in the asset's native currency. The `exchange_rate` is the conversion rate from the asset's currency to the account's currency, and is stored explicitly with the transaction.

**TRX-022 — Holding quantity update (backend)**: Creating a purchase transaction increases the `Holding.quantity` for the specified asset and account.

**TRX-023 — Form default values (frontend)**: The transaction date defaults to the current day. The `transaction_type` defaults to `Purchase` and is not displayed in the form while only `Purchase` is supported. `exchange_rate` defaults to 1.0.

**TRX-024 — Micro-unit representation (full stack)**: All financial amounts (quantity, price, fees, total) are represented as 64-bit integers (`i64`) using a micro-unit scale (×1,000,000) throughout the stack, as per [ADR-001](../adr/001-use-i64-for-monetary-amounts.md). The frontend stores and manipulates these values internally as micro-units. The only decimal↔micro conversion occurs at the UI boundary: user input (decimal string → `i64` micro) and display (`i64` micro → formatted decimal string, 3 decimal places).

**TRX-025 — Holding cost basis update (backend)**: Creating a purchase transaction updates the `Holding.average_price` using the VWAP method (TRX-030).

**TRX-026 — Total amount computation (backend)**: `total_amount` is computed by the backend as `floor(floor(quantity × unit_price / MICRO) × exchange_rate / MICRO) + fees`. All values are `i64` micro-units (TRX-024); arithmetic uses `i128` intermediates to prevent overflow. `total_amount` is never received from the frontend — the DTO (`CreateTransactionDTO`) intentionally omits it. The frontend computes the same formula locally for real-time display preview only (see TRX-024).

**TRX-027 — Atomicity of transaction and holding updates (backend)**: The transaction record insert and all associated `Holding` mutations (quantity and average_price) must be performed within a single database transaction. A failure in any step rolls back the entire operation.

**TRX-028 — Archived asset auto-unarchive (backend)**: If the referenced asset is archived at the time of transaction creation or modification, the use case atomically unarchives the asset and persists the transaction in a single database operation. The archived flag is reverted if the transaction fails.

**TRX-029 — Archived asset confirmation (frontend)**: If the selected asset is archived, a confirmation dialog is shown before form submission, informing the user that saving will automatically unarchive the asset. The transaction is submitted only upon explicit user confirmation.

### Update and Deletion

**TRX-030 — VWAP Calculation (backend)**: Average purchase price for a `Holding` is calculated using the Volume Weighted Average Price (VWAP) method: `Sum(quantity × unit_price × exchange_rate) / Total Quantity`. Only purchase transactions are included in this calculation. This rule applies on creation (TRX-025) and on recalculation triggered by modification or deletion.

**TRX-031 — Transaction modification (backend)**: Modifying a transaction triggers a full recalculation of the `Holding` cost basis and quantity for the `(account_id, asset_id)` pair, processing all associated transactions in chronological order (TRX-036).

**TRX-032 — Modifiable fields (backend)**: All fields of a transaction are modifiable. Changing the `asset_id` or `account_id` is permitted and triggers a recalculation of Holdings for both the old and new `(account_id, asset_id)` pairs.

**TRX-033 — Update field validation (backend)**: When modifying a transaction, the same field constraints as TRX-020 apply, and the archived asset guard (TRX-028) is enforced. If `account_id` or `asset_id` is changed, the existence and non-archived status of the new values is verified before proceeding.

**TRX-034 — Transaction deletion (backend)**: Deleting a transaction triggers a recalculation of the `Holding` for the `(account_id, asset_id)` pair. If no transactions remain for that asset in the account, the `Holding` record is removed.

**TRX-035 — Deletion confirmation (frontend)**: Deleting a transaction requires a user confirmation dialog to prevent accidental data loss.

**TRX-036 — Chronological integrity (backend)**: Recalculations of `Holding` state following a transaction mutation must process all associated transactions for the specific `(account_id, asset_id)` pair in chronological order to ensure the cost basis and quantity remain accurate.

**TRX-037 — TransactionUpdated event (backend)**: After any successful transaction mutation (create, update, or delete), the `TransactionService` (owned by the `transaction/` bounded context) publishes a `TransactionUpdated` event on the event bus. The use case delegates to the service; the service owns the event publication (B8 compliance). This event is distinct from `AccountUpdated`: `AccountUpdated` signals structural account changes; `TransactionUpdated` signals position-data changes.

**TRX-038 — Holdings refresh on event (frontend)**: Upon receiving a `TransactionUpdated` event, the frontend refreshes the holdings data for the affected account so that the displayed portfolio state reflects the mutation. The implementation mechanism (store slice, local re-fetch, etc.) is left to the feature-planner.

### Lifecycle Management

**TRX-040 — Zero quantity handling (backend)**: If a `Holding.quantity` reaches zero due to future `Sell` transactions, the `Holding` entity remains in the database to accommodate potential future purchase transactions. The `average_price` is maintained at its last known value until the next purchase transaction initiates a new VWAP calculation. _(Implementation deferred until `Sell` transaction type is introduced.)_

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
          ├─ [Enter Exchange Rate] (default: 1.0, hidden if same currency)
          ├─ [Enter Fees]
          ├─ [Enter Note] (optional)
          │
          └─ [Save] → Backend validates (TRX-020, TRX-033)
                     → Backend computes total_amount (TRX-026)
                     → Persists Transaction in micro-units (TRX-024)
                     → Updates Holding atomically (TRX-022, TRX-025, TRX-027)
                       using VWAP in chronological order (TRX-030, TRX-036)
                     → Publishes TransactionUpdated (TRX-037)
                     → Frontend refreshes holdings view (TRX-038)
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
- Exchange Rate (Number field, visible only if asset currency ≠ account currency)
- Fees (Amount field with account currency suffix)
- Total Amount (Amount field with account currency suffix, auto-calculated and read-only)
- Note (Textarea, optional)

_`transaction_type` is not shown in the form. It is hardcoded to `Purchase` until the `Sell` type is introduced._

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
5. Total Amount is auto-calculated and read-only.
6. User clicks "Save".

---

## Open Questions

**OQ-1 — ~~`unit_price = 0` validity~~** _(resolved)_: A zero unit price is valid (gifted shares, inherited assets). TRX-020 unchanged.

**OQ-2 — ~~Transaction `date` lower bound~~** _(resolved)_: Minimum date set to `1900-01-01`. Converted to TRX-020.

**OQ-3 — ~~Frontend reactivity after transaction mutation~~** _(resolved)_: `TransactionUpdated` event published by the use case; frontend refreshes holdings on receipt. Implementation mechanism left to feature-planner. Converted to TRX-037 and TRX-038.

**OQ-4 — ~~TRX-040 / Sell scope~~** _(closed)_: Zero-quantity handling and Sell transaction type are out of scope for the initial implementation. TRX-040 remains in spec as forward documentation only.

**OQ-5 — ~~Archived asset behaviour~~** _(resolved)_: Auto-unarchive with user confirmation. Converted to TRX-028 (backend atomicity) and TRX-029 (frontend confirmation dialog).

**OQ-6 — Asset archiving eligibility rule** _(deferred — asset spec)_: An asset should only be archivable if its `Holding.quantity` is zero across all accounts. Out of scope for this spec; requires a new rule in the asset spec (`docs/asset.md`) once the `Holding` entity is available.

**OQ-7 — ~~Edit/delete workflow diagrams~~** _(deferred)_: Diagrams for the edit and delete paths are out of scope for the initial implementation. To be added in a future spec update alongside the `Sell` transaction type.
