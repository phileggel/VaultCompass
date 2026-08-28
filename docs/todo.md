# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->
<!-- Ordered by user value: entries that change what the user experiences first, -->
<!-- entries with no direct user value after the separator. -->

## (backend) — An empty sync folder is read as a portfolio reset, not as an unavailable volume

`FsFolderStore::check_available()` verifies only that the path exists, is a directory, and is readable — it never checks that `vaultcompass-sync.json` is present. So a folder that exists but is empty passes the availability gate, `read_header_bytes()` returns `None`, `header_gate(None, _)` returns `HeaderGate::Reset`, and `SyncRun::pause_for_reset()` pauses the device and reports `PortfolioReset` — "the portfolio was reset elsewhere; this device must rejoin from a fresh installation". Nothing is lost (no publish happens under the old key, and the local database is untouched) and plugging the volume back in then pressing Resume recovers it, but the message tells the user to rejoin from scratch, which would destroy their local data if followed literally.

Removable media is where an empty-but-present folder actually occurs, and the two platforms fail differently:

- **Linux, desktop auto-mount** — `/media/<user>/<LABEL>` is removed by udisks on unmount, so the path 404s: `ErrorKind::NotFound` → `FolderProblem::Missing` → correctly reported as unavailable. A yank without unmounting gives I/O errors → `IoFailure` → also correct. Safe by accident of how udisks cleans up.
- **Linux, hand-made mount point** — an `fstab` entry or a manually created `/mnt/...` directory persists as an empty writable folder when nothing is mounted. Hits the false reset.
- **Windows, bare drive letter (`E:\VaultCompass`)** — with no media in the drive the OS returns `ERROR_NOT_READY`, which Rust does not map to a named `ErrorKind`, so `classify()`'s catch-all gives `IoFailure` → correctly unavailable. If the letter exists but the folder does not, `NotFound` → `Missing`. Both safe.
- **Windows, volume mounted into an empty NTFS folder** (`C:\Mounts\Key`) — the folder remains, empty, when the volume is detached. Same false reset as the Linux fstab case, and more likely on Windows because mounting into a folder is an offered option in Disk Management.
- **Windows, drive-letter reuse** — removable letters are reassigned by insertion order, so a different stick can take `E:`. If that volume happens to carry a `VaultCompass` folder, the header decodes but its passphrase check fails, which also lands on `HeaderGate::Reset` rather than a "this is a different portfolio" message. `FolderHoldsOtherPortfolio` already exists as an error for the enable path and is the honest classification here.

Proposal: distinguish "we previously had a header here and now the folder is empty" from a genuine remote reset. A device that has already joined a portfolio knows the folder should carry a header; finding none is far more likely to be a detached volume than a start-over. Options: have `check_available()` require the header for an already-enrolled device (report `Unmounted`, the variant that exists and is currently never produced), or gate `pause_for_reset()` on having actually read a header whose check failed. Pair it with guidance in the reset message — try Resume with the volume attached before rejoining.

Also worth surfacing in the UI: recommend the auto-mount path on Linux and a bare drive letter on Windows over a persistent mount-point folder, since those degrade correctly.

**User value:** A detached USB key or drive no longer tells the user their portfolio was reset and that they must reinstall.
**Done when:** An enrolled device finding no header in an existing folder reports it unavailable, a different portfolio's folder reports `FolderHoldsOtherPortfolio`, and both are covered by tests.

## (frontend) — Give multi-device sync its own view and rework its UI

Sync ships as one `SyncSection` inside the settings page (`src/features/settings/sync/`, ~12 KB of TSX). That section now carries the whole feature: the status block (enabled/paused, device name, folder, last sync), the roster of other computers, held-back counts, failures, conflict notices, inconsistent holdings, six actions (Sync now, Pause, Rename, Change folder, Leave, Start over), two modals, and a single-field prompt shared between rename and change-folder. It has outgrown a settings section.

Observed friction (2026-08-28, first real two-computer setup):

- The six actions render as one flat row of buttons with no grouping by consequence — "Sync now" sits beside "Start over", which discards every published file.
- The shared rename/change-folder prompt forced a conditional Browse button, since only one of the two takes a path. Splitting them into purpose-built dialogs removes the conditional.
- `InstallationHoldsUserData` states that joining requires a fresh installation but gives no route forward; the user has to be told out-of-band which directory to clear. The error should explain the remedy, and ideally offer it.
- Status is a bare `<dl>` — no sense of health at a glance, and nothing shows whether the other computer has published yet, which is the first thing a user checks when a join fails.

Proposal: promote sync to its own route (`/sync`, following the `/performance` precedent in `src/router.tsx`), leaving settings with a link and, at most, the enabled/paused summary. Group the actions by consequence (routine / device / destructive), split the shared prompt, and give the status block a health-oriented layout.

Carry-over risk: `e2e/sync/sync.test.ts` selects on `sync-*` stable ids throughout (`#sync-now`, `#sync-pause`, `#sync-leave`, `#sync-change-folder`, `#sync-enable-*`, `#sync-indicator`). Moving or regrouping those elements breaks the suite silently — `just check` does not type-check `e2e/`. Rewrite the specs in the same PR.

**User value:** Sync health is readable at a glance, destructive actions sit apart from routine ones, and a refused join states how to fix it.
**Done when:** Sync lives at `/sync` with a link from settings, actions are grouped by consequence, rename and change-folder have separate dialogs, `InstallationHoldsUserData` names the remedy, and `e2e/sync/sync.test.ts` passes on the new ids.

## (fullstack) — Monitored assets, price bars, and indicator primitives

Prerequisite work for the private advice module — design in [`advice-module-design.md`](advice-module-design.md) (draft, hook not yet ratified). Two public-side steps, both useful on their own: (1) a `monitored` asset flag plus an `asset_daily_bars` table (OHLCV, separate from `asset_prices` so the latest-write-wins price semantics stay untouched), fetched as one ranged request per monitored asset at the minimum window the enabled indicators need — one year of daily bars covers every requirement including SMA(200), and its month-end closes feed the monthly algorithms without a second call (25 KB / 256 bars measured); afterwards only the missing tail is topped up by the scheduled fetch. (2) Indicator primitives (SMA/EMA, MACD, ATR, RSI, Bollinger, Donchian, monthly closes, drawdown) as pure tested functions plus an indicator panel — readings only, no verdicts. Verdicts and levels stay in the private module. Route through /spec-writer when scheduled; the doc's open questions (target weights for 5/25 drift, SMA(200) inclusion) should be closed first.

**User value:** The user reads technical indicators (SMA, EMA, MACD, ATR, RSI, Bollinger, Donchian, drawdown) for the assets they mark as monitored.
**Done when:** The `monitored` flag and `asset_daily_bars` ship with the ranged fetch, the indicator functions are unit-tested, and the panel renders readings for a monitored asset.

## (fullstack) — Explain suppressed lifetime performance metrics instead of a bare "—"

When the since-inception % and annualized-yield columns are suppressed by the Dietz guard (denominator ≤ 0), the performance view shows "—" with no cause, which reads as a bug. Real-world trigger (CTO account, 2026-07-27): opening balances typed with unit price 0 (employee free shares) plus early withdrawals make the lifetime denominator negative forever, while the windowed Perf % column computes fine — the user cannot tell the data is fixable. Proposal: the response carries a degradation reason for suppressed lifetime metrics (e.g. zero-valued opening balance vs. genuinely undefined), and the view surfaces a persistent contextual hint (info icon/tooltip on the suppressed cells — not a snackbar, which is transient and re-fires) telling the user which transaction to correct. Needs a PRF spec rule + contract field + both layers; route through /spec-writer when scheduled. Companion guardrail at the entry side (user decision 2026-07-27: warn, don't block — a truly worthless position is legitimate): when an opening-balance form is submitted with Total Cost 0, show an inline warning that zero declares no starting capital and suppresses lifetime performance, suggesting the entry-date market value instead.

**User value:** When lifetime performance shows “—”, the user learns why and which transaction to correct.
**Done when:** The response carries a degradation reason, suppressed cells show a persistent hint naming the cause, and an opening balance submitted with Total Cost 0 warns inline.

## (frontend) — Merge TXL per-asset page into the account journal (deferred)

The per-asset transaction page (`transaction_list/TransactionListPage.tsx`, route `/accounts/$accountId/transactions/$assetId`, the holdings-row loupe target) predates the account journal and is now a strict subset of it — both already share `TransactionTable`, `EditTransactionModal`, delete flow, and `routeEditTransaction`. Consolidate: the loupe navigates to the journal with the asset filter prepopulated (`/accounts/$accountId/journal?asset=<assetId>`); delete the TXL page/hook/route. Decided 2026-07-06: cash-statement columns (Cash out / Cash in / Balance) render only in the unfiltered (global) journal view; with an asset filter active the table shows plain Total Amount — a running balance over a filtered subset is misleading.

Must carry over before deleting TXL: (1) add-transaction CTA + `AddTransactionModal` with prefill from the active filter; (2) the `pendingTransactionAssetId` deep-link round-trip — re-target its senders (`HoldingRow`, `ClosedHoldingRow`, `AssetManager` `returnPath` create-asset flow) to the journal route; (3) fold TXL-0xx spec rules into the journal spec. TXL's in-place account switcher is intentionally dropped. E2E: the suite uses `txl-*` stable ids throughout — rewrite those specs in the same PR (selector-removal trap).

**User value:** One transaction view instead of two near-identical ones; the holdings loupe opens the journal filtered to that asset.
**Done when:** The loupe navigates to `/accounts/$accountId/journal?asset=…`, the TXL page/hook/route are deleted, add-transaction prefill and the `pendingTransactionAssetId` deep link work from the journal, and the `txl-*` specs are rewritten.

---

<!-- Below: no direct user value — test infrastructure, conventions, dependency currency. -->

## (e2e) — Drive a second device in the E2E suite

The multi-device sync E2E covers the single-device critical path only (plan § Halt Artifact H1): `wdio.conf.ts` launches one binary with one `VAULT_COMPASS_E2E_DATA_DIR` and `maxInstances: 1`, so joining a folder another device created (SYN-014/036) is proven by the two-database integration test `src-tauri/tests/sync_two_devices.rs`, not through the UI. A real two-device E2E needs an `e2e/helpers/second_device.ts` that launches a second binary against its own data directory plus a wdio multi-remote configuration — a separate, pre-requisite task before any join scenario is written.

**User value:** None directly — test infrastructure.
**Done when:** A wdio multi-remote config and `e2e/helpers/second_device.ts` launch a second binary on its own data directory, and a join scenario (SYN-014/036) passes through the UI.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

**User value:** None — dependency currency.
**Done when:** `tauri-specta` releases without the `specta =2.0.0-rc.22` pin and the project builds on `specta` rc.23 + `specta-typescript` 0.0.10.

## (deps) — Accepted risk: WebdriverIO 9 transitive advisories (deepmerge-ts, extract-zip, expect-webdriverio)

`npm audit` flags 13 high advisories rooted in `deepmerge-ts < 8` (stack exhaustion on recursive graphs), `extract-zip` (symlink traversal, no fixed version published) and `expect-webdriverio`, all reached only through `@wdio/*` 9.31.2 — the latest release still pins them, and npm's only "fix" is a downgrade to WebdriverIO 7. The packages are E2E test tooling in `devDependencies`: nothing from them enters the application bundle or the Tauri binary. Re-run `npm audit` at each release and drop this entry once WebdriverIO picks up `deepmerge-ts` 8 and a patched `extract-zip`.

**User value:** None — devDependency advisories; nothing from them enters the shipped bundle.
**Done when:** WebdriverIO ships `deepmerge-ts` 8 and a patched `extract-zip`, `npm audit` is clean, and this entry is deleted.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.

**User value:** None — the vulnerable RSA path is unreachable in a SQLite-only build.
**Done when:** sqlx stops compiling `sqlx-mysql` for sqlite-only builds or `rsa` publishes a fix, `cargo audit` is clean, and this entry is deleted.
