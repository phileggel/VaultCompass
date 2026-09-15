# Tech Debt

Observations of code smells, brittle patterns, or pre-existing issues surfaced
during work that don't warrant immediate action. Format produced by the
`/techdebt` skill.

Entries are observations, not commitments, and this file is the agent's: it
files here what it notices and what it did not fix. Each entry carries a
permanent `TD-NNN` reference (never renumbered, never reused; next free:
TD-027) so the human can queue it in `docs/todo.md` § Next like any todo.
Remove an entry once it has been resolved.

---

## 2026-08-23 — TD-001 — Local writes do not take the sync gate

- Found by: reviewer-security + reviewer-backend (PR-C, `.review/reviewer-security-2026-08-23-01.md`)
- Where: src-tauri/src/context/sync/application/run.rs (`SyncGate`), every synced repository write
- Context: branch `feat/multi-device-sync-resolve` @ `4d6ec37`
- Severity: 🟡
- Observation: SYN-064 says a local write and an in-progress apply never interleave. The apply holds `SyncGate` and runs in one SQLite write transaction with the change recorder suspended; local writes do not take the gate. SQLite's single writer serialises them at the database level and the recorder reads the logical clock under that lock, so the remaining window is a local write that computed `based_on` before an apply committed — benign for the rank order, but not the guarantee the spec states. Closing it means a `begin_write()` helper on the recorder that takes the gate before opening the transaction, applied at all 30 capture sites — its own PR.
- User value: None observable — the remaining window is benign for merge order.
- Done when: A `begin_write()` recorder helper takes `SyncGate` before opening the transaction at all 30 capture sites, so SYN-064 holds as written.

## 2026-08-23 — TD-002 — Held-back changes and conflict notices have no bound

- Found by: reviewer-security (PR-C)
- Where: src-tauri/src/context/sync/application/run.rs (`apply_intake`), `held_back_changes`, `conflict_notices`
- Context: branch `feat/multi-device-sync-resolve` @ `4d6ec37`
- Severity: 🟡
- Observation: A hostile or buggy peer could grow `held_back_changes` without bound (every run retries all of them) and `conflict_notices` never evicts. Unreachable for one user's own desktops; add caps / eviction before any multi-user or untrusted-peer scenario. Note (2026-09-12): a cap or eviction contradicts SYN-066 as written ("persist until the user dismisses them individually"), so this is a spec decision before it is a code change.
- User value: None for a user's own devices; bounds growth caused by a buggy or hostile peer.
- Done when: `held_back_changes` has a cap and `conflict_notices` evicts, both covered by tests.

## 2026-08-23 — TD-003 — Join replays a device's whole history in memory

- Found by: reviewer-security (PR-C)
- Where: src-tauri/src/context/sync/application/join.rs
- Context: branch `feat/multi-device-sync-resolve` @ `4d6ec37`
- Severity: 🔵
- Observation: Only the per-file 64 MiB cap bounds a join; the full history of each device is held in memory inside one transaction. Acceptable under the KISS cut (a personal portfolio's history is a few KB a month); revisit with checkpoints if history or device count grows.
- User value: None at present history sizes.
- Done when: Join streams or checkpoints history instead of holding every device's full history in one in-memory transaction.

## 2026-08-22 — TD-004 — Account-deletion cascade is no longer a single transaction

- Found by: reviewer-backend (PR-A change capture, `.review/reviewer-backend-2026-08-22-01.md`)
- Where: src-tauri/src/context/account/service.rs (`delete` / `remove_children`)
- Context: branch `feat/multi-device-sync-capture` @ `c7471e9`
- Severity: 🟡
- Observation: To record one change + tombstone per child (SYN-024, CFR-030), `AccountService::delete` now removes transactions, holding notes, fee schedules and catch-up positions through their own repositories — each atomic with its own change — before deleting the account. Previously a single `DELETE` with `ON DELETE CASCADE` did it all in one transaction. A crash mid-cascade leaves a half-deleted account (recoverable on retry, never silently diverging, since every child removal carries its change). Restoring single-transaction semantics needs a transaction spanning several repositories — the unit of work ADR-006 describes and the codebase never built (see the ADR-006 entry above). Fold the cascade into that unit of work when it lands.
- User value: A crash midway through deleting an account cannot leave it half-deleted.
- Done when: The cascade runs inside one unit of work spanning the child repositories.

## 2026-08-22 — TD-005 — ADR-006 unit of work is accepted but unimplemented

- Found by: feature-planner (multi-device sync plan, D1) — confirmed by plan-reviewer and by grep
- Where: src-tauri/src/ (no `UnitOfWork` / `TransactionManager` / `uow` anywhere; three raw `sqlx` transactions at `context/account/repository/account.rs:268`, `context/asset/repository/category.rs:109`, `context/asset/repository/asset_price.rs:128`)
- Context: branch `feat/multi-device-sync` @ `a9ed932`
- Severity: 🟡
- Observation: `docs/adr/006-unit-of-work.md` is Accepted and `docs/uow_example.md` documents the pattern, but nothing in the codebase implements it; cross-aggregate writes open ad-hoc transactions. ADR-019 originally relied on it for change capture and was amended in place to a `ChangeRecorder` port on the live connection instead. Decide later whether to implement ADR-006 (and route the recorder through it) or supersede it; status left Accepted meanwhile.
- User value: None directly.
- Done when: ADR-006 is implemented and the three ad-hoc `sqlx` transactions route through it, or the ADR is superseded.

## 2026-07-25 — TD-006 — Linux bundle carries Tauri template leftovers

- Found by: main-agent
- Where: src-tauri/Cargo.toml, src-tauri/tauri.conf.json
- Context: branch `main` @ `09d622c`
- Severity: 🟡
- Observation: The `.deb` installs the app binary under the template name `tauri-app` (not `vault-compass`) and also packages `generate_bindings`, a dev-only helper binary, into the installer. The Windows NSIS pipeline is likely affected the same way (binary name inside the installer). Menu entries and app labels are correct; only the on-disk binary names and the extra packaged binary are off. The Linux release job now publishes this bundle as an AppImage and a `.deb`, so the leftovers ship to users. Renaming the binary reaches `wdio.conf.ts` (`BINARY_NAME`), `scripts/screenshot.sh`, and the Windows installer path, and changing an installed binary name between versions is an updater upgrade question — it needs its own PR.
- User value: The installed binary is named after the app, and no dev-only helper ships inside the package.
- Done when: The `[[bin]]` is renamed, `generate_bindings` is excluded from the bundle, `wdio.conf.ts` and `scripts/screenshot.sh` follow, and the Windows updater upgrade path across the rename is verified.

---

## 2026-05-16 — TD-007 — ADR status vocabulary lacks an "amends" relationship

- Found by: adr-reviewer (during review of ADRs 008/009/010/011)
- Where: docs/adr/003-cross-context-use-case-orchestration.md, docs/adr/005-account-details-inject-transaction-service.md, docs/adr/README.md
- Context: branch `docs/adr-asset-valuation` @ `4d706dd`
- Severity: 🔵
- Observation: ADR-003 carries `Status: Accepted — amended by ADR-005` and ADR-005 carries `Status: Accepted — amends ADR-003`. The kit's `adr-writer` skill permits only three status values (`Accepted`, `Accepted — supersedes ADR-{NNN}`, `Superseded by ADR-{NNN}`). The "amends / amended by" relationship — capturing "this ADR refines another without superseding it" — has no permitted encoding, so the local files use an unsanctioned vocabulary that won't pass strict reviewer checks. The kit gap (no "amends" relationship class) is the upstream cause; the local files reflect that gap. Not fixed in the ADR-asset-valuation branch because resolving it requires either an upstream kit decision (add an "amended by" vocabulary) or a deliberate local decision to convert ADR-003 → Superseded by ADR-005 (loses the "still partly valid" nuance) — both are out of scope for an in-place ADR edit.
- User value: None — ADR vocabulary.
- Done when: The kit permits an “amends” status, or ADR-003 and ADR-005 are converted to a permitted one.

---

## 2026-05-10 — TD-008 — Migrate to FE gold layout (per kit proposals #21–#23)

- Found by: manual (post-FE-architecture delta scan)
- Where: src/ (top-level structure + features/account_details cross-imports)
- Context: branch `main` @ `114cb79`
- Severity: 🟡
- Observation: Three FE layout/coupling deltas surfaced by mirroring the BE architecture revisit on the frontend. The current shape works but encodes implicit conventions that diverge from the kit gold layout (now codified as F26/F27/F28 in `docs/frontend-rules.md` since kit v4.6+; the original kit issues phileggel/claude-kit#21/#22/#23 are effectively ratified). Migration is bit-by-bit per `CLAUDE.md` § Gold Standards & Bit-by-Bit Trajectory — apply gold to new code; defer existing-code reshape unless it fits the 50-LOC + locality + mechanical gates.
  1. ~~**`src/lib/update/` is a feature, mislocated.**~~ Resolved 2026-07-11 (v0.36.0 batch): the banner UI + hook moved to `src/features/update/`; the updater command adapter stayed a platform adapter at `src/lib/updateGateway.ts` (reviewer-arch: a feature-owned gateway would force `about_page` into a cross-feature import — the refined cut is UI = feature, command adapter = lib).

  2. **`features/account_details/{buy,sell}_transaction/` cross-imports from `features/transactions/`.** Today the imports are `RecordPriceCheckbox` (component), `TransactionFormData` (type), `validateTransactionForm` / `validateSellForm` (pure functions), and `useTransactions` (hook with state). Per the F23 reframing in kit proposal #21, the first three (primitives) become fine; the fourth (behavior coupling via a hook) remains a code smell. Either `account_details` owns its own thin wrapper around the gateway calls it needs, or the two features consolidate. Worth deciding _with_ the consolidation question (delta #3) rather than fixing the hook coupling alone.

  3. **`account_details` sub-feature bloat (8 sub-features).** Half of them — `buy_transaction`, `sell_transaction`, `deposit_transaction`, `withdrawal_transaction` — are conceptually transaction-recording flows and overlap with the `transactions/` feature. Two reasonable shapes: (a) consolidate the four into `transactions/` and let `account_details` stay focused on the holdings view, or (b) formalize the split — `account_details` owns "modals invoked from the holding row," `transactions/` owns "the transaction list page and its CRUD." Pick (b) as the lighter move; (a) is a bigger refactor.

  4. **`src/lib/*Storage.ts` adapters belong in `src/infra/settings/`.** The browser-`localStorage` UI-preference adapters (`autoFetchStorage.ts`, `autoRecordPriceStorage.ts`, `lastOperationDateStorage.ts`, `closedSectionStorage.ts`) are platform adapters per F28's Store-kinds table and should move to `src/infra/settings/`. New ones keep landing in `src/lib/` to stay consistent with their siblings (a partial move would orphan one file mid-migration). Mechanical folder move + import-path update; fold into the same `lib/ → infra/` rename PR.

  Migration is mechanical for #1/#4 (folder move + import sites) and conventional for #2/#3 (depends on the consolidation decision). Cleanest as one or two dedicated PRs after the kit proposals land (so the project mirrors the kit-ratified spec).

- User value: None — internal layout.
- Done when: `src/lib/*Storage.ts` moves to `src/infra/settings/`, the `useTransactions` cross-feature coupling is removed, and the `account_details` / `transactions` split is formalised.

## 2026-05-09 — TD-009 — Migrate to gold DDD layout (per kit proposals #17–#19)

- Found by: manual (post-PR-#12 design discussion)
- Where: src-tauri/src/ (top-level structure)
- Context: branch `main` @ `eb4e180`
- Severity: 🟡
- Observation: Three layout deltas from the kit gold target (now codified as B0/B37–B43 in `docs/backend-rules.md` since kit v4.4+; the original kit issues phileggel/claude-kit#17/#18/#19 are effectively ratified). The current shape works but documents the architecture imperfectly to newcomers. Migration is bit-by-bit per `CLAUDE.md` § Gold Standards & Bit-by-Bit Trajectory — apply gold to new code; defer existing-code reshape unless it fits the 50-LOC + locality + mechanical gates.
  1. **`service.rs` lives at the BC root, not in `application/`.** Inconsistent with `domain/` and `repository/` (which ARE folders). Migrate `service.rs` → `application/service.rs` per BC. Note (2026-06-27, v0.28.0 T1): the `account/` BC no longer has an `application/` folder at all — moving `{BC}Error` to the BC root (`error.rs`, per error-model gold) emptied it, so it was removed rather than left as an empty marker. The `application/` folder returns for `account/` when this `service.rs → application/service.rs` migration lands. reviewer-arch flagged the missing folder as a B38 gap; deferred here (an empty layer folder while the BC is otherwise old-layout — `service.rs` at root, `repository/` not `infrastructure/` — would be speculative scaffolding).

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

- User value: None — internal layout.
- Done when: `service.rs` moves to `application/service.rs`, `repository/` becomes `infrastructure/`, and `core/` becomes `shared/{application,domain,infrastructure}` across every bounded context.

## 2026-09-12 — TD-010 — A different portfolio's folder reads as a reset

- Found by: manual (closing the empty-sync-folder todo)
- Where: src-tauri/src/context/sync/application/run.rs (`header_gate`)
- Context: branch `chore/next-2026-09` @ `6c2fcd4`
- Severity: 🔵
- Observation: When a removable volume's drive letter is reused by a different stick that happens to carry a `VaultCompass` folder, the header decodes but its passphrase check fails — exactly what a genuine "started over elsewhere" (SYN-071) looks like from the header alone. Both write a fresh header with a new creation mark, so the two cases are indistinguishable by content; the device reports `PortfolioReset` (SYN-084) where "this is another portfolio" would be the honest message. `FolderHoldsOtherPortfolio` exists as the enable-path error but nothing in the run can justify raising it.
- User value: The reset message would not fire for a stick that merely took the same drive letter.
- Done when: A sync run can tell a reset of its own portfolio from another portfolio's folder — by something other than the header's content — and reports `FolderHoldsOtherPortfolio` for the latter.

## 2026-09-12 — TD-012 — Ubiquitous-language Domain Events table lags the event enum

- Found by: reviewer-arch (T2)
- Where: docs/ubiquitous-language.md § Domain Events; src/lib/store.ts `locallyHandledEvents`
- Context: branch `chore/next-2026-09` @ `6e211b2`
- Severity: 🔵
- Observation: The table omits `FeeScheduleUpdated`, `AssetPriceFetchProgress` and `SyncCompleted`, all of which the enum carries; and `AssetPriceUpdated` (MKT-037) is handled by its own views yet is absent from the store's locally-handled allowlist, so each publish logs an "unhandled event" debug line. The two events added today are registered in both places; the older gaps are untouched.
- User value: None — documentation and a debug-log nuisance.
- Done when: every `Event` variant has a row in the table, and the allowlist names every event the global store deliberately ignores.

## 2026-09-12 — TD-013 — Applied writes announce themselves before the apply transaction commits

- Found by: reviewer-backend (T2, `.review/reviewer-backend-2026-09-12-02.md`)
- Where: src-tauri/src/context/sync/application/run.rs (`apply_intake`, one transaction for the whole apply); every `apply_*` service method that publishes
- Context: branch `chore/next-2026-09` @ `6e211b2`
- Severity: 🟡
- Observation: `apply_fee_schedule`, `apply_currency_rate`, `apply_holding_note`, `apply_currency_pair` and both `apply_removal`s publish their event from inside the sync apply transaction, which commits once at the end of the batch. A subscriber that re-fetches on the event reads the pre-transaction snapshot (SQLite readers on another connection never see uncommitted rows), and if a later item in the same batch fails and rolls the transaction back, the event announced a write that never happened. The subscriber's next refresh corrects it; the pattern predates this branch.
- User value: None observable today — a view may refresh one moment too early after a sync and show the state from before the apply until the next event.
- Done when: the apply path collects the events its writes would raise and publishes them after `commit()`, so every announcement describes a committed state.

## 2026-09-12 — TD-015 — Three E2E specs select by text or duplicate a shared helper

- Found by: reviewer-e2e (`.review/reviewer-e2e-2026-09-12-01.md`, pre-existing section)
- Where: e2e/accounts/accounts.test.ts:23 (local `navigateToAccounts` next to the shared one in e2e/helpers/navigation.ts), e2e/asset_web_lookup/asset_web_lookup.test.ts:47 (`button[aria-label="Fill manually"]`), e2e/assets/assets.test.ts:79 and :102 (XPath on `normalize-space(text())`)
- Context: branch `ci/e2e-on-pull-requests` @ `2088878`
- Severity: 🔵
- Observation: Two specs locate elements by their English label or cell text rather than a stable id (E4), which ties them to the forced `en_US` locale and to copy that the i18n files own; one spec carries its own copy of a navigation helper the shared module already provides, so a change to the accounts route has two places to drift.
- User value: None — suite robustness.
- Done when: the three sites select by `id` (adding the ids on the frontend elements in the same commit) and the local helper is replaced by the shared import.

## 2026-09-12 — TD-016 — A controlled-input value can be lost once in the E2E buy flow

- Found by: manual (first pull-request E2E run, PR #106 attempt 1, run 34717977006)
- Where: e2e/account_details/buy_sell.test.ts (TRX-010), e2e/helpers/react.ts (`setReactInputValue`), src/ui/components/field/CalcField.tsx
- Context: branch `ci/coverage-gates` @ `2088878`
- Severity: 🟡
- Observation: TRX-010 failed with `submit still not enabled after 5000ms`; the failure screenshot shows the date and the unit price filled and the quantity field empty, so the value set by `setReactInputValue("buy-trx-quantity", "10")` between the two others did not stick. The same spec passed six times on `main` the same day and the field's own state sync guards against prop clobbering, so no deterministic path is known. With E2E as a required check, a once-in-N loss of a set value is a merge blocked for a reason unrelated to the change.
- User value: None — suite reliability.
- Done when: the loss is reproduced (or its trigger understood) and either the helper waits for the field to report the value back before returning, or the field's handling is changed so a dispatched `input` event can never be dropped; TRX-010 no longer needs a re-run to pass.

## 2026-09-12 — TD-017 — Backend logic coverage sits at 86 % against the 90 % target

- Found by: manual (`python3 scripts/coverage-gate.py --backend` on the tarpaulin report of `6ac326c`)
- Where: src-tauri/src/use_cases/update_checker/service.rs (0 % of 75 lines), src-tauri/src/use_cases/scheduled_fetch/headless.rs (3 % of 73), src-tauri/src/use_cases/asset_web_lookup/orchestrator.rs (38 % of 108), src-tauri/src/use_cases/portfolio_sync/applier.rs (66 % of 119), src-tauri/src/context/sync/application/join.rs (72 % of 148), src-tauri/src/use_cases/holding_transaction/orchestrator.rs (78 % of 231), src-tauri/src/context/account/service.rs (83 % of 737), src-tauri/src/context/asset/service.rs (83 % of 391)
- Context: branch `ci/coverage-gates` @ `6ac326c`
- Severity: 🟡
- Observation: 86.18 % of the 6,744 lines in domain, application, service and use-case code are covered; the gate's floor is 85.5 % and its target 90 %, about 260 more covered lines. Two files carry almost no test at all because they talk to the network or run the app headless; the other six are orchestration paths with untested branches. The floor in `coverage-gates.json` is a ratchet — raise it in the same change that lifts coverage, never lower it.
- User value: None — a harness that catches logic regressions in these paths.
- Mutation survivors: the 2026-09-14 sweep (issue #137) found 301 logic changes no test notices — `context/account/domain/account.rs` 52, `use_cases/shared/valuation.rs` 33, `context/sync/domain/resolution.rs` 24, `use_cases/global_performance/orchestrator.rs` 21; each names an assertion that is missing or too weak.
- Done when: the backend floor in `coverage-gates.json` reads 90.0 and the gate passes on `main`.

## 2026-09-13 — TD-018 — 119 interactive components in feature code carry no id

- Found by: manual (`python3 scripts/arch-check.py`, rule A6, first run)
- Where: 35 files under src/features/ listed in arch-allowlist.json `missing_ids`; mostly Cancel and secondary buttons in modals, the price-history and update-banner actions, and the design-system dev page
- Context: branch `ci/arch-check` @ `280882f`
- Severity: 🔵
- Observation: E1–E4 ask every interactive element for a stable id, and the E2E suite selects by id, yet 119 of the 317 `Button` / `IconButton` / `TextField` / `DateField` / `CalcField` / `FAB` tags rendered by feature code have none. The architecture check freezes today's count per file and refuses any growth; the count can only go down. The 18 sibling-feature imports the same check freezes belong to the FE gold layout migration entry above; the 8 `Math.` uses are display rounding and the documented split preview (SPL-061) and need no action.
- User value: None — every control becomes addressable by tests and assistive tech.
- Done when: `missing_ids` in arch-allowlist.json is empty.

## 2026-09-13 — TD-019 — The assets spec's before-each hook can hit a stale element

- Found by: manual (PR #111 E2E attempt 1, run 34746573443; the PR touched no app code)
- Where: e2e/assets/assets.test.ts (`beforeEach`), e2e/helpers/modal.ts (`dismissLeftoverModal`), e2e/helpers/navigation.ts (`navigateToAssets`)
- Context: branch `test/golden-portfolio` @ `361ece6`
- Severity: 🟡
- Observation: The hook failed with `stale element reference` while creating a node handle for an `element` call — an element located by one step had been replaced by a re-render before the next step used it. It is the second distinct once-only E2E failure in two days (TD-016 is the first); both sit in setup or navigation code shared by many specs, so each has many chances to fire per run. With E2E as a required check, every such failure costs a re-run before a green PR can merge.
- User value: None — suite reliability.
- Done when: the hook re-locates elements after each navigation step instead of reusing handles across renders, or the shared helpers wait for the route to settle before returning; a month of pull-request runs shows no before-each failure.

## 2026-09-13 — TD-021 — Action pins labelled with the wrong tag in four workflows

- Found by: reviewer-infra (phase 12 review, `.review/reviewer-infra-2026-09-13-10.md`)
- Where: `.github/workflows/quality.yml`, `e2e.yml`, `review.yml`, `security-audit.yml` — `Swatinem/rust-cache@e18b4977…` (labelled v2.9.1; the tag peels to `c1937114…`) and `taiki-e/install-action@f48d2f8b…` (labelled v2.75.30; it is v2.79.6)
- Context: branch `ci/mutation-sweep` @ `5f29843`
- Severity: 🔵
- Observation: the commits are real upstream commits, so nothing is compromised, but a reader trusting the comment audits the wrong release notes. `mutants.yml` carries the correct pins; the four older files still carry the labels. Verified with `git ls-remote --tags` on 2026-09-13.

## 2026-09-14 — TD-023 — E2E specs still locate controls by label, role or form attribute

- Found by: manual (selector count while fixing dangling references in the E2E headers)
- Where: `e2e/asset_web_lookup/asset_web_lookup.test.ts` (`button[aria-label="Add asset"]`, `"Back"`, `"Fill manually"`), `e2e/account_details/manual_price_fill.test.ts` and `e2e/account_details/auto_fetch.test.ts` (`[role="dialog"]`, `[role="status"]`, `body`), `e2e/open_balance/open_balance.test.ts` and `e2e/account_details/buy_sell.test.ts` (`button[type="submit"][form="…"]`)
- Context: branch `main` @ `482527b`
- Severity: 🔵
- Observation: `docs/e2e-rules.md` asks every selector to be a stable `id`; these specs still find controls by an accessible label (which changes with the locale and the wording), by role, or by the form a submit button belongs to, because the elements carry no id of their own.
- User value: None — E2E specs that survive a wording or locale change.
- Done when: every selector in those five specs is an `id`, the controls they target carry one, and `reviewer-e2e` passes on them.

## 2026-09-14 — TD-024 — Nothing in the harness checks that pinned columns stay put while scrolling

- Found by: reviewer-frontend (F18, class-only assertions on #001)
- Where: `src/features/account_details/account_details_view/{HoldingRow,ClosedHoldingRow,AccountDetailsView}.test.tsx` (pinned-column tests), `e2e/account_details/` (no scrolling scenario)
- Context: branch `c/001-sticky-actions-asset`
- Severity: 🔵
- Observation: the holdings tables pin Actions and Asset with sticky positioning inside the view's content area, but jsdom has no layout, so the Vitest tests can only assert the classes; the behaviour itself — pinned cells and the header staying put while columns scroll, row backgrounds spanning the table — was measured once in a browser and is not re-checked by any suite, so a wrapper change that breaks the scroll set-up would pass the harness.
- User value: None — a harness that notices when the pinned columns stop holding.
- Done when: an E2E scenario scrolls the account view's content area sideways on a window narrower than the holdings table (precondition: it overflows) and asserts that a holding's action button stays in place while a plain column moves, and that the header stays at the top after scrolling down.

## 2026-09-14 — TD-025 — "Reference currency" names two different things in the vocabulary

- Found by: spec-reviewer (ACC-027–033 review on #007)
- Where: `docs/ubiquitous-language.md` (Cash Holding, Dividends received, Management fees — "the account's reference currency"), `docs/spec/global-performance.md` GPF-011 and the ACC / PMV totals ("the reference currency", EUR); the same table is also "Accounts table" (ACC-021/023/026), "accounts list" (PMV-013/016, SYN-040) and "account table" (ACC-008/030–033)
- Context: branch `c/007-accounts-total-row`
- Severity: 🔵
- Observation: the vocabulary uses "the account's reference currency" for an account's own currency, while GPF-011, the price movement total and the accounts-list portfolio total use "the reference currency" for the fixed EUR every cross-account figure is reported in; the two meanings share one phrase, and neither "cross-account reference currency" nor "portfolio total" has an entry of its own.
- User value: None — one word per concept in the specs and the code.
- Done when: the vocabulary names the account's own currency and the cross-account reference currency with distinct, confirmed terms, and has a confirmed "portfolio total" entry, each validated by the human.

## 2026-09-14 — TD-026 — Nothing follows an account currency's rate to the reference currency

- Found by: spec-reviewer (ACC-027/028 second pass on #007)
- Where: `docs/spec/fx-rate.md` FXR-013 / FXR-071 (only asset → account pairs are followed), `docs/spec/global-performance.md` GPF-020, `docs/spec/account.md` ACC-027/028, `src-tauri/src/use_cases/account_summary/orchestrator.rs` (portfolio total)
- Context: branch `c/007-accounts-total-row`
- Severity: 🟡
- Observation: rates are fetched and followed for the pairs an asset needs to reach its account's currency, but no rule follows the pair from an account's currency to the cross-account reference currency; a USD account holding only USD assets therefore never gets a USD → EUR rate unless the user declares the pair, and the accounts-list portfolio total stays marked partial for it (the global performance view has the same dependency).
- User value: the portfolio total and the global performance figures count every account without the user having to declare a pair first.
- Done when: an account whose currency differs from the reference currency has its pair followed like an asset pair (or the application asks the user to declare it), stated as an FXR rule, and a USD-only account with a fetched rate no longer leaves the total incomplete.

## 2026-09-15 — TD-027 — ADR-012 does not record the price history backfill's fill-only exception

- Found by: spec-reviewer (MKT-190–199 second pass on #008)
- Where: `docs/adr/012-latest-write-wins-source-as-metadata.md` (decision 1), `docs/spec/market-price.md` MKT-192
- Context: branch `c/008-price-history-backfill`
- Severity: 🔵
- Observation: ADR-012 decision 1 says every price write upserts unconditionally; MKT-192 exempts the price history backfill, which writes only dates without a price. The spec names the exception and the ADR does not, so a reader of the ADR alone misses it; the ADR status vocabulary has no "amended by" form yet (TD-007).
- User value: None — the decision record matches the rules.
- Done when: ADR-012, or an ADR that supersedes it, names the fill-only exception, and adr-reviewer passes it.

## 2026-09-15 — TD-029 — CI reviewers spend two steps per changed file and hit the turn cap

- Found by: manual (PR #145, runs 34947453064 and 34949000885, session logs)
- Where: `.claude/agents/reviewer-*.md` (Steps 1, 3, 4), `.github/workflows/review.yml` (`--max-turns 40`)
- Context: branch `c/008-price-history-backfill`
- Severity: 🟡
- Observation: reviewer-backend (50 steps) and reviewer-frontend (44 steps) posted "No issues found" and still failed the check on the 40-step cap; re-runs on the same diffs took 32. Every session made one tool call per step, and the prompts asked for a `Glob`, a diff and a full read per changed file (11–14 diffs, 8–11 source reads) before 6–19 exploratory searches, so the step count grew with the number of files and with how much the reviewer searched. The prompts now read the whole diff in one call, batch the full reads, and search only to confirm a finding.
- User value: None — a required check that goes red only on a real critical.
- Done when: the next pull request with ten or more files in one lane runs that lane's reviewer under 30 steps (read from the session log), with no turn-cap failure.
