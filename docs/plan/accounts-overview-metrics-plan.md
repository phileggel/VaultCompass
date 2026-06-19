# Implementation Plan — Accounts-Overview Metrics (ACC-023, ACC-024)

> Scope: ONLY the new `account` rules **ACC-023** (per-account `total_unrealized_pnl`)
> and **ACC-024** (per-account YTD performance) on `AccountSummary`. All other ACC
> rules are shipped and out of scope.
> Spec: `docs/spec/account.md` · Contract: `docs/contracts/account-contract.md`
> No new command (extends `get_account_summaries` / `AccountSummary`). No DB migration.
> **Single commit for all phases** on `refactor/ux-improvements` (no per-layer commits/PRs).

Add two per-account columns to the Accounts overview table: account-wide unrealized
P&L and year-to-date performance. Both values are computed in the existing
`AccountSummaryUseCase` (recompute-on-read, ADR-013) and surfaced on `AccountSummary`,
which already feeds the table — so no new IPC call from the FE.

---

## 1. Workflow TaskList

**Setup**

- [ ] 📖 Read spec `docs/spec/account.md` (ACC-023, ACC-024) + the referenced MKT-040/FXR-040 (unrealized P&L) and PRF-034/PRF-032 (YTD Dietz) rules
- [ ] 📖 Read contract `docs/contracts/account-contract.md` (`AccountSummary` +2 fields)
- [ ] 📖 Read ADRs: `001` (i64 micros), `003` (cross-context orchestration), `004` (use cases inject services), `013` (recompute-on-read)
- [ ] 📖 Read conventions: `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/test_convention.md`

**Backend phase**

- [ ] ✍️ Backend test stubs (`test-writer-backend`) — red confirmed
- [ ] 🏗️ Backend implementation (minimal — make failing tests pass)
- [ ] 🔍 Backend review (`reviewer-backend` + `reviewer-arch` in parallel → `/review-triage`) — **arch must rule on the composition decision below**
- [ ] 🔗 `just generate-types` → `src/bindings.ts`
- [ ] 🔧 `npx tsc --noEmit` → fix TS errors from new bindings only
- [ ] 🧹 `just format`
- [ ] _(no commit — single-commit feature; continue to FE)_

**Frontend phase**

- [ ] ✍️ Frontend test stubs (`test-writer-frontend`; `modified_functions` below) — red confirmed
- [ ] 💻 Frontend implementation (minimal)
- [ ] 📸 `/visual-proof` on the accounts table (light + dark) — `.tsx` changed
- [ ] 🔍 Frontend review (`reviewer-frontend` → `/review-triage`)
- [ ] 🧹 `just format`

**Closure**

- [ ] ✍️ E2E — _optional / skip_: column addition on an existing screen; unit + integration cover it. Note the skip.
- [ ] 📚 Docs: `docs/todo.md` (no open entry); `ARCHITECTURE.md` — only if the composition introduces a new cross-use-case dependency pattern worth recording
- [ ] ✅ `spec-checker` on ACC-023/024 [HARD GATE]
- [ ] 🧹 `just format`
- [ ] 💾 **Single** `/smart-commit` for the whole feature (commit #2 on `refactor/ux-improvements`)

---

## 2. Detailed Implementation Plan

### Backend (`src-tauri/src/use_cases/account_summary/orchestrator.rs`)

- **Extend `AccountSummary`** with `pub total_unrealized_pnl: Option<i64>` and `pub ytd_performance_pct: Option<i64>` (doc comments citing ACC-023 / ACC-024; i64 micros / micro-percent per ADR-001).
- **`get_account_summaries` loop** — for each account, in addition to `compute_global_value`:
  - `total_unrealized_pnl` (ACC-023): the account-wide unrealized P&L per MKT-040/FXR-040. **Reuse, do not reimplement** the exclusion/FX logic.
  - `ytd_performance_pct` (ACC-024): the latest period's `year_to_date.pct` from the account-performance computation (PRF-034). `None` only when the Dietz denominator is 0 (PRF-032); first-year accounts use the inception baseline and are present.

  **Reuse approach (decided — ADR-004 directive): service-level / shared-helper reuse, NOT use-case composition.** `AccountSummaryUseCase` MUST inject services only (ADR-004) and never another use case. `plan-reviewer` confirmed this: the three use cases already share `AccountService` / `AssetService` / `CurrencyService`, and the existing `compute_global_value` already computes inline from those services (with a noted tech-debt to share the accumulator later) — so the summary already reaches everything it needs.
  - **`total_unrealized_pnl` (ACC-023)**: compute **inline** in the summary loop from the shared services, mirroring `compute_global_value` (same valuation pass / FX handling). This matches the existing precedent in this file.
  - **`ytd_performance_pct` (ACC-024)**: **extract** the Simple-Dietz YTD computation currently inside `AccountPerformanceUseCase` into a **shared pure helper** (e.g. a function/module under `use_cases/account_performance/` taking the inputs — net flows, year-start baseline value, current valuation, dates) callable by BOTH the performance orchestrator (unchanged behavior) and the summary orchestrator. The summary gathers the inputs via its existing services and calls the helper for the current-year span (Jan 1 → today). No use-case→use-case dependency; **no algorithm duplication**.
  - **Rejected alternative**: injecting `AccountDetailsUseCase` / `AccountPerformanceUseCase` into the summary (use-case-composes-use-case) — unsanctioned by ADR-004 and the naive list-view N+1 ADR-003 warns about.

- **DI wiring** (`src-tauri/src/lib.rs`): `AccountSummaryUseCase::new` keeps its current service set; no new use-case dependency. The shared YTD helper is a free function/module, not an injected collaborator.
- Errors: unchanged — `get_account_summaries` stays a read returning `DatabaseError`; per-account performance/price failures degrade to `None` (do not abort the list), consistent with the existing global-value degradation.

### Frontend (`src/features/accounts/`)

- **`shared/presenter.ts`** — format the two new fields on the account-row VM: `totalUnrealizedPnl` (account currency, "—" when null) and `ytdPerformancePct` (e.g. `+8,00%`, "—" when null); carry raw values for sorting + sign coloring.
- **`account_table/AccountTable.tsx`** — two new sortable `<th>` (after Global Value) + two `<td>` per row, with `id` per F25 where a stable selector is useful; P&L/percent sign coloring like the holding-row P&L cells.
- **`account_table/useAccountTable.ts`** — add `total_unrealized_pnl` and `ytd_performance_pct` sort keys; **nulls sort last** in both directions (ACC-008).
- **i18n** `src/i18n/locales/{en,fr}/common.json` — `account.column_unrealized_pnl`, `account.column_ytd_performance`.
- No gateway/data change — `useAccountSummaries` already returns the enriched `AccountSummary[]` after the bindings regen.

#### Rules Coverage

| Rule    | Layer              | Task                                                                                        | Notes                    |
| ------- | ------------------ | ------------------------------------------------------------------------------------------- | ------------------------ |
| ACC-023 | backend + frontend | `AccountSummary.total_unrealized_pnl` (orchestrator) + table column + presenter             | MKT-040/FXR-040; ADR-001 |
| ACC-024 | backend + frontend | `AccountSummary.ytd_performance_pct` (orchestrator, composes perf YTD) + column + presenter | PRF-034/PRF-032; ADR-013 |

**`modified_functions`** (for `test-writer-frontend`): `[account_table/useAccountTable.ts:sort comparator (new keys, nulls-last)]`, `[accounts/shared/presenter.ts:account-row mapping (new fields)]`, `[account_table/AccountTable.tsx:column rendering]`.

---

## 3. PR Plan

- **Strategy**: `1 commit` (user-directed — Workflow A rigor, single commit for all phases; no per-layer commits or PRs).
- **Estimate**: BE ~3 files / ~140 LOC (summary orchestrator inline unrealized + YTD-helper call; extract shared YTD helper from the performance orchestrator; performance orchestrator now calls the helper). FE ~4 files / ~120 LOC (presenter, table, sort hook, i18n). ~7 files / ~260 LOC — under split thresholds, tightly coupled (FE needs the enriched bindings). The accepted per-account recompute cost (ADR-003/013) is now just the YTD helper per account, not a full `AccountPerformanceResponse` assembly.
- **PR list**:
  - **Commit**: `feat(accounts): account-wide P&L and YTD on the overview`
  - **Scope**: all layers; lands as commit #2 on `refactor/ux-improvements`.
  - **Dependency**: none (branch already exists with commit #1).

> Minimal implementation: build only what makes the failing `test-writer-*` stubs pass — no
> speculative fields, no algorithm duplication (reuse MKT-040/FXR-040 + PRF-034).
