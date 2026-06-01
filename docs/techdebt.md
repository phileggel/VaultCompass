# Tech Debt

Observations of code smells, brittle patterns, or pre-existing issues surfaced
during work that don't warrant immediate action. Format produced by the kit's
`/techdebt` skill — see `.claude/kit-tools.md`.

Entries are observations, not commitments. Triaged by `/whats-next` alongside
`docs/todo.md`. Remove an entry once it has been resolved or moved to
`docs/todo.md` for active work.

---

## 2026-05-29 — release-manual.yml GitHub Action versions not SHA-pinned

- Found by: reviewer-infra (during CI gold-alignment review)
- Where: `.github/workflows/release-manual.yml` (`actions/checkout@v4`, `actions/setup-node@v4`, `Swatinem/rust-cache@v2`, `tauri-apps/tauri-action@v0`)
- Context: branch `chore/ci-gold-alignment` @ HEAD
- Severity: 🔵
- Observation: After the gold-CI alignment, `quality.yml` / `e2e.yml` / `release-windows.yml` / `security-audit.yml` pin every action to a full commit SHA (supply-chain hardening). `release-manual.yml` still uses mutable major tags (`@v4`, `@v2`, `@v0`), so it's now the lone inconsistency. Pin its four actions to the same SHAs already used in `release-windows.yml`. Deferred from the alignment PR to avoid speculative edits to a release workflow beyond the `releaseDraft` change the alignment required.

## 2026-05-29 — E2E account navigation: shared helper + #id row selector

- Found by: reviewer-e2e (during PRF E2E review)
- Where: `e2e/account_details/buy_sell.test.ts`, `e2e/account_details/cash.test.ts`, `e2e/account_details/auto_fetch.test.ts` (and previously `e2e/account_performance/account_performance.test.ts`)
- Context: branch `feat/account-performance-e2e` @ `4bb7056`
- Severity: 🔵
- Observation: `navigateToAccounts()` and `navigateToAccountDetails()` are duplicated verbatim across four E2E suites; `e2e/helpers/navigation.ts` only exports `navigateToAssets`. Separately, the three `account_details/*` suites still select the account row via `tr[aria-label="Open account …"]` rather than the stable `#account-row-{id}` id (the PRF E2E was migrated to `#id` this branch, and `AccountTable` now carries the row id). Resolution: promote both navigation helpers to `e2e/helpers/navigation.ts` and migrate the three `account_details` suites to the `#account-row-{id}` selector. Deferred — multi-file change across suites not otherwise touched by the PRF work.

## 2026-05-25 — Per-BC error split violates error-model gold (anti-pattern #1) — audit + collapse

- Found by: triage discussion during /whats-next (account-contract reconciliation)
- Where: see per-BC inventory below
- Context: branch `docs/bc-error-gold-audit` @ HEAD
- Severity: 🟡
- Observation: `docs/error-model.md` § Anti-patterns explicitly lists "Per-BC `*ApplicationError` / `*DomainError` split — collapse into a single `{BC}Error`" as anti-pattern #1. Gold mandates ONE flat `{BC}Error` per bounded context holding every variant (aggregate-invariant + service-layer + infra translation), with composites living ONLY in `use_cases/{name}/error.rs` wrapping one BC enum per touched BC plus a `{UseCase}Task` sub-enum. **Wire shape is unaffected today** — every leaf carries `#[serde(tag="code")]` and composites are `#[serde(untagged)]`, so the FE receives flat `{ code: "...", ... }` correctly; the split is purely internal Rust organization.

### Per-BC inventory (audit 2026-05-25)

**`context/account/` — 🟡 not gold (6 leaf enums + 2 in-BC composites)**

| Location                      | Type                                                    | Variants                                                                                                                                                                          | Under gold                                 |
| ----------------------------- | ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `domain/error.rs`             | `AccountDomainError`                                    | `NameEmpty`, `InvalidCurrency{currency}`                                                                                                                                          | merged into `AccountError`                 |
| `domain/error.rs`             | `HoldingDomainError`                                    | `NegativeQuantity`, `NegativeAveragePrice`                                                                                                                                        | merged into `AccountError`                 |
| `domain/error.rs`             | `OpeningBalanceDomainError`                             | `InvalidTotalCost`                                                                                                                                                                | merged into `AccountError`                 |
| `domain/error.rs`             | `AccountOperationError`                                 | `ClosedPosition`, `Oversell{available,requested}`, `CascadingOversell`, `TransactionNotFound`, `InsufficientCash{current_balance_micros,currency}`                                | merged into `AccountError`                 |
| `domain/transaction_error.rs` | `TransactionDomainError`                                | `InvalidDate`, `DateInFuture`, `DateTooOld`, `QuantityNotPositive`, `AmountNotPositive`, `UnitPriceNegative`, `FeesNegative`, `ExchangeRateNotPositive`, `TotalAmountNotPositive` | merged into `AccountError`                 |
| `application/error.rs`        | `AccountApplicationError`                               | `AccountNotFound{account_id}`, `NameAlreadyExists`, `DatabaseError`                                                                                                               | merged into `AccountError`                 |
| `application/error.rs`        | `HoldingTransactionError` (in-BC composite, 3 wrappers) | —                                                                                                                                                                                 | **deleted** (gold has no in-BC composites) |
| `application/error.rs`        | `AccountCrudError` (in-BC composite, 2 wrappers)        | —                                                                                                                                                                                 | **deleted** (gold has no in-BC composites) |

- Total wire-reachable variants: 22 → single `AccountError` with 22 variants.
- Variant name collisions within BC on collapse: **none**.

**`context/asset/` — 🟡 not gold (7 leaf enums + 3 in-BC composites; partial gold via `error.rs::AssetError` on fetch surface only)**

| Location               | Type                                              | Variants                                                                                                                                                                            | Under gold                                                                         |
| ---------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `error.rs`             | `AssetError` (fetch-surface only)                 | `DatabaseError`                                                                                                                                                                     | extended to hold all asset variants                                                |
| `application/error.rs` | `AssetApplicationError`                           | `NotFound{id}`, `DatabaseError`                                                                                                                                                     | merged into `AssetError` (variant renamed `AssetNotFound{id}` — see collisions)    |
| `application/error.rs` | `AssetPriceApplicationError`                      | `PriceNotFound{asset_id,date}`, `DatabaseError`                                                                                                                                     | merged into `AssetError`                                                           |
| `application/error.rs` | `CategoryApplicationError`                        | `NotFound{id}`, `DuplicateName`, `DatabaseError`                                                                                                                                    | merged into `AssetError` (variant renamed `CategoryNotFound{id}` — see collisions) |
| `domain/error.rs`      | `AssetDomainError`                                | `NameEmpty`, `ReferenceEmpty`, `InvalidRiskLevel{received}`, `InvalidCurrency{currency}`, `Archived`, `CashAssetNotEditable`, `InvalidExchange{exchange_code}`, `InvalidIsinFormat` | merged into `AssetError`                                                           |
| `domain/error.rs`      | `AssetPriceDomainError`                           | `NotPositive`, `NonFinite`, `DateInFuture`, `InvalidDateFormat{date}`                                                                                                               | merged into `AssetError` (`DateInFuture` collides only across BC → no issue)       |
| `domain/error.rs`      | `CategoryDomainError`                             | `LabelEmpty`, `SystemReadonly`, `SystemProtected`                                                                                                                                   | merged into `AssetError`                                                           |
| `application/error.rs` | `AssetPriceError` (in-BC composite, 3 wrappers)   | —                                                                                                                                                                                   | **deleted**                                                                        |
| `application/error.rs` | `AssetCrudError` (in-BC composite, 3 wrappers)    | —                                                                                                                                                                                   | **deleted**                                                                        |
| `application/error.rs` | `CategoryCrudError` (in-BC composite, 2 wrappers) | —                                                                                                                                                                                   | **deleted**                                                                        |

- Total wire-reachable variants: ~23 → single `AssetError`.
- Variant name collisions within BC on collapse: `NotFound{id}` appears in `AssetApplicationError` AND `CategoryApplicationError` — both struct variants with identical shape. **Wire-visible rename required**: `AssetNotFound{id}` + `CategoryNotFound{id}`. This breaks the FE narrowing in the gateway/presenter pipeline → contract update + i18n key update + FE switch-arm updates accompany the BC-2 PR. Other duplicates (`DatabaseError`, `DateInFuture` across BCs) are not collisions because they live in distinct enums on the wire.

**`use_cases/*/error.rs` composites**

| File                           | Composite                                                        | Wrappers (gold = BC enums + 1 Task)                                                   | Task sub-enum                                                                                       | Naming gold?                                         | Notes                                                                                |
| ------------------------------ | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `holding_transaction/error.rs` | `OpenHoldingError`                                               | 4 (Account.Application, Account.Domain, Account.OpeningBalance, Account.TxValidation) | `OpenHoldingApplicationError` (3 variants: AssetNotFound, ArchivedAsset, OpeningBalanceOnCashAsset) | ❌ task should be `OpenHoldingTask`                  | Collapses to 2 wrappers (AccountError + OpenHoldingTask) once account/ BC collapses. |
| `asset_price_fetch/error.rs`   | `FetchAllAssetPricesError` + `FetchAccountAssetPricesError`      | 3 (Asset, Account, Failure)                                                           | `FetchPriceTask` (3 variants)                                                                       | ✅ gold-conformant                                   | Reference shape.                                                                     |
| `delete_asset/error.rs`        | `DeleteAssetError`                                               | 3 (Asset.Crud, Account, Application)                                                  | `DeleteAssetApplicationError` (1 variant: ExistingTransactions)                                     | ❌ task should be `DeleteAssetTask`                  | Collapses to 2 wrappers once asset/ BC collapses.                                    |
| `archive_asset/error.rs`       | `ArchiveAssetError`                                              | 3 (Asset.Crud, Account, Application)                                                  | `ArchiveAssetApplicationError` (1 variant: ActiveHoldings)                                          | ❌ task should be `ArchiveAssetTask`                 | Same shape as DeleteAssetError.                                                      |
| `asset_web_lookup/error.rs`    | _(no composite — `WebLookupApplicationError` returned directly)_ | n/a                                                                                   | `WebLookupApplicationError` (3 variants)                                                            | ⚠ rename to `WebLookupError` (no BC wrappers needed) | Use-case-only failure surface; trivial rename.                                       |

Use cases without their own `error.rs` (return a BC enum directly, gold-conformant by transitivity once the BC collapses): `account_deletion`, `account_details`, `account_summary`, `update_checker`.

### Sequenced collapse plan

| PR       | Scope                                                                                                                                                                                                                                                        | Wire change                              | Files touched                                                                                                                                                                    | Estimate |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| **BC-1** | Collapse `account/` BC: fold 6 leafs + 2 in-BC composites into single `AccountError`                                                                                                                                                                         | none (variants keep `code` discriminant) | `account/domain/`, `account/application/`, `account/service.rs`, callsites in `use_cases/holding_transaction/` + tests (~315 test fns may reference variants)                    | 3–6h     |
| **BC-2** | Collapse `asset/` BC: fold 7 leafs + 3 in-BC composites into single `AssetError`; rename `NotFound{id}` → `AssetNotFound{id}` / `CategoryNotFound{id}`                                                                                                       | **yes** — variant rename                 | `asset/domain/`, `asset/application/`, `asset/service.rs`, asset/account contracts, FE switch arms in gateway/presenter, i18n keys (en + fr), use_cases that wrap AssetCrudError | 4–6h     |
| **UC-1** | Rename use-case task sub-enums to gold convention: `OpenHoldingApplicationError` → `OpenHoldingTask`, `DeleteAssetApplicationError` → `DeleteAssetTask`, `ArchiveAssetApplicationError` → `ArchiveAssetTask`, `WebLookupApplicationError` → `WebLookupError` | none                                     | each file under `use_cases/{name}/error.rs` + callsites                                                                                                                          | ≤1h      |

**Ordering**: BC-1 first (internal-only, low blast radius). UC-1 can ship any time (mechanical rename). BC-2 last (the wire-visible variant rename forces contract + FE coordination — easier to batch once the project is comfortable with the BC-1 pattern).

---

## 2026-05-24 — Rust test functions missing `test_` prefix project-wide

- Found by: reviewer-backend (during ISIN-lookup-split review)
- Where: src-tauri/src/ (project-wide — 315 of 391 test functions use descriptive naming without the `test_` prefix)
- Context: branch `feat/explicit-isin-lookup` @ `30ec513`
- Severity: 🔵
- Observation: `docs/test_convention.md` mandates the `test_<subject>_<condition>_<expected_outcome>` naming pattern, but the codebase has organically settled on descriptive names without the `test_` prefix (e.g. `validates_ishares_sp500_isin`, `rejects_empty_string_as_wrong_length`). Only ~20% of test functions (76 of 391) carry the prefix. The reviewer surfaced 26 new tests in the ISIN-lookup feature that follow the existing local convention but diverge from the doc. Resolution direction (project-wide rename to align with the doc OR doc update to codify the de facto pattern) is a separate decision and a separate MR; either path is mechanical but spans every Rust test file in the repo.

---

## 2026-05-16 — ADR status vocabulary lacks an "amends" relationship

- Found by: adr-reviewer (during review of ADRs 008/009/010/011)
- Where: docs/adr/003-cross-context-use-case-orchestration.md, docs/adr/005-account-details-inject-transaction-service.md, docs/adr/README.md
- Context: branch `docs/adr-asset-valuation` @ `4d706dd`
- Severity: 🔵
- Observation: ADR-003 carries `Status: Accepted — amended by ADR-005` and ADR-005 carries `Status: Accepted — amends ADR-003`. The kit's `adr-writer` skill permits only three status values (`Accepted`, `Accepted — supersedes ADR-{NNN}`, `Superseded by ADR-{NNN}`). The "amends / amended by" relationship — capturing "this ADR refines another without superseding it" — has no permitted encoding, so the local files use an unsanctioned vocabulary that won't pass strict reviewer checks. The kit gap (no "amends" relationship class) is the upstream cause; the local files reflect that gap. Not fixed in the ADR-asset-valuation branch because resolving it requires either an upstream kit decision (add an "amended by" vocabulary) or a deliberate local decision to convert ADR-003 → Superseded by ADR-005 (loses the "still partly valid" nuance) — both are out of scope for an in-place ADR edit.

---

## 2026-05-10 — Migrate to FE gold layout (per kit proposals #21–#23)

- Found by: manual (post-FE-architecture delta scan)
- Where: src/ (top-level structure + features/account_details cross-imports)
- Context: branch `main` @ `114cb79`
- Severity: 🟡
- Observation: Three FE layout/coupling deltas surfaced by mirroring the BE architecture revisit on the frontend. The current shape works but encodes implicit conventions that diverge from the kit gold layout (now codified as F26/F27/F28 in `docs/frontend-rules.md` since kit v4.6+; the original kit issues phileggel/claude-kit#21/#22/#23 are effectively ratified). Migration is bit-by-bit per `CLAUDE.md` § Gold Standards & Bit-by-Bit Trajectory — apply gold to new code; defer existing-code reshape unless it fits the 50-LOC + locality + mechanical gates.
  1. **`src/lib/update/` is a feature, mislocated.** It has full feature shape — `gateway.ts` + sub-feature folder (`update_banner/`) + hook + test — but lives under `src/lib/`. Per kit proposal #23, `lib/` (renamed `infra/`) hosts platform adapters only; features must live in `src/features/`. Move to `src/features/update/`. Mechanical rename + import-path update.

  2. **`features/account_details/{buy,sell}_transaction/` cross-imports from `features/transactions/`.** Today the imports are `RecordPriceCheckbox` (component), `TransactionFormData` (type), `validateTransactionForm` / `validateSellForm` (pure functions), and `useTransactions` (hook with state). Per the F23 reframing in kit proposal #21, the first three (primitives) become fine; the fourth (behavior coupling via a hook) remains a code smell. Either `account_details` owns its own thin wrapper around the gateway calls it needs, or the two features consolidate. Worth deciding _with_ the consolidation question (delta #3) rather than fixing the hook coupling alone.

  3. **`account_details` sub-feature bloat (8 sub-features).** Half of them — `buy_transaction`, `sell_transaction`, `deposit_transaction`, `withdrawal_transaction` — are conceptually transaction-recording flows and overlap with the `transactions/` feature. Two reasonable shapes: (a) consolidate the four into `transactions/` and let `account_details` stay focused on the holdings view, or (b) formalize the split — `account_details` owns "modals invoked from the holding row," `transactions/` owns "the transaction list page and its CRUD." Pick (b) as the lighter move; (a) is a bigger refactor.

  Migration is mechanical for #1 (folder move + ~5 import sites) and conventional for #2/#3 (depends on the consolidation decision). Cleanest as one or two dedicated PRs after the kit proposals land (so the project mirrors the kit-ratified spec).

## 2026-05-09 — Migrate to gold DDD layout (per kit proposals #17–#19)

- Found by: manual (post-PR-#12 design discussion)
- Where: src-tauri/src/ (top-level structure)
- Context: branch `main` @ `eb4e180`
- Severity: 🟡
- Observation: Three layout deltas from the kit gold target (now codified as B0/B37–B43 in `docs/backend-rules.md` since kit v4.4+; the original kit issues phileggel/claude-kit#17/#18/#19 are effectively ratified). The current shape works but documents the architecture imperfectly to newcomers. Migration is bit-by-bit per `CLAUDE.md` § Gold Standards & Bit-by-Bit Trajectory — apply gold to new code; defer existing-code reshape unless it fits the 50-LOC + locality + mechanical gates.
  1. **`service.rs` lives at the BC root, not in `application/`.** Inconsistent with `domain/` and `repository/` (which ARE folders). After PR 2b introduced `application/error.rs` per BC, the application layer has half its content in a folder, half at root. Migrate `service.rs` → `application/service.rs` per BC.

  2. **`repository/` should be `infrastructure/`** (DDD layer name). `repository/` is one TYPE of infrastructure; renaming protects against the day a BC adds an external API client, cache adapter, or message-queue subscriber (avoids proliferating peer folders). Today the folder only contains repository impls — stay flat (`infrastructure/{aggregate}.rs`) until non-repo infra arrives, then add siblings without nesting.

  3. **`core/` should be `shared/`**, restructured into the three DDD layer folders. `core/` overpromises ("central business logic" — but BCs ARE the business). Target shape:

     ```
     shared/
     ├── application/error.rs        ← shared InfrastructureError
     ├── domain/cash.rs              ← shared kernel (system_cash_asset_id)
     └── infrastructure/{db, event_bus, logger, specta_*, uow}
     ```

     `InfrastructureError` reclassifies as application-layer (it's the typed application translation of opaque infra failures, per the DDD doc's travel rule — the NAME describes the source, the LAYER is application).

  Migration is mechanical (folder moves + module-path updates, ~50–100 import sites total). Cleanest as a single dedicated chore PR after the kit proposals land (so the project mirrors the kit-ratified spec).

---

## 2026-05-24 — Dialog dialog/close-button lack stable F25 ids

- Found by: reviewer-frontend (during Dialog viewport-clip fix review)
- Where: `src/ui/components/modal/Dialog.tsx:54` (role="dialog" surface), `src/ui/components/modal/Dialog.tsx:69` (close button uses `data-testid="modal-close-btn"`)
- Context: branch `fix/e2e-navigate-click-intercept` @ `148aed1`
- Severity: 🔵
- Observation: F25 mandates stable `id` attributes on dialog containers and prefers `id` over `data-testid` for E2E selectors. Dialog's `role="dialog"` surface has no `id`, and the close button is selectable only via `data-testid="modal-close-btn"`. Migration would touch every Dialog consumer (8+ feature modals) plus the `dismissLeftoverModal` E2E helper which queries the testid — multi-file fanout outside any single fix's scope.

## 2026-05-30 — Row-level "edit asset" affordance is mouse-only

- Found by: reviewer-frontend
- Where: `src/features/account_details/account_details_view/HoldingRow.tsx:131` (holding row), `src/features/assets/asset_table/AssetTable.tsx:182` (asset row)
- Context: branch `feat/ui-tweaks-account-asset` @ `8d904a1`
- Severity: 🟡
- Observation: The row-level "edit asset" affordance (double-click on a holding row, and the analogous AssetTable row) is mouse-only; there is no keyboard equivalent. Designing keyboard parity (e.g. Enter-to-edit) needs a consistent decision across both tables, since AssetTable's Enter/Space currently selects the row rather than opening edit.

## 2026-06-01 — Backend coverage (tarpaulin `--lib`) ignores integration tests

- Found by: manual (user-flagged during FXR PR #63 codecov review)
- Where: `.github/workflows/quality.yml` (~L171) + `justfile` `coverage-be` — kit-managed
- Context: branch `feat/fx-rate` @ `ca7ff88`
- Severity: 🟡
- Observation: `cargo tarpaulin` runs with `--lib` in both local (`coverage-be`) and CI, in both VaultCompass and PatientManager (a kit-shipped pattern). `--lib` measures only inline `#[cfg(test)]` unit tests, not the `tests/*.rs` integration binaries — so code reachable only via integration tests reports 0% coverage despite those tests passing under `cargo test` + the pre-push suite. Contributors must duplicate integration coverage as inline unit tests to satisfy codecov, and true coverage is silently under-reported.
- Proposed fix (for triage): `--lib` → `--lib --tests` so integration binaries are measured (still excludes the Tauri app bin + doctests, the likely reason for bare `--lib`). The committed `.sqlx` cache + `SQLX_OFFLINE=true` already enable offline integration runs; cost is slower CI. Belongs in claude-kit so it syncs to both projects — a per-repo patch is overwritten on `just sync-kit`.

## 2026-06-01 — `period_end_dates` mirrors the build_yearly/build_monthly period iteration

- Found by: reviewer-backend
- Where: `src-tauri/src/use_cases/account_performance/orchestrator.rs:462-493`
- Context: branch `feat/fx-rate-valuation` @ `b358b4e`
- Severity: 🟡
- Observation: `period_end_dates` enumerates the valuation period-ends by re-deriving the year iteration in `build_yearly` and the month iteration + prior-year-end YTD baseline in `build_monthly`. The three loops must stay in lockstep — if a new valuation point is ever added to a build method but not to `period_end_dates`, the pre-resolved FX `rate_map` misses that date and `end_value_as_of` degrades foreign holdings to 0 (FXR-034) rather than erroring, so the resulting performance drift is silent. The duplication is currently correct and commented; the risk is future divergence, not a present bug.
