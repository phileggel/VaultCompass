# Tech Debt

Observations of code smells, brittle patterns, or pre-existing issues surfaced
during work that don't warrant immediate action. Format produced by the kit's
`/techdebt` skill — see `.claude/kit-tools.md`.

Entries are observations, not commitments. Triaged by `/whats-next` alongside
`docs/todo.md`. Remove an entry once it has been resolved or moved to
`docs/todo.md` for active work.

---

## 2026-05-24 — Rust test functions missing `test_` prefix project-wide

- Found by: reviewer-backend (during ISIN-lookup-split review)
- Where: src-tauri/src/ (project-wide — 315 of 391 test functions use descriptive naming without the `test_` prefix)
- Context: branch `feat/explicit-isin-lookup` @ `30ec513`
- Severity: 🔵
- Observation: `docs/test_convention.md` mandates the `test_<subject>_<condition>_<expected_outcome>` naming pattern, but the codebase has organically settled on descriptive names without the `test_` prefix (e.g. `validates_ishares_sp500_isin`, `rejects_empty_string_as_wrong_length`). Only ~20% of test functions (76 of 391) carry the prefix. The reviewer surfaced 26 new tests in the ISIN-lookup feature that follow the existing local convention but diverge from the doc. Resolution direction (project-wide rename to align with the doc OR doc update to codify the de facto pattern) is a separate decision and a separate MR; either path is mechanical but spans every Rust test file in the repo.

---

## 2026-05-24 — WEB-050a keyword filter narrower than WEB-023 mapping

- Found by: spec-reviewer
- Where: docs/spec/asset-web-lookup.md (WEB-050a + WEB-023)
- Context: branch `feat/explicit-isin-lookup` @ `30ec513`
- Severity: 🟡
- Observation: WEB-050a restricts the keyword `/v3/search` request to `securityType: "Common Stock"`, but WEB-023 publishes a broader asset-class mapping (ETF, MutualFunds, Bonds, DigitalAsset, RealEstate, Cash, Derivatives). Practical effect: on the keyword path only stocks can surface; ETFs, bonds, mutual funds, etc. are reachable only via the ISIN path. Pre-existing inconsistency introduced by 4fd0f2e; surfaced by spec-reviewer during the ISIN-lookup-split amendment, out of scope for that amendment.

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

## 2026-05-18 — E2E nav selectors are locale-coupled (E4 violation)

- Found by: reviewer-e2e
- Where: `e2e/account_details/auto_fetch.test.ts`, `e2e/account_details/buy_sell.test.ts`, `e2e/account_details/cash.test.ts`, `e2e/accounts/accounts.test.ts` (all use aria-label-based nav selectors)
- Context: branch `feat/mkt-stooq-autofetch-e2e` @ `da7471f`
- Severity: low
- Observation: E2E nav selectors target `button[aria-label="Assets"]`, `aria-label="Accounts"`, `aria-label="Settings"`, `aria-label="Price history"` — all locale-coupled per E4. `wdio.conf.ts` forces English so they work today, but rename of any i18n key or non-English run silently breaks navigation. Pattern is established across 4+ E2E files and not introduced by any single PR.

---

## 2026-05-23 — snackbarStore lives in src/lib/ instead of src/ui/components/snackbar/ (F28)

- Found by: reviewer-frontend + reviewer-arch (during F27 cleanup for refresh-prices hooks)
- Where: `src/lib/snackbarStore.ts` (imported as `@/lib/snackbarStore` by 15+ hooks/components across `features/account_details`, `features/accounts`, `features/transactions`, `features/assets`)
- Context: branch `fix/f27-refresh-prices-hooks` @ `a2c022c`
- Severity: 🟡
- Observation: `snackbarStore` is a stateful UI runtime; per F28 it belongs under `src/ui/components/snackbar/snackbarStore.ts` (colocated with its widget), not in the pre-gold `src/lib/` bucket. Move is mechanical (file rename + path-alias update across ~15 callers) but spans many features outside any single PR's scope. Same pattern as the broader `src/lib/` → F28-bucket migration already tracked above; this entry just calls out snackbarStore specifically since it surfaces on every snackbar-dispatching feature.

## 2026-05-24 — Dialog dialog/close-button lack stable F25 ids

- Found by: reviewer-frontend (during Dialog viewport-clip fix review)
- Where: `src/ui/components/modal/Dialog.tsx:54` (role="dialog" surface), `src/ui/components/modal/Dialog.tsx:69` (close button uses `data-testid="modal-close-btn"`)
- Context: branch `fix/e2e-navigate-click-intercept` @ `148aed1`
- Severity: 🔵
- Observation: F25 mandates stable `id` attributes on dialog containers and prefers `id` over `data-testid` for E2E selectors. Dialog's `role="dialog"` surface has no `id`, and the close button is selectable only via `data-testid="modal-close-btn"`. Migration would touch every Dialog consumer (8+ feature modals) plus the `dismissLeftoverModal` E2E helper which queries the testid — multi-file fanout outside any single fix's scope.

## 2026-05-24 — contract — account: event-naming and error-coverage gaps vs wire surface

- Found by: contract-reviewer
- Where: `docs/contracts/account-contract.md`
- Context: branch `docs/contracts-wire-only-account-update` @ `c90eb5e`
- Severity: 🟡
- Observation: Five gaps between the contract and the actual wire surface, surfaced during the wire-only framing migration. (1) Published `AccountUpdated` row cites TRX-037 which actually defines `TransactionUpdated`; the BE emits two distinct events (`AccountUpdated` on CRUD via `AccountService::emit_account_updated`, `TransactionUpdated` on transaction ops via `emit_transaction_updated` — see `src-tauri/src/context/account/service.rs:453-461`) but the published `TransactionUpdated` event is entirely missing from the contract. (2) Subscribed `AccountUpdated` row cites ACD-039 incorrectly — ACD-039 in `docs/spec/account-details.md` reacts to `TransactionUpdated`, not `AccountUpdated`; confirmed in `src/features/account_details/account_details_view/useAccountDetails.ts:65`. (3) `correct_transaction` and `cancel_transaction` Errors columns omit `AccountNotFound { account_id }`, reachable via the shared `load_account` helper at `src-tauri/src/context/account/service.rs:470`. (4) `UnitPriceNegative` (TransactionDomainError) wire-reachable for all HoldingTransactionError mutating commands (`src-tauri/src/context/account/domain/transaction.rs:291`) but absent from every command's Errors column. (5) `open_holding` Errors column omits `InvalidDate` (TRX-046), part of OpenHoldingError via TxValidation (`src-tauri/src/use_cases/holding_transaction/error.rs:61`). Shared root cause: the contract was never reconciled against the actual wire surface after the BE landed.

## 2026-05-24 — ISIN country prefix `IE` → only Dublin (`ID`) in primary-venue table

- Found by: manual (user testing IE00B53L3W79 — iShares Core S&P 500 UCITS ETF)
- Where: `src-tauri/src/use_cases/asset_web_lookup/primary_listing_processor.rs` (`ISIN_COUNTRY_TO_PRIMARY_VENUES`)
- Context: branch `fix/e2e-selectors-post-rename` @ `44701f8`
- Severity: 🔵
- Observation: The curated table maps `IE → ["ID"]` (Dublin / Irish Stock Exchange). That is correct for Irish equities (Ryanair, CRH, Kerry Group — all Dublin-listed) but misleading for Irish-domiciled UCITS ETFs, which constitute the vast majority of `IE0...` ISINs encountered in practice. These ETFs are domiciled in Dublin for tax/regulatory reasons but trade primarily on LSE, Amsterdam (XAMS), Xetra (Frankfurt), Borsa Italiana, and SIX Swiss — Dublin itself carries little or no liquidity for them. Today the WEB-050e filter fails to find a Dublin entry for these ETFs and falls through to `GLOBAL_VENUE_PRIORITY`, which surfaces Amsterdam first. The user's reaction ("I expected Ireland") confirms the misleading framing. Possible refinement: extend `IE → ["ID", "LO", "NA", "GY"]` so Dublin still wins for Irish equities (when present) but UCITS ETFs get a useful default ordering. Tradeoff: requires curation review to confirm no edge cases regress.
