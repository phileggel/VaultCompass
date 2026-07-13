# Implementation Plan — Scheduled Price Fetch (SPF)

Spec: `docs/spec/scheduled-price-fetch.md` (34 rules, SPF-010…061) · Contract: `docs/contracts/scheduled-fetch-contract.md` (`configure_scheduled_fetch`, `get_scheduled_fetch_status`)

Platform decision (user, 2026-07-12): all three adapters ship — **Linux (systemd user timer) fully verified by unit + E2E tests; Windows (schtasks) and macOS (launchd) ship with unit-level verification of the generated definitions only** (no machine to verify live registration). SPF-017.

---

## 1. Workflow TaskList

**Setup**

- [ ] 📖 Read spec: `docs/spec/scheduled-price-fetch.md`
- [ ] 📖 Read contract: `docs/contracts/scheduled-fetch-contract.md`
- [ ] 📖 Read constraining ADRs: `docs/adr/001` (i64 micros), `docs/adr/004` (use cases inject services), `docs/adr/009` (FX provider chain), `docs/adr/012` (latest-write-wins), `docs/adr/014` (refresh-lock scope exclusion), `docs/adr/017` (Yahoo keyless)
- [ ] 📖 Read conventions: `ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/backend-patterns.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/frontend-visual-proof.md`, `docs/test_convention.md`

**Backend phase (PR 1)**

- [ ] 🗄️ Database Migration `202607120001_create_scheduled_fetch.sql` (`just db-migrate` + `just prepare-sqlx`)
- [ ] ✍️ Backend test stubs (`test-writer-backend` from `docs/contracts/scheduled-fetch-contract.md` — red confirmed)
- [ ] 🏗️ Backend Implementation (minimal — make failing tests pass; no anticipation of future rules)
- [ ] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` + `reviewer-sql` + `reviewer-security` (new Tauri commands + process-spawning scheduler adapters ship in this PR) in parallel → `/review-triage` → apply Follow-ups)
- [ ] 🔗 `just generate-types`
- [ ] 🔧 `npx tsc --noEmit` → fix TS errors from new bindings only
- [ ] 🧹 `just format`
- [ ] 💾 Commit via `/smart-commit` — suggested: `feat: daily price download runs on a schedule, even with the app closed`
- [ ] 🔀 `/create-pr` — PR 1 (backend). After merge, branch PR 2 off updated `main`.

**Frontend phase (PR 2)**

- [ ] ✍️ Frontend test stubs (`test-writer-frontend` — red confirmed; `modified_functions: [SettingsPage.tsx:SettingsPage]`)
- [ ] 💻 Frontend Implementation (minimal — make failing tests pass; no defensive code, no anticipation of future rules)
- [ ] 📸 `/visual-proof` — ScheduledFetchSection all states, light + dark
- [ ] 🔍 Frontend Review (`reviewer-frontend` → `/review-triage` → apply Follow-ups)
- [ ] 🧹 `just format`
- [ ] 💾 Commit via `/smart-commit` — suggested: `feat: set up the daily download from the app settings`
- [ ] _(no `/create-pr` here — PR 2 continues with closure below)_

**Closure (PR 2, continued)**

- [ ] ✍️ E2E scenarios (`test-writer-e2e` — settings section flow; E2E runs use the no-op scheduler, see § E2E notes)
- [ ] ▶️ `npm run test:e2e` green (17 existing + new spec)
- [ ] 🔍 Cross-cutting Review (`reviewer-e2e` + `reviewer-infra` (justfile/scripts touched if any) + `reviewer-security` (new Tauri commands + process-spawning scheduler adapters — mandatory here) → `/review-triage`)
- [ ] 📚 Documentation Update — `docs/todo.md` (close "(infra) — Scheduled daily automatic price download"), `ARCHITECTURE.md` (new `use_cases/scheduled_fetch/`, `shared/infrastructure/scheduler/`, headless entry), `docs/ddd-divergences.md` (use-case-owned persistence), `docs/ubiquitous-language.md` (ScheduledFetchRun, trigger time)
- [ ] ✅ `spec-checker` [HARD GATE]
- [ ] 🧹 `just format`
- [ ] 💾 Commit via `/smart-commit` — suggested: `test: cover the daily download settings end to end` (or `docs:`/`chore:` split as needed)
- [ ] 🔀 `/create-pr` — PR 2 (frontend + E2E + closure)

---

## 2. Detailed Implementation Plan

### Migrations

**`src-tauri/migrations/202607120001_create_scheduled_fetch.sql`** _(tables owned by `use_cases/scheduled_fetch/` — see spec Context divergence)_

- `scheduled_fetch_configuration` — singleton: `id INTEGER PRIMARY KEY CHECK (id = 1)`, `enabled INTEGER NOT NULL DEFAULT 0`, `trigger_time TEXT NOT NULL DEFAULT '22:15'` (SPF-011, SPF-018). Seed the single row in the migration so reads never face an empty table.
- `scheduled_fetch_runs` — `id TEXT PRIMARY KEY`, `executed_at TEXT NOT NULL`, `trigger_date TEXT NOT NULL`, `outcome TEXT NOT NULL` (`Succeeded` | `Failed` | `SkippedAlreadyRun`), `updated_count INTEGER NOT NULL`, `skipped_count INTEGER NOT NULL` (SPF-050). Index on `trigger_date` (guard lookup SPF-021).
- Run `just db-migrate` then `just prepare-sqlx` before any backend code.

### Backend

**New use case — `src-tauri/src/use_cases/scheduled_fetch/`** (mirrors `asset_price_fetch/`: `mod.rs`, `orchestrator.rs`, `api.rs`, `error.rs`, plus `repository.rs`, `headless.rs`)

- `repository.rs` — `ScheduledFetchRepository` (sqlx): `get_configuration()`, `save_configuration(enabled, trigger_time)`, `last_run()`, `last_successful_run()`, `record_run(run)`. Use-case-owned persistence (spec Context; record divergence in `docs/ddd-divergences.md` at closure). Factory methods per CLAUDE.md (`from_storage`, `new`) on `ScheduledFetchRun`.
- `orchestrator.rs` — `ScheduledFetchOrchestrator { asset_service, currency_service, repository, scheduler }` (ADR-004: services, not repositories, for the BC calls):
  - `configure(enabled, trigger_time)` — validate time (SPF-019) → OS schedule register/re-register/remove **first**, persist **after** OS success (SPF-013 consistency) (SPF-011, SPF-012)
  - `status()` — configuration + `last_run` (SPF-052)
  - `run_scheduled_fetch()` — latest-pending-trigger resolution + once-per-day guard (SPF-021, SPF-022); scope via asset service (SPF-040 = MKT-110/116/151 exclusions); per-asset daily close series → upserts dated per trading day (SPF-030–034, MKT-025/102/125); FX dated rates for all persisted pairs (SPF-035–039, FXR-071); per-asset/per-pair silent skip counted (SPF-041, SPF-038); empty scope = quiet success (SPF-042); 3-attempt provider retry with increasing delay (SPF-051); record run (SPF-050, SPF-039)
  - `self_heal()` — verify/repair or remove the OS entry per config, called from `lib.rs` setup (SPF-015)
- `api.rs` — `configure_scheduled_fetch`, `get_scheduled_fetch_status` Tauri commands returning typed `ScheduledFetchError` per `docs/error-model.md` (`InvalidTriggerTime`, `ScheduleRegistrationFailed`, `ScheduleRemovalFailed`, `DatabaseError`)
- `headless.rs` — entry for the OS-triggered run: resolve data dir (below), open pool, wire the minimal service graph, `run_scheduled_fetch()`, exit code 0 unless the run record itself could not be written

**Scheduler adapter — `src-tauri/src/shared/infrastructure/scheduler/`** (`mod.rs`, `systemd.rs`, `windows_task.rs`, `launchd.rs`)

- `mod.rs` — `DailyFetchScheduler` trait: `register(trigger_time)`, `remove()`, `is_registered()`; `platform_scheduler()` factory (`cfg(target_os)`); a `NoopScheduler` used when `VAULT_COMPASS_E2E_DATA_DIR` is set (debug builds only — same gate as the existing E2E data-dir override in `lib.rs:322`)
- `systemd.rs` — writes `~/.config/systemd/user/vaultcompass-fetch.{service,timer}` (`OnCalendar=*-*-* HH:MM`, `Persistent=true` for SPF-022) + `systemctl --user daemon-reload && enable --now`; **fully unit-tested on generated file content; live-verified on Linux**
- `windows_task.rs` — `schtasks /Create /SC DAILY /ST HH:MM /F` with `StartWhenAvailable` via XML definition; unit tests on the generated arguments/XML only (SPF-017)
- `launchd.rs` — `~/Library/LaunchAgents/com.vaultcompass.fetch.plist` (`StartCalendarInterval`) + `launchctl bootstrap`; unit tests on generated plist only (SPF-017)
- All adapters register the **current executable path** with the `--scheduled-fetch` argument; `self_heal()` re-registers when the recorded path differs (SPF-015)

**Headless entry**

- `src-tauri/src/main.rs` — before `vault_compass_lib::run()`: if args contain `--scheduled-fetch`, call `vault_compass_lib::run_scheduled_fetch_headless()` and exit (SPF-016, SPF-020)
- `src-tauri/src/lib.rs` — new `pub fn run_scheduled_fetch_headless()`; new `resolve_app_local_data_dir()` helper mirroring Tauri's `app_local_data_dir()` (platform data-local dir + bundle identifier from `tauri.conf.json`) so both entries open the same database — unit test asserting the identifier matches `tauri.conf.json`; app-start `self_heal()` call in the existing setup closure (SPF-015); SQLite WAL + `busy_timeout` pragmas confirmed/added in `core/db.rs` for cross-process safety (SPF-023)

**Bounded-context extensions**

- `src-tauri/src/context/asset/repository/yahoo_client.rs` — `fetch_daily_closes(symbol, from, to) -> Result<Vec<DatedQuote>>` via `/v8/chart/?period1=…&period2=…&interval=1d`, parsing `timestamp[]` × `indicators.quote[0].close[]`, carrying the meta currency for MKT-125 normalization; `Ok(empty)` on `chart.error` (skip semantics)
- `src-tauri/src/context/asset/` service — dated-close upsert path (delegates to the existing `AssetPrice` upsert with `source = YahooFinance`, MKT-102) — **no event publication from the headless path** (SPF-024)
- **Scope extraction (SPF-040)**: the MKT-116/MKT-151 exclusions live in `use_cases/asset_price_fetch/orchestrator.rs::build_scope` (~137–162), not in a service query. Extract `build_scope` to `use_cases/shared/` (precedent: the B18 valuation engine lives there) and consume it from both `asset_price_fetch` and `scheduled_fetch` — no rule-logic duplication; the extraction is mechanical and bounded to the two call sites
- `src-tauri/src/context/currency/infrastructure/frankfurter_client.rs` — date-range fetch (`/v1/{from}..{to}?base=…` — verified live 2026-07-12); `ecb_client.rs` untouched (its 90-day daily feed already covers a 30-day window if the chain falls back); `application/service.rs` — dated-rate upsert for all persisted pairs (FXR-071 scope)
- `src-tauri/src/core/specta_builder.rs` — register both new commands (THE registry, nowhere else)

### Frontend

- `src/features/settings/gateway.ts` — **new** (feature's first IPC): `configureScheduledFetch(enabled, triggerTime)`, `getScheduledFetchStatus()` — positional args matching `bindings.ts` exactly
- `src/features/settings/scheduled_fetch/ScheduledFetchSection.tsx` — toggle + time field (native time input constraining SPF-019, default 22:15 SPF-018, stable ids per F25: `scheduled-fetch-toggle`, `scheduled-fetch-time`, `scheduled-fetch-status`) + status line (SPF-052) + loading/in-flight/error states (SPF-060, SPF-061, SPF-013)
- `src/features/settings/scheduled_fetch/useScheduledFetchSection.ts` + colocated `.test.ts` — status load, configure call, optimistic-revert on rejection (SPF-013)
- `src/features/settings/shared/presenter.ts` — **new** `shared/` for the feature: error `code` → i18n key mapping (F27) for the four error codes
- `src/features/settings/SettingsPage.tsx` — mount the section (`modified_functions` entry)
- `src/i18n/locales/{fr,en}/common.json` — section title, labels, status-line formats, error messages (fr default + en, per `docs/i18n-rules.md`)

### E2E notes

- New spec `e2e/scheduled_fetch/` — enable → time defaults to 22:15 → status "No download yet"; change time; disable. Stable-id selectors only (E1/E4).
- E2E launches use `VAULT_COMPASS_E2E_DATA_DIR` (debug build) → `NoopScheduler` is active, so specs exercise the full FE ↔ BE ↔ SQLite stack without touching the CI host's systemd. **Live systemd registration is verified manually on the dev machine** (checklist item in the PR 2 description), not by E2E.
- The headless run pipeline is covered by Rust integration tests (`src-tauri/tests/scheduled_fetch_crud.rs` mirroring `free_shares_crud.rs` — also pre-empts the SPL/HNO tests-parity techdebt pattern), not by E2E.

### Rules Coverage

| Rule    | Layer              | Task                                                        | Notes                                                  |
| ------- | ------------------ | ----------------------------------------------------------- | ------------------------------------------------------ |
| SPF-010 | frontend           | `ScheduledFetchSection.tsx` toggle + time field             | `[unit-test-needed]` (`SettingsPage.tsx:SettingsPage`) |
| SPF-011 | backend            | `repository.rs` configuration persistence                   | singleton row                                          |
| SPF-012 | frontend + backend | `orchestrator.configure` + scheduler adapters               | OS first, persist after                                |
| SPF-013 | frontend + backend | typed errors + `useScheduledFetchSection` revert            | F27 presenter                                          |
| SPF-014 | backend            | `OnCalendar`/`/ST`/`StartCalendarInterval` local wall-clock | OS owns DST                                            |
| SPF-015 | backend            | `orchestrator.self_heal()` from `lib.rs` setup              | exe-path drift                                         |
| SPF-016 | backend            | `main.rs` headless branch exits without window              |                                                        |
| SPF-017 | backend            | three adapters; unit-only verification for win/mac          | user decision                                          |
| SPF-018 | frontend           | 22:15 default in section state                              |                                                        |
| SPF-019 | frontend + backend | time validation both sides                                  | `InvalidTriggerTime`                                   |
| SPF-020 | backend            | `--scheduled-fetch` arg → `run_scheduled_fetch_headless()`  | no Tauri builder                                       |
| SPF-021 | backend            | latest-pending-trigger + guard in orchestrator              | index on `trigger_date`                                |
| SPF-022 | backend            | `Persistent=true` / `StartWhenAvailable` / launchd wake     | + SPF-021 settle rule                                  |
| SPF-023 | backend            | WAL + busy_timeout in `core/db.rs`; no MKT-113 coupling     | cross-process safety                                   |
| SPF-024 | frontend           | no event wiring — nothing to build; asserted in E2E         |                                                        |
| SPF-030 | backend            | close-series dating in orchestrator                         |                                                        |
| SPF-031 | backend            | `fetch_daily_closes` window since last success, cap 30d     | ADR-017                                                |
| SPF-032 | backend            | absent series days → no rows                                |                                                        |
| SPF-033 | backend            | series naturally omits incomplete day                       | self-corrects                                          |
| SPF-034 | backend            | delegate to existing upsert (MKT-025/102/125)               | ADR-012                                                |
| SPF-035 | backend            | `frankfurter_client` date-range, all persisted pairs        | FXR-071 scope                                          |
| SPF-036 | backend            | FX backfill window mirror                                   | verified live                                          |
| SPF-037 | backend            | absent rate days → no rows                                  |                                                        |
| SPF-038 | backend            | per-pair skip counted                                       |                                                        |
| SPF-039 | backend            | independent price/rate outcome in run record                |                                                        |
| SPF-040 | backend            | reuse asset_price_fetch scope query                         | MKT-116/151                                            |
| SPF-041 | backend            | per-asset skip counted                                      | MKT-114 set                                            |
| SPF-042 | backend            | empty scope → Succeeded(0,0)                                |                                                        |
| SPF-050 | backend            | `record_run` on every path                                  |                                                        |
| SPF-051 | backend            | 3-attempt retry w/ increasing delay                         |                                                        |
| SPF-052 | frontend           | status line formatting in presenter                         |                                                        |
| SPF-053 | backend            | no code — property of SPF-031; integration test             |                                                        |
| SPF-060 | frontend           | in-flight disable in section                                | `[unit-test-needed]` via hook test                     |
| SPF-061 | frontend           | loading/error states in section                             |                                                        |

---

## 3. PR Plan

- **Strategy**: 2 PRs (user-selected)
- **Estimate**: BE ~16 files / ~1000 LOC; FE ~8 files / ~400 LOC; E2E ~2 files
- **PR 1** — `feat: daily price download runs on a schedule, even with the app closed`
  - Scope: spec + contract + migration + all backend (use case, scheduler adapters, headless entry, BC extensions, specta registration) + regenerated `bindings.ts` (unused by FE yet, no runtime impact). Terminates at the Backend-phase `/create-pr` checkpoint.
  - Dependency: none. Branch: `feat/scheduled-price-fetch` (current).
- **PR 2** — `feat: set up the daily download from the app settings`
  - Scope: frontend (gateway, section, presenter, i18n) + E2E + docs closure + spec-checker. Terminates at the Closure `/create-pr` checkpoint.
  - Dependency: rebase off `main` after PR 1 merges. Branch: `feat/scheduled-price-fetch-fe`.
