# Plan — `next` batch, September 2026

> Source: `tmp/whats-next-2026-09-12-01.md`. Small todo and techdebt items only — no new feature, no spec of its own. One branch (`chore/next-2026-09`), **one commit per task**, Workflow B per task (reviewers for the touched layer → `/review-triage` → `/smart-commit`), `just merge` at the end, then `just release` once the main-push E2E is green.

## Tasks — in execution order

### T1 — A detached sync drive no longer reports the portfolio as reset

Source: `docs/todo.md` § "(backend) — An empty sync folder is read as a portfolio reset". The one user-facing item in the batch.

Today `header_gate(None, _)` returns `HeaderGate::Reset`, so an enrolled device whose folder is reachable but empty — a persistent mount point with nothing mounted — pauses itself and tells the user to rejoin from a fresh installation. `FolderProblem::Unmounted` exists and is never produced.

- **Spec** `docs/spec/multi-device-sync.md`: SYN-084 narrows to "a header whose passphrase check no longer matches the kept key"; SYN-069 gains "a mount point present but empty — the folder carries no header the device follows — is reported as the volume not being mounted". The drive-letter-reuse case (a _different_ portfolio's header) stays as it is: by content it is indistinguishable from a reset, so it is recorded as out of scope, not solved.
- **Backend** `src-tauri/src/context/sync/application/run.rs`: `HeaderGate` gains `Absent`; the run's gate call site maps it to a `FolderUnavailable { problem: Unmounted }` failure without pausing. `kept_key == None` stays a reset (the device can open nothing). Check the resume path (SYN-073) takes the same branch.
- **Frontend** `src/i18n/locales/{en,fr}/common.json`: the `Unmounted` problem text says to reconnect the volume and press Resume / Sync now. Copy only — no logic.
- **Tests**: `missing_header_reports_portfolio_reset` (run.rs:909) inverts to "reports the folder unavailable and does not pause"; `passphrase_mismatch_reports_portfolio_reset_and_pauses_the_device` stays. Grep `e2e/sync/` for a reset scenario that relied on the old behaviour.
- **Docs**: close the todo entry.
- Commit: `fix: a detached sync drive no longer reports the portfolio as reset`

### T2 — Applied holding notes and currency pairs raise their own events

Source: `docs/techdebt.md` 2026-08-23. `apply_holding_note` and `apply_currency_pair` raise nothing; the frontend compensates by re-fetching account details and currency rates on the bare `SyncCompleted` marker.

- **Backend** `src-tauri/src/core/event_bus/event.rs`: `HoldingNoteUpdated`, `CurrencyPairUpdated`. Raised by the local writes (`upsert_holding_note`, `delete_holding_note`, `declare_currency_pair`) and by the two apply paths alike (SYN-064).
- **Frontend** `useAccountDetails.ts`, `useCurrencyRatesView.ts`: subscribe to the new events; drop `SyncCompleted` from both.
- `just generate-types`; tests on both layers; techdebt entry deleted.
- Commit: `refactor: applied notes and currency pairs announce themselves like local writes`

### T3 — One definition of whether the unpriced-prices modal is open

Source: `docs/techdebt.md` 2026-09-11. `UnpricedPricesModalMount` and `AccountManager` each derive it from `unpricedAssets`.

- `src/lib/store.ts`: export a `selectUnpricedModalOpen` selector; both readers use it. Tests adjust the mock. Techdebt entry deleted.
- Commit: `refactor: one definition of whether the unpriced-prices modal is open`

### T4 — The price-fetch use case receives its currency service directly

Source: `docs/techdebt.md` 2026-09-11. `AssetPriceFetchUseCase::new` reaches through `Dispatcher::currency_service()`; the composition root already holds the `Arc`.

- `orchestrator.rs`: new parameter; `dispatcher.rs`: accessor deleted; 14 `AssetPriceFetchUseCase::new` call sites (`lib.rs` + tests) pass the service. Mechanical. Techdebt entry deleted.
- Commit: `refactor: hand the price fetch use case its currency service directly`

### T5 — Close the two deferred PMV coverage items

Source: `docs/techdebt.md` 2026-09-11.

- `usePriceMovementReport.test.ts`: a gateway fake that honours `dispose()` — mount → unmount → fire the event → remount → `report === null` (PMV-016).
- `/visual-proof` for the **Undated** state (both dates absent, `pmv.column_values`), light + dark, into `screenshots/`. Techdebt entry deleted.
- Commit: `test: prove the price movement report is discarded across a remount`

### T6 — Bump specta, tauri-specta and specta-typescript in lockstep

Source: `docs/todo.md` § "(deps) — Update specta to rc.23" (targets stale). `tauri-specta` rc.25 pins `specta ==2.0.0-rc.25` and `specta-typescript ^0.0.12`, so the three move together: rc.22 → rc.25, rc.21 → rc.25, 0.0.9 → 0.0.12.

- `src-tauri/Cargo.toml`, `cargo update -p …`, `just generate-types`. **Gate**: the `src/bindings.ts` diff must be cosmetic (ordering, comments, formatting). Any type or signature change → revert the task, leave the todo entry with the finding. Todo entry closed on success.
- Commit: `chore: bump specta and tauri-specta to rc.25`

### T7 — Docs hygiene

One commit, all doc-only:

- `docs/spec/management-fee-deduction.md`: FEE-074 and FEE-077 reworded to behaviour-only (no `fee_rate_percent_micros`, `HoldingDetail`, `AccountError::…`, `Account::ensure_…`); the FEE-074 paragraph moved between FEE-073 and FEE-075. **IDs are permanent — no renumbering.** Techdebt entry deleted.
- `docs/spec-index.md`: SYN, CFR, PMV `planning` → `active`.
- `docs/plan/{multi-device-sync,price-movement,scheduled-price-fetch}-plan.md`: the 106 remaining checklist boxes ticked — the features shipped in v0.38.0 / v0.40.0 / v0.37.0 — with a one-line "Shipped in vX.Y.Z" note at the top of each.
- `docs/todo.md` § "(deps) — Accepted risk: WebdriverIO 9": rewritten to the state after `ff986f1` (14 advisories, single `extract-zip` root via `@puppeteer/browsers`, `deepmerge-ts` resolved).
- `docs/ddd-divergences.md` § 13: test functions use descriptive names without a `test_` prefix (315 of 391 already do; `docs/test_convention.md` is kit-owned). Techdebt entry 2026-05-24 deleted.
- Commit: `docs: close shipped plans, fix FEE wording, codify test naming`

### T8 — Land and release

- `just merge` from the branch; watch the main-push E2E and Quality.
- `/dep-audit` (expect the 14 accepted dev-only advisories, nothing new), `reviewer-security` release sweep.
- `just release -y` (patch bump — the batch carries one `fix`), publish the draft once both platform jobs attach their assets, verify the live `latest.json`.

## Deferred — and why

| Item                                                                                                                                    | Reason                                                                                                                                                                                  |
| --------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Held-back changes / conflict notices unbounded                                                                                          | A cap or eviction contradicts SYN-066 ("persist until the user dismisses them individually") — needs a spec decision, not a small fix. Threat needs an untrusted peer; none is planned. |
| Linux bundle template leftovers (`tauri-app` binary rename)                                                                             | The Windows updater's upgrade path across a binary rename needs real verification on Windows.                                                                                           |
| Local writes do not take the sync gate                                                                                                  | 30 capture sites; its own PR.                                                                                                                                                           |
| Account-deletion cascade / ADR-006 unit of work                                                                                         | Requires the UoW design.                                                                                                                                                                |
| ADR "amends" vocabulary                                                                                                                 | The kit permits exactly three statuses; converting ADR-003/005 would lose the amendment link. Needs a kit change.                                                                       |
| Sync's own view · TXL merge · lifetime-metric explanation · monitored assets · second-device E2E · gold layout migrations · join memory | Major or multi-layer feature work — outside a small batch.                                                                                                                              |

## Sequencing notes

- T1 first: the only behaviour change, and the one that can fail E2E on main; everything after it is internal.
- T6 late: if the bindings diff is not cosmetic the task is reverted without disturbing the rest.
- Reviewers per task: T1 `reviewer-backend` + `reviewer-arch` (+ `reviewer-e2e` if a spec changes); T2 backend + arch + frontend; T3/T5 `reviewer-frontend` + arch; T4 backend + arch; T6 `reviewer-infra`; T7 none (docs).
