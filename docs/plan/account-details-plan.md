# Implementation Plan: Account Details Feature (ACD)

**Spec Document:** `docs/spec/account-details.md`
**Trigram:** ACD

## 1. Workflow TaskList

- [ ] 📖 Review Architecture & Rules (`ARCHITECTURE.md`, `backend-rules.md`, `frontend-rules.md`)
- [ ] 🏗️ Backend Implementation (Domain, Repository, Service, API)
- [ ] 🔗 Type Synchronization (`just generate-types`)
- [ ] 💻 Frontend Implementation (Gateway, Hook, Component, i18n)
- [ ] 🧹 Formatting & Linting (`just format` + `python3 scripts/check.py`)
- [ ] 🔍 Code Review (`reviewer`)
- [ ] 🎭 UX Review (`ux-reviewer`)
- [ ] 🌐 i18n Review (`i18n-checker`)
- [ ] 🔧 Script Review (`script-reviewer`)
- [ ] 🧪 Unit & Integration Tests
- [ ] 📚 Documentation Update (`ARCHITECTURE.md` + `docs/todo.md`)
- [ ] ✅ Final Validation (`spec-checker` + `workflow-validator`)

---

## 2. Detailed Implementation Plan

### Backend (Context: `account/`)

- **Domain**:
  - `src-tauri/src/context/account/domain/holding.rs`: Add `HoldingPerformance` DTO with Specta derive.
- **Repository**:
  - `src-tauri/src/context/account/repository/holding.rs`: Implement `get_by_account_id(account_id: Uuid) -> Result<Vec<Holding>>`.
- **Service**:
  - `src-tauri/src/context/account/service.rs`: Add `AccountDetailsService`.
  - Method `get_account_details(id: Uuid) -> Result<AccountDetailsDTO>`.
  - Logic for fallback (ACD-027) using `AssetPriceRepository::get_latest(asset_id)`.
- **API**:
  - `src-tauri/src/context/account/api.rs`: Register `get_account_details`.
  - `src-tauri/src/core/specta_builder.rs`: Add `get_account_details` to `collect_commands!`.

### Frontend (`src/features/accounts/`)

- **Gateway**:
  - `src/features/accounts/gateway.ts`: Add `getAccountDetails: (id: string) => Promise<Result<AccountDetailsDTO, string>>`.
- **Feature Structure**:
  - `src/features/accounts/account_details/AccountDetailsView.tsx`: Main page layout.
  - `src/features/accounts/account_details/useAccountDetails.ts`: Fetch hook.
  - `src/features/accounts/account_details/HoldingTable.tsx`: Position list.
- **Reactivity**:
  - `src/lib/store.ts`: Listen for `TransactionUpdated` / `AssetUpdated` events in the `EventBus` to re-fetch account details.

---

## 3. Rules Coverage Mapping

| Rule        | Implementation Strategy                                                     |
| :---------- | :-------------------------------------------------------------------------- |
| **ACD-020** | `SqliteHoldingRepository::get_by_account_id` filtering `quantity > 0`.      |
| **ACD-024** | Use `i128` for VWAP and market value in `AccountDetailsService`.            |
| **ACD-027** | Fallback logic: `AssetPrice` -> last transaction rate.                      |
| **ACD-040** | `events.event.listen` in `src/lib/store.ts` + `useAccountDetails` re-fetch. |

---

## 4. Architectural Constraints

- **ADR-001**: Always use `i64` micro-units for currency.
- **Convention**: Follow Rust `snake_case` and TS `camelCase`.
- **Performance**: Aggregate performance metrics in `AccountDetailsService` (Rust layer).
