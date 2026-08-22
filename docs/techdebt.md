# Tech Debt

Observations of code smells, brittle patterns, or pre-existing issues surfaced
during work that don't warrant immediate action. Format produced by the kit's
`/techdebt` skill — see `.claude/kit-tools.md`.

Entries are observations, not commitments. Triaged by `/whats-next` alongside
`docs/todo.md`. Remove an entry once it has been resolved or moved to
`docs/todo.md` for active work.

---

## 2026-08-22 — FEE spec carries contract vocabulary and out-of-order rules

- Found by: spec-reviewer (round 3 of the SYN/CFR review, `.review/spec-reviewer-2026-08-22-03.md`)
- Where: docs/spec/management-fee-deduction.md (FEE-074, FEE-077)
- Context: branch `feat/multi-device-sync` @ `a127abe`
- Severity: 🔵
- Observation: FEE-077 names an error variant and a method, FEE-074 names a typed field (`fee_rate_percent_micros: Option<i64>`) — contract vocabulary inside a spec ("what & why, never how"). FEE-074 also sits after FEE-078, out of numeric order. Pre-existing; only the missing scope tags on FEE-075–078 were fixed as boyscout while the file was touched for FEE-043/048. Rewording to behaviour-only text needs a judgement pass, not a mechanical edit — do it in a docs-only PR.

## 2026-07-25 — Linux bundle carries Tauri template leftovers

- Found by: main-agent
- Where: src-tauri/Cargo.toml, src-tauri/tauri.conf.json
- Context: branch `main` @ `09d622c`
- Severity: 🔵
- Observation: The locally-built `.deb` installs the app binary under the template name `tauri-app` (not `vault-compass`) and also packages `generate_bindings`, a dev-only helper binary, into the installer. The Windows NSIS pipeline is likely affected the same way (binary name inside the installer). Menu entries and app labels are correct; only the on-disk binary names and the extra packaged binary are off.

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
  1. ~~**`src/lib/update/` is a feature, mislocated.**~~ Resolved 2026-07-11 (v0.36.0 batch): the banner UI + hook moved to `src/features/update/`; the updater command adapter stayed a platform adapter at `src/lib/updateGateway.ts` (reviewer-arch: a feature-owned gateway would force `about_page` into a cross-feature import — the refined cut is UI = feature, command adapter = lib).

  2. **`features/account_details/{buy,sell}_transaction/` cross-imports from `features/transactions/`.** Today the imports are `RecordPriceCheckbox` (component), `TransactionFormData` (type), `validateTransactionForm` / `validateSellForm` (pure functions), and `useTransactions` (hook with state). Per the F23 reframing in kit proposal #21, the first three (primitives) become fine; the fourth (behavior coupling via a hook) remains a code smell. Either `account_details` owns its own thin wrapper around the gateway calls it needs, or the two features consolidate. Worth deciding _with_ the consolidation question (delta #3) rather than fixing the hook coupling alone.

  3. **`account_details` sub-feature bloat (8 sub-features).** Half of them — `buy_transaction`, `sell_transaction`, `deposit_transaction`, `withdrawal_transaction` — are conceptually transaction-recording flows and overlap with the `transactions/` feature. Two reasonable shapes: (a) consolidate the four into `transactions/` and let `account_details` stay focused on the holdings view, or (b) formalize the split — `account_details` owns "modals invoked from the holding row," `transactions/` owns "the transaction list page and its CRUD." Pick (b) as the lighter move; (a) is a bigger refactor.

  4. **`src/lib/*Storage.ts` adapters belong in `src/infra/settings/`.** The browser-`localStorage` UI-preference adapters (`autoFetchStorage.ts`, `autoRecordPriceStorage.ts`, `lastOperationDateStorage.ts`, `closedSectionStorage.ts`) are platform adapters per F28's Store-kinds table and should move to `src/infra/settings/`. New ones keep landing in `src/lib/` to stay consistent with their siblings (a partial move would orphan one file mid-migration). Mechanical folder move + import-path update; fold into the same `lib/ → infra/` rename PR.

  Migration is mechanical for #1/#4 (folder move + import sites) and conventional for #2/#3 (depends on the consolidation decision). Cleanest as one or two dedicated PRs after the kit proposals land (so the project mirrors the kit-ratified spec).

## 2026-05-09 — Migrate to gold DDD layout (per kit proposals #17–#19)

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
