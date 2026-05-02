# Implementation Plan — Opening Balance (TRX-042…TRX-058)

> Spec: `docs/spec/financial-asset-transaction.md` (Opening Balance section, TRX-042 to TRX-058)
> Contract: `docs/contracts/record_transaction-contract.md` (`open_holding` command, `OpenHoldingDTO`)
> ADRs: ADR-001 (i64 micro-units), ADR-002 (Holding entity)
> Trigram: TRX (registered, status `active` in `docs/spec-index.md`)

---

## Locked Design Decisions (from feature owner)

- New `TransactionType::OpeningBalance` variant alongside `Purchase` and `Sell`.
- New `AccountService::open_holding(account_id, asset_id, date, quantity, total_cost)` aggregate orchestrator method — same BC as buy/sell, no new use case (no cross-context atomicity needed beyond what `Account.save()` already provides).
- `total_amount = total_cost` (direct), `unit_price = floor(total_cost * MICRO / quantity)` (micro-aware), `fees = 0`, `exchange_rate = 1_000_000`. TRX-026 formula does not apply.
- `OpeningBalance` participates in VWAP recalculation identically to `Purchase` (TRX-048) — extend `recalculate_holding` match arm.
- Backend auto-unarchives archived assets atomically when persisting (TRX-050, mirrors TRX-028) — no `ArchivedAsset` error variant on `open_holding`.
- Entry point: dedicated button in Account Details page header (TRX-055), account pre-filled — no holding-row action.
- Asset selector lists active assets only (TRX-043) — no exchange rate or fees fields.
- Edit reuses existing `correct_transaction`; delete reuses existing `cancel_transaction`. The edit form must hide fees & exchange rate and rebuild `total_amount` from total_cost on submit (TRX-051).
- Transaction list label: "Opening Balance" (TRX-052); unit_price column shows the stored value (TRX-053); realized P&L cell is empty for opening-balance rows (TRX-054).
- Error variants exposed: `QuantityNotPositive`, `InvalidTotalCost` (new), `DateInFuture`, `DateTooOld`, `AccountNotFound`, `AssetNotFound`, `DbError` (TRX-056). No `ArchivedAsset`.

> **Cross-BC archive concern — resolved (OQ-A, Option 2)**: `open_holding` stays in `AccountService` with no use case. If the target asset is archived (race condition — asset selector only shows active assets per TRX-043), the command returns `ArchivedAsset` error; no auto-unarchive. TRX-050 and TRX-056 updated accordingly. Auto-unarchive on buy remains a buy-specific behavior (TRX-028).

---

## Workflow TaskList (mandatory quality gates)

- [x] Review architecture & rules (`ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/frontend-rules.md`, `docs/e2e-rules.md`, ADR-001, ADR-002)
- [x] No DB migration required (the existing `transactions.transaction_type TEXT` column accepts the new `"OpeningBalance"` string; no schema change)
- [x] Backend test stubs (`test-writer-backend` — all stubs written, red confirmed)
- [x] Backend implementation (minimal — make failing tests pass; no methods, defensive code, or features beyond what the stubs demand)
- [x] `just format` (rustfmt + clippy --fix)
- [x] Backend review (`reviewer-backend` → fix issues)
- [x] Type synchronization (`just generate-types` — emits `OpenHoldingDTO`, `OpeningBalance` enum variant, `OpenHoldingCommandError` into `src/bindings.ts`)
- [x] Compilation fixup (resolve TypeScript errors in the existing transactions feature caused by the new `TransactionType` variant — exhaustive match arms, presenter type label, edit-modal type checks; **no new UI work**)
- [x] `just check` — TypeScript clean
- [x] Commit — backend layer (`e977466 feat(account): implement opening-balance transaction type`)
- [x] Frontend test stubs (`test-writer-frontend` — all stubs written, red confirmed)
- [x] Frontend implementation (minimal — make failing tests pass)
- [x] `just format`
- [x] Frontend review (`reviewer-frontend` → fix issues)
- [x] Commit — frontend layer (`e40b1b2 feat(opening-balance): implement frontend for TRX-042–058`)
- [x] E2E tests — test file written (`e2e/open_balance/open_balance.test.ts`); not run against live app (skipped by user)
- [x] Commit — E2E + compliance (`3e75006`, `7e4b82f`)
- [x] Cross-cutting review (`reviewer-arch` — 4 findings resolved)
- [x] i18n review (`i18n-checker` — 2 placeholder keys added; all other keys clean)
- [x] Documentation update (`ARCHITECTURE.md` updated; `docs/todo.md` 2 tech-debt items added; `docs/ubiquitous-language.md` `open_holding`/`OpeningBalance` confirmed)
- [x] Spec check (`spec-checker` — 15/17 rules fully covered; TRX-055/046/058 gaps fixed in review pass)
- [ ] Commit — tests & docs (pending)

---

## Detailed Implementation Plan

### Backend

> All micro-unit fields use `i64` per ADR-001. Cross-aggregate writes (auto-unarchive + persist) follow the use-case + service delegation pattern (B5, B22).

#### B1. Domain — extend `TransactionType` (TRX-042)

- File: `src-tauri/src/context/account/domain/transaction.rs`
  - Add variant `OpeningBalance` to the `TransactionType` enum (keep `#[derive]` chain unchanged — `strum_macros::Display` and `EnumString` handle DB serialization).
  - Update the doc comment on the enum and on `Transaction.transaction_type` field to mention the new variant (TRX-042).
  - **No schema change**: `transactions.transaction_type` is `TEXT` and stores the strum-rendered name.
  - **No new error variants on `TransactionDomainError`**: `InvalidTotalCost` is a command-boundary concern only (the domain validator already enforces `total_amount > 0` via `TotalAmountNotPositive`); a new variant would be redundant and would not capture the pre-computation case where `total_cost` itself is the user input.

#### B2. Domain — extend `Account.recalculate_holding` (TRX-048)

- File: `src-tauri/src/context/account/domain/account.rs`
  - In `recalculate_holding`, change the inner `match t.transaction_type` so `TransactionType::OpeningBalance` follows the **same branch** as `TransactionType::Purchase`: contributes `t.total_amount` to `vwap_numerator` and `t.quantity` to `total_quantity` (TRX-048).
  - In `correct_transaction`, the `match tx_type { ... }` decides which formula recomputes `total_amount`. Add a third arm `TransactionType::OpeningBalance => Self::compute_opening_balance_total(quantity, unit_price)` — see B3 — so editing an opening balance preserves TRX-047 semantics on edit (TRX-051).

#### B3. Domain — add `Account::open_holding` aggregate root method (TRX-042, TRX-047, TRX-048)

- File: `src-tauri/src/context/account/domain/account.rs`
  - New method `open_holding(asset_id: String, date: String, quantity: i64, total_cost: i64) -> Result<&Transaction>` on `Account`.
  - Body:
    1. Compute `unit_price = floor(total_cost as i128 * MICRO / quantity as i128) as i64` — guard `quantity > 0` first to avoid divide-by-zero (returns `TransactionDomainError::QuantityNotPositive`).
    2. Compute `total_amount = total_cost` (direct).
    3. Build `Transaction::new(self.id.clone(), asset_id.clone(), TransactionType::OpeningBalance, date, quantity, unit_price, /*exchange_rate*/ 1_000_000, /*fees*/ 0, total_amount, /*note*/ None, /*realized_pnl*/ None)?`. The existing validator covers TRX-046 (date) via `InvalidDate`/`DateInFuture`/`DateTooOld`, and TRX-044 (qty > 0) via `QuantityNotPositive`. `total_amount > 0` is enforced; `unit_price >= 0` is satisfied by construction (positive total_cost / positive quantity).
    4. Push the transaction, recompute the holding for the `(account, asset)` pair (`recalculate_holding`), enqueue `TransactionInserted` + `HoldingUpserted`, mirror the in-memory holding.
  - Add a private helper `compute_opening_balance_total(quantity: i64, unit_price: i64) -> i64` that returns `floor(quantity as i128 * unit_price as i128 / MICRO) as i64` — used by `correct_transaction` to recompute `total_amount` on edit per TRX-051.
  - **Note**: TRX-047 expresses `total_amount` as the user-entered total cost both on creation and on edit. On edit, the `CorrectTransactionDTO` carries `quantity` and `unit_price` (decimal-style fields — no `total_cost`). To keep TRX-047 strict, the frontend must pre-compute `unit_price = floor(total_cost * MICRO / quantity)` on edit too (matches TRX-047 formula); the backend reconstructs `total_amount` from `quantity * unit_price`. Document this expectation in the method's rustdoc; it removes the need for a new DTO.

#### B4. Domain — adjust `Transaction::validate` for `OpeningBalance` (TRX-046, no new errors)

- File: `src-tauri/src/context/account/domain/transaction.rs`
  - The existing validator already enforces TRX-044/TRX-045/TRX-046 indirectly via `QuantityNotPositive`, `TotalAmountNotPositive`, and the date checks. No change needed — confirm by inline tests (B6).

#### B5. Service — `AccountService::open_holding` method (TRX-042, TRX-050, TRX-056)

> OQ-A resolved: Option 2 — stays in `AccountService`, no use case. Returns `ArchivedAsset` error if the asset is archived (race condition only; asset selector shows active assets per TRX-043). No cross-BC call needed.

- File: `src-tauri/src/context/account/service.rs`
  - Add `pub async fn open_holding(&self, account_id: &str, asset_id: String, date: String, quantity: i64, total_cost: i64) -> anyhow::Result<Transaction>`.
  - Body:
    1. Verify asset exists via `self.asset_repository.get_by_id(&asset_id)` — return `OpenHoldingError::AssetNotFound` if missing.
    2. If `asset.is_archived`, return `OpenHoldingError::ArchivedAsset` (TRX-050).
    3. Load account aggregate via `AccountRepository::get_with_holdings_and_transactions`.
    4. Call `account.open_holding(asset_id, date, quantity, total_cost)?` (B3).
    5. Save via `AccountRepository::save`. Publish `TransactionUpdated` event (TRX-037).

- File: `src-tauri/src/context/account/api.rs`
  - New Tauri command: `pub async fn open_holding(service: State<'_, AccountService>, dto: OpenHoldingDTO) -> Result<Transaction, OpenHoldingCommandError>`.
  - Validate `total_cost > 0` before calling service — return `OpenHoldingCommandError::InvalidTotalCost` (TRX-045/TRX-056).
  - `OpenHoldingCommandError` enum: `AccountNotFound`, `AssetNotFound`, `ArchivedAsset`, `QuantityNotPositive`, `InvalidTotalCost`, `DateInFuture`, `DateTooOld`, `Unknown`.
  - `OpenHoldingDTO`: `account_id: String, asset_id: String, date: String, quantity: i64, total_cost: i64`.

- Update `src-tauri/src/core/specta_builder.rs`:
  - Add `.typ::<OpenHoldingDTO>()`, `.typ::<OpenHoldingCommandError>()`.
  - Append `account::open_holding` to `collect_commands![...]`.

#### B6. Inline tests (`#[cfg(test)] mod tests`) — covering TRX-042…TRX-051, TRX-056

- `transaction.rs` — extend tests with:
  - `opening_balance_round_trip_through_strum` (serialize + parse `"OpeningBalance"` round-trip).
- `account.rs` — extend tests with:
  - `open_holding_persists_transaction_with_computed_unit_price` — TRX-047 (qty=2, total_cost=300 → unit_price=150).
  - `open_holding_records_total_amount_equal_to_total_cost` — TRX-047.
  - `open_holding_rejects_non_positive_quantity` — TRX-044.
  - `open_holding_rejects_future_date` — TRX-046.
  - `open_holding_participates_in_vwap_with_purchase` — TRX-048 (open balance qty=2 @ total_cost=200 + purchase qty=2 @ unit_price=300 → VWAP = (200 + 600) / 4 = 200).
  - `correct_transaction_recomputes_total_amount_for_opening_balance` — TRX-051 (edit qty/unit_price; total stays the entered total_cost expressed as qty × unit_price).
- `use_cases/open_holding/orchestrator.rs` — integration tests with real SQLite repos (B27):
  - `open_holding_returns_account_not_found_when_account_missing` — TRX-056.
  - `open_holding_returns_asset_not_found_when_asset_missing` — TRX-056.
  - `open_holding_unarchives_archived_asset_atomically_then_persists` — TRX-050.
  - `open_holding_persists_for_active_asset_without_unarchive_call` — TRX-050 (sanity).

#### B7. Logging (B16/B17/B18)

- `OpenHoldingUseCase::open_holding` and `Account::open_holding` log `info!(target: BACKEND, account_id, asset_id, "open_holding")` mirroring the existing buy/sell trace style.

#### B8. Rule coverage table (Backend)

| Rule              | Layer              | File / Function                                                                                                                                                               |
| ----------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TRX-042           | domain             | `TransactionType::OpeningBalance` variant                                                                                                                                     |
| TRX-044           | domain (validator) | `Transaction::validate` → `QuantityNotPositive`                                                                                                                               |
| TRX-045           | use case           | `OpenHoldingUseCase` returns `OpenHoldingError::InvalidTotalCost` when `total_cost <= 0`                                                                                      |
| TRX-046           | domain (validator) | `Transaction::validate` → `DateInFuture` / `DateTooOld`                                                                                                                       |
| TRX-047           | domain             | `Account::open_holding` constructs Transaction with `total_amount = total_cost`, `unit_price = floor(total_cost * MICRO / quantity)`, `fees = 0`, `exchange_rate = 1_000_000` |
| TRX-048           | domain             | `Account::recalculate_holding` treats `OpeningBalance` like `Purchase`                                                                                                        |
| TRX-049           | domain             | inherent — no per-pair uniqueness constraint anywhere                                                                                                                         |
| TRX-050           | use case           | `OpenHoldingUseCase` calls `AssetService::unarchive_asset` before persist                                                                                                     |
| TRX-051 (backend) | domain             | `Account::correct_transaction` adds `OpeningBalance` arm calling `compute_opening_balance_total`; total_amount semantics preserved                                            |
| TRX-056           | api                | `OpenHoldingCommandError` enum maps every locked variant                                                                                                                      |

---

### Frontend

> All Tauri invocations live in feature gateways. The new gateway method belongs in `account_details/gateway.ts` (entry point lives in Account Details) — but the existing `transactions/gateway.ts` already centralizes transaction commands. Decision: place `openHolding` in the **transactions** gateway because the contract groups it under "Holding Operations" alongside `buyHolding`/`sellHolding`/`correctTransaction` (consistency with the contract). The Account Details modal imports it just like it imports `buyHolding` today via `useTransactions()`.

#### F1. Generated bindings — verify after `just generate-types`

- File: `src/bindings.ts` (auto-generated; **DO NOT EDIT**)
  - Verify presence of:
    - `commands.openHolding(dto: OpenHoldingDTO)` returning `Result<Transaction, OpenHoldingCommandError>`.
    - `OpenHoldingDTO` type with `account_id`, `asset_id`, `date`, `quantity`, `total_cost`.
    - `TransactionType` union extended with `"OpeningBalance"`.
    - `OpenHoldingCommandError` discriminated union with the seven locked variants + `Unknown`.

#### F2. Gateway extension

- File: `src/features/transactions/gateway.ts`
  - Add `async openHolding(dto: OpenHoldingDTO): Promise<Result<Transaction, OpenHoldingCommandError>>` calling `commands.openHolding(dto)`.
  - Update the import block to include `OpenHoldingDTO` and `OpenHoldingCommandError` from `@/bindings`.

#### F3. `useTransactions` hook extension

- File: `src/features/transactions/useTransactions.ts`
  - Add `openHolding` callback mirroring `buyHolding` style: returns `{ data, error }` shape; on error, returns `error: \`error.${res.error.code}\`` (i18n key prefix consistent with existing patterns).

#### F4. New sub-feature `open_balance` in `account_details`

> Folder layout per F1/F2 (gold layout). Lives under `account_details` because the entry point and its contextual data (`accountId`, `accountName`) belong to the Account Details use case (consistent with `buy_transaction/` and `sell_transaction/`).

- New folder: `src/features/account_details/open_balance/`
  - `OpenBalanceModal.tsx` — `FormModal` with the four fields specified in TRX-043:
    - Account display: read-only `TextField` showing pre-filled account name (TRX-055).
    - Asset selector: `ComboboxField` populated from `useAppStore.assets.filter(a => !a.is_archived)` (TRX-043 — active only).
    - Date: `DateField`, default today, max=today (TRX-046).
    - Quantity: numeric `TextField`, min=0, step=`0.000001` (TRX-044).
    - Total cost: numeric `TextField`, min=0, step=`0.000001`, currency suffix = account currency (TRX-045).
    - No fees field, no exchange rate field, no note field, no auto-record-price checkbox (TRX-043 strict).
    - Footer: Cancel + Submit. Submit button disabled while `!isFormValid` or `isSubmitting`; loading indicator on submit (TRX-057).
    - E2E selectors per `docs/e2e-rules.md`: `id="open-balance-form"`, fields `id="open-balance-{field}"`, submit `type="submit" form="open-balance-form"`.
  - `useOpenBalance.ts` — colocated hook. State: `formData` (asset_id, date, quantity, total_cost — decimal strings); `error`; `isSubmitting`. Computes `qtyMicro` and `totalCostMicro` via `decimalToMicro`. `isFormValid` = qty > 0 && totalCost > 0 && date valid && asset_id non-empty (TRX-044/045/046). On submit: call `useTransactions().openHolding({ account_id, asset_id, date, quantity: qtyMicro, total_cost: totalCostMicro })`. On success: snackbar "transaction.success_opening_balance" + `onSubmitSuccess()` (TRX-058). On error: render i18n key.
  - `useOpenBalance.test.ts` — vitest tests:
    - submit calls gateway with correctly converted micro values.
    - submit disabled when qty is 0/empty (TRX-044).
    - submit disabled when total_cost is 0/empty (TRX-045).
    - submit disabled when date is empty / future (TRX-046).
    - on backend error returns localized error key.
    - on success calls `onSubmitSuccess`.
  - `OpenBalanceModal.test.tsx` — minimal smoke (mounts; submit button has correct selectors per E1–E3).

#### F5. Account Details header — add entry point button (TRX-055)

- File: `src/features/account_details/account_details_view/AccountDetailsView.tsx`
  - In the summary header (around line 117–127, near the existing tonal "Add Transaction" button), add a sibling secondary `Button` labelled `t("account_details.open_balance")` with icon `<PlusCircle />` (or distinct icon) and `aria-label` set to the same i18n key for E2E selection (E4).
  - State: `const [isOpenBalanceOpen, setIsOpenBalanceOpen] = useState(false);` — open the modal on click; close on success or cancel.
  - Condition: render only when `summary && !summary.isEmpty` is **false too**, i.e. always-visible regardless of empty state (per TRX-055 — the button is in the page header, not a row action). Confirm with feature owner if it should be hidden in the all-empty state; the spec text says "alongside the existing Add Transaction button", so mirror that button's visibility rule.
  - Wire `<OpenBalanceModal isOpen={isOpenBalanceOpen} onClose={...} accountId={accountId} accountName={summary?.accountName ?? ""} onSubmitSuccess={() => { setIsOpenBalanceOpen(false); retry(); }} />` next to the existing modals (TRX-058 — `retry` re-fetches; the global `TransactionUpdated` listener also refreshes via `useAccountDetails` effect).

#### F6. Transaction list — label + unit price + realized P&L (TRX-052, TRX-053, TRX-054)

- File: `src/features/transactions/shared/presenter.ts`
  - The existing `toTransactionRow.type` field is currently `tx.transaction_type` (raw enum value). Replace with an i18n-mapped label so the row renders "Buy", "Sell", or "Opening Balance":
    ```ts
    type: t(`transaction.type_${tx.transaction_type.toLowerCase()}`);
    ```
    Pass the `t` function in (refactor pure mapper to accept it) **only if** the existing rendering already does mapping; otherwise return the raw `transaction_type` and let the row component i18n it. Inspect current usage and minimize the diff. Either way, ensure the surfaced label for `OpeningBalance` is `t("transaction.type_openingbalance")` → "Opening Balance" / "Solde initial".
  - `realizedPnl` currently maps from `tx.realized_pnl`. No change needed — `OpeningBalance` rows have `realized_pnl: null` so the cell is already empty (TRX-054). Confirm by test.
  - `unitPrice` already shows `microToFormatted(tx.unit_price)`. No change — the stored `unit_price` matches TRX-053.

- File: `src/features/transactions/transaction_list/TransactionListPage.tsx`
  - Verify the "Type" column renders the localized label correctly. No structural change.
  - Edit and Delete row actions already wire `correct_transaction` and `cancel_transaction` — these work for `OpeningBalance` rows out of the box because the backend handles the type generically. Confirm by E2E.

- File: `src/i18n/locales/{en,fr}/common.json`
  - Add keys: `transaction.type_purchase`, `transaction.type_sell`, `transaction.type_openingbalance`. Use existing translations for the first two if any; new EN key: "Opening Balance"; new FR key: "Solde initial".

#### F7. Edit modal — accommodate `OpeningBalance` (TRX-051)

- File: `src/features/transactions/edit_transaction_modal/EditTransactionModal.tsx` and `useEditTransactionModal.ts`
  - When `transaction.transaction_type === "OpeningBalance"`:
    - Hide fees field, exchange rate field, and `RecordPriceCheckbox`.
    - Show only date, quantity, and total cost.
    - The form's "total cost" input maps to `total_cost` decimal — the hook must compute `unit_price = floor(total_cost * MICRO / quantity)` before submit, and pass `fees = 0`, `exchange_rate = 1_000_000` to `correct_transaction` so the backend's `compute_opening_balance_total` (B3) reproduces `total_amount = total_cost`.
    - Validation reuses TRX-044/045/046: qty > 0, total_cost > 0, date in range. Add a small validator helper `validateOpenBalanceForm(formData)` in `shared/validateTransaction.ts` (TRX-044/045/046 mirror of `validateTransactionForm`).
  - For `Purchase`/`Sell`, behavior is unchanged.

- File: `src/features/transactions/shared/validateTransaction.ts`
  - Add `validateOpenBalanceForm(data, qtyMicro, totalCostMicro)` returning the first i18n error key or `null` (mirrors existing `validateTransactionForm`).

- File: `src/features/transactions/shared/types.ts`
  - Optional: add a discriminated `OpenBalanceFormData = { date: string; quantity: string; totalCost: string }` if it makes the hooks cleaner. Otherwise reuse `TransactionFormData` and document `unitPrice` repurposed.

#### F8. Compilation fixup (post `just generate-types`)

- Anywhere in the frontend that does `switch (tx.transaction_type) { case "Purchase": ...; case "Sell": ... }` without a default branch will error. Search:
  - `src/features/transactions/edit_transaction_modal/useEditTransactionModal.ts` (computeTotalMicro vs computeSellTotalMicro selection).
  - Any presenter/discriminator. Add the `OpeningBalance` case explicitly mapped (no default — TS exhaustiveness).
  - Run `just check` and resolve every reported error before continuing.

#### F9. Logging (F13/F14)

- `OpenBalanceModal` logs `info` on mount.
- `useOpenBalance.handleSubmit` logs `error` only on unexpected gateway exception (not on validation).

#### F10. Rule coverage table (Frontend)

| Rule    | Layer            | File / Function                                                                                                                                                           |
| ------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TRX-043 | component        | `OpenBalanceModal` renders only the four locked fields; asset combobox sources `useAppStore.assets.filter(!is_archived)`                                                  |
| TRX-044 | hook             | `useOpenBalance.isFormValid` rejects qty ≤ 0                                                                                                                              |
| TRX-045 | hook             | `useOpenBalance.isFormValid` rejects total_cost ≤ 0                                                                                                                       |
| TRX-046 | component        | `DateField max={today}`; `useOpenBalance.isFormValid` rejects empty/invalid/future date                                                                                   |
| TRX-051 | component        | `EditTransactionModal` hides fees/exchange rate when `transaction_type === "OpeningBalance"`; `useEditTransactionModal` recomputes `unit_price` from total_cost on submit |
| TRX-052 | presenter / i18n | `transaction.type_openingbalance` → "Opening Balance" / "Solde initial"                                                                                                   |
| TRX-053 | presenter        | unchanged — `microToFormatted(tx.unit_price)` displays the stored value                                                                                                   |
| TRX-054 | presenter        | `realizedPnl: null` for OpeningBalance rows; cell already renders empty                                                                                                   |
| TRX-055 | component        | `AccountDetailsView` summary header renders the `Open Balance` button                                                                                                     |
| TRX-057 | component        | `OpenBalanceModal` submit button: `loading={isSubmitting}` and `disabled` while submitting                                                                                |
| TRX-058 | hook             | `useOpenBalance` shows snackbar + invokes `onSubmitSuccess` (parent calls `retry()` to refresh)                                                                           |

---

### E2E

- New folder: `e2e/opening_balance/`
  - `opening_balance.test.ts` — flow:
    1. App boots; create a fresh account ("Acc Test").
    2. Navigate to Account Details.
    3. Click the new "Open Balance" header button (selector by `aria-label`).
    4. Fill asset, date (today), quantity (1), total cost (100).
    5. Submit; assert success snackbar; modal closes.
    6. Holding row appears with quantity = 1.000 and avg price = 100.000 (cost basis 100.000).
    7. Open the transaction list for the new holding; assert the row shows type "Opening Balance" and unit price 100.000.
    8. Edit the row → only date/qty/total_cost visible (TRX-051); change total_cost to 200; save; row updates accordingly.
    9. Delete the row; assert holding disappears.
- Selectors must follow `docs/e2e-rules.md` (E1–E4): form id `open-balance-form`, field ids `open-balance-asset`, `open-balance-date`, `open-balance-quantity`, `open-balance-total-cost`, submit `button[type="submit"][form="open-balance-form"]`, header button `button[aria-label="Open Balance"]` (or the localized fallback).

---

## Open Questions

**OQ-A — ~~Use case vs same-BC service method~~** _(resolved)_: Option 2 chosen — `open_holding` stays in `AccountService`. Returns `ArchivedAsset` error instead of auto-unarchiving. No use case needed. TRX-050, TRX-056, contract, and plan updated accordingly.
