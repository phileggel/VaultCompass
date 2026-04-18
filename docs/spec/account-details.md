# Business Rules — Account Details (ACD)

## Context

The `Account Details` feature provides a view of a specific account's current positions and their cost basis. It allows users to drill down into an account to see their active holdings, the quantity held, the volume-weighted average purchase price, and the total cost basis for each position.

This feature consumes data from two bounded contexts: `account` (for Holding data) and `asset` (for asset metadata: name, ticker, currency). Because these contexts must not import each other directly (B2), all cross-context reads are orchestrated by a dedicated `use_cases/account_details/` use case that injects `AccountService` and `AssetService` (per ADR-003 and ADR-004).

> Cross-spec dependency: entry-point navigation behavior is owned by [Account Management](account.md) rule ACC-010.

> Market price tracking (current price, unrealized gain/loss, performance percentage) is not implemented in this version. It will be introduced as a dedicated feature once a market price data source is designed.

---

## Entity Definition

### Holding (Position)

Represents the current state of a financial position within the account.

| Field           | Business meaning                                                         |
| --------------- | ------------------------------------------------------------------------ |
| `asset_id`      | The financial asset held.                                                |
| `quantity`      | Current number of units held (i64 micros).                               |
| `average_price` | Volume-weighted average purchase price in account currency (i64 micros). |

### HoldingDetail (Backend DTO)

A computed view of a holding enriched with asset metadata and cost basis. Defined as a Rust struct with `#[derive(Type, Serialize)]` so it is auto-generated into `bindings.ts` via Specta. It is returned as part of the `AccountDetailsResponse` wrapper; the frontend presenter maps it to display-ready values.

| Field             | Business meaning                                                     |
| ----------------- | -------------------------------------------------------------------- |
| `asset_id`        | ID of the held asset.                                                |
| `asset_name`      | Display name of the asset (from asset context).                      |
| `asset_reference` | Ticker or user-defined reference of the asset (from asset context).  |
| `quantity`        | Current number of units held (i64 micros).                           |
| `average_price`   | VWAP purchase price in account currency (i64 micros).                |
| `cost_basis`      | Total cost of the position (`quantity × average_price`, i64 micros). |

### AccountDetailsResponse (Backend DTO)

The top-level response returned by the `get_account_details(account_id)` Tauri command. Defined as a Rust struct with `#[derive(Type, Serialize)]`.

| Field                 | Business meaning                                                                 |
| --------------------- | -------------------------------------------------------------------------------- |
| `account_name`        | Display name of the account (per ACD-032).                                       |
| `holdings`            | Filtered list of `HoldingDetail` sorted per ACD-033 (quantity > 0, per ACD-020). |
| `total_holding_count` | Count of all holdings for the account regardless of quantity (used by ACD-034).  |
| `total_cost_basis`    | Sum of `cost_basis` across all active holdings (per ACD-031).                    |

---

## Business Rules

### Navigation

**ACD-010 — View entry point (frontend)**: The Account Details view is accessed by clicking on an account row in the Account Table, excluding action buttons. This is the canonical navigation gesture defined in ACC-010; ACD-010 records the resulting destination.

**ACD-011 — Account selection persistence (frontend)**: The selected account is identified by its ID in the route `/accounts/:id`, enabling direct linking and browser "Back" navigation.

**ACD-012 — Invalid account guard (backend + frontend)**: If the `account_id` supplied to the backend does not correspond to an existing account, the backend returns an explicit not-found error. The frontend transitions to the error state (ACD-038).

### Holding List and Cost Basis

**ACD-020 — Active holding filter (backend)**: Only holdings with `quantity > 0` are included in the Account Details view. Holdings that reach zero quantity (e.g. from future Sell transactions, per TRX-040) are excluded from display.

**ACD-021 — Archived asset inclusion (backend)**: Holdings for archived assets are included in the display as long as their `quantity > 0`. Archiving an asset does not close its position.

**ACD-022 — Asset metadata enrichment (backend)**: For each holding, the use case fetches the corresponding asset's name and reference (ticker) from `AssetService`. These fields populate `HoldingDetail.asset_name` and `HoldingDetail.asset_reference`.

**ACD-023 — Cost basis calculation (backend)**: The cost basis for a holding is computed as `Holding.quantity × Holding.average_price`. This is the only financial computation performed by this feature.

**ACD-024 — Calculation precision (backend)**: Cost basis calculations use `i128` intermediates to prevent overflow before scaling back to `i64` micro-units, per ADR-001.

### Account Summary

**ACD-031 — Total account cost basis (backend)**: The total cost basis of the account is the sum of `cost_basis` across all active holdings (those included per ACD-020). When the account has no active holdings, the total cost basis is `0`.

**ACD-032 — Account name in response (backend)**: The use case fetches the account record via `AccountService.get_by_id` as its first step. The account name is included in `AccountDetailsResponse` so the frontend can display it in the header without a separate request.

**ACD-033 — Holdings sort order (backend)**: Holdings in the response are sorted alphabetically by `asset_name` ascending. This is the default display order; no user-controlled sort is required for the initial implementation.

### States

**ACD-034 — Empty account state (frontend)**: If no holdings remain after applying the `quantity > 0` filter (ACD-020), the view displays one of two messages depending on the backend response: "No positions yet" when the account has no holdings at all, or "All positions are closed" when holdings exist but all have `quantity = 0`. The Tauri command response includes `total_holding_count` so the frontend can distinguish these two cases without a second request.

**ACD-035 — Empty state CTA (frontend)**: In the empty state, the view displays an "Add Transaction" button that opens the Add Transaction modal pre-filled with the `account_id` from the current route (ACD-011), per the pre-fill contract defined in TRX-011.

**ACD-036 — Non-empty state CTA (frontend)**: When the account has at least one active holding, the view displays a FAB or "Add Transaction" button that opens the Add Transaction modal pre-filled with the `account_id` from the current route (ACD-011), per TRX-011. This is the entry point referenced by TRX-010.

**ACD-037 — Loading state (frontend)**: While the backend call is in progress, the view displays skeleton screens for the header (account name, total cost basis) and for the holdings table. The loading state is entered on initial mount and on each re-fetch triggered by ACD-039 or ACD-040.

**ACD-038 — Error state (frontend)**: If the backend call fails (network error, not-found per ACD-012, or unexpected error), the view displays a generic error message and a "Retry" button. Pressing "Retry" re-triggers the backend call.

### Data Integrity and Reactivity

**ACD-039 — Reactivity to transactions (frontend)**: When a `TransactionUpdated` event is received (published by TRX-037), the Account Details view re-fetches the full `AccountDetailsResponse` for the current account, as holdings data (quantity, average_price) is derived from transactions. ACD-039 is the authoritative re-fetch mechanism for the Account Details view; TRX-038's store action is scoped to the transaction list feature and does not affect account details data.

**ACD-040 — Reactivity to asset updates (frontend)**: When an `AssetUpdated` event is received, the Account Details view re-fetches the full `AccountDetailsResponse` to reflect any changes to asset metadata (name or reference). The event is published by the `asset` context; ACD-040 only governs the frontend reaction.

**ACD-041 — Precision handling (backend)**: All financial values in `HoldingDetail` and `AccountDetailsResponse` are serialised as `i64` micro-unit values per ADR-001. The frontend presenter is responsible for converting them to display-ready strings.

---

## Workflow

```
[User clicks account row (ACC-010)]
  → Route: /accounts/:id
          │
          ├─ [use_cases/account_details/: Fetch Account metadata from account context]
          ├─ [use_cases/account_details/: Fetch Holdings (quantity > 0) from account context]
          ├─ [use_cases/account_details/: Fetch asset metadata for each holding from asset context]
          ├─ [use_cases/account_details/: Compute cost_basis per holding (ACD-023)]
          ├─ [use_cases/account_details/: Compute total_cost_basis (ACD-031)]
          │
          └─ [Frontend: Loading state → Header (Account name + Total Cost Basis)]
             [Frontend: Holdings table (one row per HoldingDetail)]
             [Frontend: Empty / Error state as applicable]
```

---

## UX Draft

### Entry Point

- Clicking a row in the `AccountTable` (excluding action buttons), per ACC-010.

### Main Component

**ManagerLayout** containing:

- **Header**: Account name + Total Cost Basis (large).
- **Position Table**:
  - Asset (Name + Ticker)
  - Quantity
  - Avg. Price
  - Cost Basis

### States

- **Loading**: Skeleton screens for the header and table.
- **Empty (no positions)**: Illustration "No positions yet" + "Add Transaction" button.
- **Empty (all closed)**: Message "All positions are closed" + "Add Transaction" button.
- **Error**: Generic error message with "Retry" button.

### User Flow

1. User clicks on an account row in the Account Table.
2. Route navigates to `/accounts/:id`; loading skeletons appear.
3. Data loads: user sees each active position with its cost basis, and the total account cost basis.
4. User can add a transaction via the FAB or "Add Transaction" button.

---

## Open Questions

**~~ADR-REQUIRED~~ — Multi-context read orchestration** _(resolved)_: Orchestration strategy and dependency injection boundary are both decided.

- ADR-003: cross-context use cases use sequential service calls.
- ADR-004: use cases always inject services, never repositories.

The `use_cases/account_details/` use case injects `AccountService` and `AssetService` and calls them in sequence. `TransactionService` is not injected — holdings already carry the pre-computed VWAP `average_price`.

**~~OQ-ACD-002~~ — Double re-fetch on TransactionUpdated** _(resolved)_: ACD-039 is the authoritative re-fetch for the Account Details view. TRX-038's store action is scoped to the transaction list feature and does not overlap with account details data. No duplication occurs.
