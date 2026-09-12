# Implementation Plan — Price Movement (PMV)

> Shipped in v0.40.0 (2026-09-11). The checklist below is the record of that walk; every box is ticked.

> Spec: [`docs/spec/price-movement.md`](../spec/price-movement.md) — trigram **PMV**, 31 rules, registered in [`docs/spec-index.md`](../spec-index.md) line 31 (status `planning`).
> Contract: [`docs/contracts/asset-contract.md`](../contracts/asset-contract.md) — changelog entry **2026-09-11** (`FetchTrigger`, `PriceMovementReport`, `PriceMovementRow`, `AssetPriceFetchCompleted.movement`).
> Branch: `feat/price-fetch-result`
> ADRs in force: **ADR-001** (i64 micros), **ADR-012** (latest-date-wins: `get_latest` is `ORDER BY date DESC LIMIT 1`), **ADR-013** (recompute on read — nothing persisted), **ADR-014** (refresh-lock = scope exclusion), **ADR-003/004** (use-case orchestration, inject services not repositories).
> **No migration** — `PriceMovementReport` is a transient value object, like `AssetLookupResult`.

---

## 1. Workflow TaskList

- [x] 📖 Review Architecture & Rules (`ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/backend-patterns.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/test_convention.md`, `docs/e2e-rules.md`, `docs/ubiquitous-language.md`)
- [x] 🗄️ Database Migration — **N/A**, no schema change (transient report)
- [x] ✍️ Backend test stubs (`test-writer-backend` → contract: `docs/contracts/asset-contract.md` § "Asset Price Fetch Tasks" + § Events `AssetPriceFetchCompleted` + § Shared Types `FetchTrigger` / `PriceMovementReport` / `PriceMovementRow` — all stubs written, red confirmed)
- [x] 🏗️ Backend Implementation (minimal — make failing tests pass, green confirmed; **no defensive code, no anticipation of future rules** — the test set defines the scope)
- [x] 🧹 `just format` (rustfmt + clippy --fix)
- [x] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` → save reports to `.review/` → `/review-triage` → apply Follow-ups)
- [x] 🔗 Type Synchronization (`just generate-types`)
- [x] ✍️ **Frontend test stubs — pass 1 (call-site only)** (`test-writer-frontend` → contract: `docs/contracts/asset-contract.md` § "Asset Price Fetch Tasks" + `modified_functions: [gateway.ts:fetchAllAssetPrices, App.tsx:maybeLaunchAutoFetch, useRefreshGlobalPrices.ts:refresh]` — red confirmed). Scope is strictly the `FetchTrigger` argument (PMV-010 / PMV-015); no panel, no report rendering.
- [x] 🔧 Call-site implementation (make pass-1 tests pass: `gateway.ts`, `App.tsx`, `useRefreshGlobalPrices.ts` — **no UI work**)
- [x] ✅ `just check` (TypeScript clean) **and** `npm run test:coverage` (the gate `quality.yml` actually runs on every PR — `just check` skips tests by design)
- [x] 💾 Commit: backend layer via `/smart-commit` — suggested title: `feat: report what a price refresh did to each account's value`
- [x] 🔀 `/create-pr` — **PR #1 (backend + call sites)** per the PR Plan. After merge, branch PR #2 off updated `main`.
- [x] ✍️ **Frontend test stubs — pass 2 (report surface)** (`test-writer-frontend` → contract + `modified_functions: [AccountManager.tsx:AccountManager]` — red confirmed)
- [x] 💻 Frontend Implementation (minimal — make failing tests pass, green confirmed; **no defensive code, no anticipation of future rules**)
- [x] 🧹 `just format`
- [x] 📸 Visual proof (`/visual-proof` — `PriceMovementPanel` in every state × light/dark, plus `AccountManager` as the consuming surface; stage screenshots before commit)
- [x] 🔍 Frontend Review (`reviewer-frontend` → save report to `.review/` → `/review-triage` → apply Follow-ups)
- [x] 💾 Commit: frontend layer via `/smart-commit` — suggested title: `feat: show the price movement panel after a manual refresh`
- [x] ✍️ E2E scenarios (`test-writer-e2e` — one deterministic scenario, see §2.4)
- [x] ▶️ Run E2E suite (`just test-e2e-headless` → green confirmed; main agent triages any failure). **Local E2E is known-broken on this machine (L-011)** — CI main-push E2E is the gate.
- [x] 🔍 E2E Review (`reviewer-e2e` → save report to `.review/` → `/review-triage` → apply Follow-ups)
- [x] 💾 Commit: E2E tests via `/smart-commit` — suggested title: `test: cover the price movement panel end to end`
- [x] 🔍 Cross-cutting Review (`reviewer-arch` — `.rs` modified; `reviewer-sql` — **skip, no migration**; `reviewer-infra` — **skip unless** a config/script/workflow file is touched; `reviewer-security` — **run**, `fetch_all_asset_prices` is a Tauri command whose signature changes → save reports to `.review/` → `/review-triage` → apply Follow-ups)
- [x] 📚 Documentation Update (`ARCHITECTURE.md` — new `use_cases/shared/global_value.rs`, `use_cases/shared/price_movement.rs`, `use_cases/asset_price_fetch/movement_capture.rs`, and the `features/accounts/price_movement/` sub-feature; `docs/spec-index.md` — flip PMV `planning` → shipped status; `docs/todo.md` — close any related entry)
- [x] ✅ Spec check (`spec-checker` against `docs/spec/price-movement.md`)
- [x] 💾 Commit: tests & docs via `/smart-commit` — suggested title: `docs: record the price movement report in the architecture map`
- [x] 🔀 `/create-pr` — **PR #2 (frontend + E2E + closure)** per the PR Plan

---

## 2. Detailed Implementation Plan

### 2.0 Migrations

**None.** The report is produced, published on an event, rendered, dismissed and discarded (PMV-061). No table, no column, no `just prepare-sqlx`.

---

### 2.1 Backend

All paths verified to exist unless marked **(new)**.

#### a. Wire types — `src-tauri/src/core/event_bus/event.rs` (modify)

- Add `PriceMovementRow` and `PriceMovementReport` structs, field-for-field as the contract declares them, immediately after the existing `UnpricedAsset` struct. Same derive set as `UnpricedAsset`: `#[derive(Debug, Clone, Eq, PartialEq, Serialize, specta::Type)]`.
- Extend the `Event::AssetPriceFetchCompleted` variant with `movement: Option<PriceMovementReport>`.
- **Why here and not in `use_cases/`**: `core/` is the lowest layer; it must not import from `use_cases/`. `UnpricedAsset` sets the precedent — event payload types live beside the event. The use-case modules import them (`use_cases` → `core` is the existing direction, already used by `dispatcher.rs`).
- All money fields `i64` micros (**ADR-001**); `total_movement_pct` / `movement_pct` are `Option<i64>` micro-percent, mirroring `AccountSummary.ytd_performance_pct` (8.00 % = `8_000_000`).
- Doc comments use **domain vocabulary** (`docs/ubiquitous-language.md`) and describe what the field _is_ — no transition/rationale comments.

#### b. Trigger type — `src-tauri/src/use_cases/asset_price_fetch/trigger.rs` **(new)**

```
pub enum FetchTrigger { Launch, Manual }
```

- Derives: `Debug, Clone, Copy, Eq, PartialEq, Deserialize, specta::Type` (it is a command **input**, so `Deserialize` not `Serialize`).
- Declared in `mod.rs` and re-exported (**B19**).
- **Verified finding — only two variants are needed.** The headless Scheduled fetch (SPF) does **not** go through `fetch_all_asset_prices` / `Dispatcher::spawn`: `use_cases/scheduled_fetch/orchestrator.rs` owns its own `gather_scope` + `sweep_prices_with_retry` loop and never publishes `AssetPriceFetchCompleted`. No third variant, no internal-only value. This matches the contract's `enum FetchTrigger { Launch, Manual }` exactly.

#### c. Command surface — `src-tauri/src/use_cases/asset_price_fetch/api.rs` (modify)

- `fetch_all_asset_prices(uc, trigger: FetchTrigger) -> Result<(), FetchAllAssetPricesError>` → `uc.fetch_all(trigger).await`.
- `fetch_account_asset_prices` unchanged (PMV-010 — account refresh never reports).
- No new error variant (PMV-014). `src-tauri/src/core/specta_builder.rs` needs **no edit** — the command name is unchanged and `Event` is already collected; `FetchTrigger` and `PriceMovementReport` are picked up transitively.

#### d. Orchestrator — `src-tauri/src/use_cases/asset_price_fetch/orchestrator.rs` (modify)

- `fetch_all(&self, trigger: FetchTrigger)` — thread `trigger` into `Arc::clone(&self.dispatcher).spawn(scope, fx_pairs, lease, trigger)`.
- Rejection paths are untouched: `FetchAlreadyRunning` (MKT-113) and `NoFetchableHoldings` (MKT-111) still return **before** `spawn`, so no `AssetPriceFetchCompleted` is published and no report exists → **PMV-012** falls out of the existing control flow with zero new code. Assert this in a test rather than adding a guard.
- `fetch_for_account` unchanged.

#### e. Global-value extraction — `src-tauri/src/use_cases/shared/global_value.rs` **(new)**

**This is the most important structural decision (PMV-023).** Today `compute_global_value` is a _private async method_ on `AccountSummaryUseCase` (`use_cases/account_summary/orchestrator.rs:196`) that reads prices and rates live. PMV needs the same arithmetic driven by _frozen_ prices and _frozen_ rates. **B18** forbids `asset_price_fetch` importing from `account_summary`. So extract the arithmetic into `use_cases/shared/`, following the `use_cases/shared/scope.rs` precedent (`build_scope` was extracted the same way for SPF-040).

Shape — pre-resolve into maps, then run a **pure synchronous** function, the idiom `valuation.rs` already uses (`RateMap`, `PricedAsset`):

- `pub struct AssetValuationFacts { pub currency: String, pub class: AssetClass }`
- `pub struct ValuationSnapshot { assets: HashMap<String, AssetValuationFacts>, prices: HashMap<String, AssetPrice>, rates: HashMap<(String, String), i64> }`
  - `prices`: `asset_id → the whole AssetPrice` — **price micros AND its observation date**. The date is load-bearing for the PMV-022 latest-wins guard in §2.1f; a bare `HashMap<String, i64>` cannot express that guard, and `observed_from` (a single max over the whole scope) does not supply it either.
  - `rates`: `(from_currency, to_currency) → rate micros`; identity pairs present as `1_000_000`; a **missing key means "no usable rate"** (FXR-034).
- `pub fn account_global_value(account_currency: &str, holdings: &[Holding], snapshot: &ValuationSnapshot) -> i64`

**The arithmetic must be copied verbatim, not simplified.** The current method truncates **twice**:

```rust
let converted_price = (latest.price as i128 * rate as i128 / 1_000_000) as i64;
let market_value = (holding.quantity as i128 * converted_price as i128 / 1_000_000) as i64;
```

The intermediate converted price is truncated to integer micros **before** it is multiplied by quantity. A fused `quantity × price × rate / MICRO²` is a _different_ number — they diverge by up to `quantity / MICRO` micros per holding. `use_cases/shared/valuation.rs:505-508` (`end_value_as_of`) uses the same two-step form, so two-step is the established project arithmetic and a fused version would be a third one. Preserve the two-step form exactly, including the intermediate narrowing to `i64`.

Remaining rules, line-for-line as today: skip `quantity <= 0`; a cash holding adds `holding.quantity` verbatim (CSH-094); a missing price **or** a missing rate contributes `0` (FXR-034); accumulate with `saturating_add`.

**Test the truncation, don't assume it.** "Existing account-summary tests stay green" is _not_ proof of behaviour preservation — those tests use round quantities and rates where both forms agree. Add a golden test with a deliberately truncating triple (e.g. price `3_333_333`, rate `333_333`, quantity `1_000_000_000`) asserting the two-step result, so any future fusion fails loudly.

**Error handling.** The current method returns `Err(AccountError::DatabaseError)` when a holding references a missing asset; a pure sync function cannot error. The failure moves to the **snapshot builder**: on the account-summary path the builder keeps raising `AccountError::DatabaseError` exactly as today; on the PMV path `capture()` degrades to `None` per **PMV-014**. Same detection, two policies, chosen by the caller.

`AccountSummaryUseCase::compute_global_value` becomes: load holdings → build the snapshot for that one account (the same live lookups it does today) → call `account_global_value`.

`pub const REFERENCE_CURRENCY: &str = "EUR";` — **moved** here from the private const in `use_cases/global_performance/orchestrator.rs:24` (GPF-011). PMV-040 needs it and B18 forbids importing it from `global_performance`. Mechanical move, ~2 LOC at each site.

> **Scope of the claim**: this makes `account_summary` and PMV share one Global-Value implementation, which is what PMV-023 requires ("the same one behind `AccountSummary.total_global_value`"). It is **not** an app-wide valuation unification — `use_cases/account_details/orchestrator.rs` keeps its own inlined per-holding accumulator by design (it needs per-holding intermediates the summary path discards). Do not widen the refactor to reach it.

> Gold/scope note: ~140 LOC of extraction plus call-site rewiring in two orchestrators, **mandated by PMV-023** — the alternative is a second valuation path, which the spec forbids outright. It stays inside the file set the feature already requires.

#### f. Report arithmetic — `src-tauri/src/use_cases/shared/price_movement.rs` **(new)**

**Pure, synchronous, no I/O, no injected services** — which is exactly what `use_cases/shared/` documents itself as holding ("stateless cross-context valuation primitives").

- `pub struct PriceMovementBaseline { accounts, holdings_by_account, snapshot: ValuationSnapshot, before_by_account: HashMap<String, i64>, observed_from: Option<String>, rate_missing_accounts: HashSet<String> }`
- `pub fn build_report(baseline: PriceMovementBaseline, fetched: &HashMap<String, AssetPrice>, unpriced_asset_ids: &HashSet<String>) -> PriceMovementReport`

**PMV-022 / PMV-026 — the latest-wins overlay guard.** Build the "after" snapshot by cloning the baseline snapshot and overlaying `fetched`, **but only where the fetched price actually becomes the asset's latest recorded price**:

```
apply fetched[asset_id] when
    baseline.snapshot.prices[asset_id] is absent            // never priced before
 OR fetched[asset_id].date >= baseline price's date         // equal date: the upsert replaced that row (MKT-025)
otherwise keep the baseline price
```

Why the guard is mandatory: `resolve_observation_date` (MKT-118) stores whatever provider date parses and is not in the future, so a fetch **can** write a row dated before the stored latest — the user records today's price manually in the morning, Yahoo returns Friday's close. `get_latest` is `ORDER BY date DESC LIMIT 1` (**ADR-012**), so that past-dated write changes nothing the application would read; an unconditional overlay would value the holding at Friday's price while `compute_global_value` still returns today's, and the report would state movement that did not happen — the precise failure PMV-023 forbids. PMV-022's own wording settles it: "the price **this refresh left in place**" — a past-dated write leaves nothing in place. The guard compares only against the baseline captured at refresh start, never a re-read, so **PMV-026 is preserved**.

Remaining arithmetic:

- Recompute `after` per account with `account_global_value` over the overlaid snapshot. Rates and holdings are the baseline's, untouched (**PMV-020**).
- One `PriceMovementRow` **per account** (**PMV-030**), including zero-movement and price-free accounts.
- `movement_pct = (after - before) × PERCENT_SCALE / before` in `i128`, then `i64`. `None` when `before <= 0` (**PMV-025**) or `after == before` (**PMV-031**).
- `incomplete` per row (**PMV-032**) = the account has an active holding whose `asset_id ∈ unpriced_asset_ids` (the MKT-171 skip set) **OR** the account is in `rate_missing_accounts`. Deliberately **not** set by: system cash (never in `unpriced`, MKT-116) or refresh-locked assets (never in fetch scope, so never in `unpriced` — MKT-151/158/ADR-014). Having no recorded price at all never sets it on its own.
- `rows` sorted by `name` ascending (**PMV-033**), independent of any FE sort.
- `total_before` / `total_after` = Σ over accounts of `value × (acct_ccy → EUR rate) / MICRO`; an account with no such rate contributes **0 to both** and is already `incomplete` (**PMV-042**). `total_currency = REFERENCE_CURRENCY` (**PMV-040**).
- `total_movement_pct` derived **from the two totals**, never from the row percentages (**PMV-041**); `None` when `total_before <= 0` (**PMV-044**) or the totals are equal (**PMV-045**).
- **Observation dates (PMV-050 / PMV-051 / PMV-052)** — the two dates are **independent**:
  - `observed_from` = the baseline's, i.e. the max price date over the in-scope assets at refresh start; `None` when no in-scope holding carried any recorded price before the refresh.
  - `observed_to` = max date among the **applied** `fetched` entries; `None` when the refresh produced no dated price, **or** when `observed_from` is present and `observed_to` is not strictly later than it (**PMV-051**).
  - When `observed_from` is `None` but the refresh produced a date, **`observed_to` is still stated** (**PMV-052** as amended) — the comparison reads as running from nothing priced to that date. Only when both are absent does the report carry no date at all. _(Do not force both to `None`; that would hide a real date on a first-ever fetch.)_
- `incomplete` (report level) = any row incomplete (**PMV-043**).
- **PMV-060 carries no backend flag**: "nothing moved" is `rows.iter().all(|r| r.before == r.after)` — the FE derives the empty state from values it already receives, and the report states no count of its own. Adding a `moved: bool` field would deviate from the contract.

- Declare `global_value` and `price_movement` in `src-tauri/src/use_cases/shared/mod.rs` with a one-line doc comment each, matching the existing `scope` / `valuation` entries.

#### g. Baseline capture — `src-tauri/src/use_cases/asset_price_fetch/movement_capture.rs` **(new)**

The **stateful** half lives with its only caller, not in `shared/` — `shared/` is stateless free functions, while this holds three injected `Arc<dyn Service>` dependencies. **B18** forbids importing another _use case_; it does not require `shared/`.

```
pub struct PriceMovementCapture {
    account_service: Arc<dyn AccountServiceContract>,
    asset_service: Arc<dyn AssetServiceContract>,
    currency_service: Arc<CurrencyService>,
}
```

- `pub async fn capture(&self, scope_asset_ids: &HashSet<String>, today: NaiveDate) -> Option<PriceMovementBaseline>` — the **"before" snapshot, taken at refresh start**:
  - Load every account (`account_service.get_all`) and its holdings (**B24 / ADR-004**: services, never repositories).
  - Build the `ValuationSnapshot`: asset facts + `get_latest_price` per distinct asset (kept whole, **with its date** — §2.1e) + **every rate the two readings will ever need**, resolved once here with `as_of = today`:
    - `(asset_currency → account_currency)` for every active non-cash holding, and
    - `(account_currency → REFERENCE_CURRENCY)` for every account.
  - Compute `before` per account via `account_global_value`.
  - `observed_from` = max `AssetPrice.date` over the assets in `scope_asset_ids`; `None` when none carries a price.
  - `rate_missing_accounts` = any account with an active non-cash holding whose `(asset_ccy → acct_ccy)` rate is absent, **or** whose own `(acct_ccy → EUR)` rate is absent (PMV-032 second condition, PMV-042).
  - Returns `None` on any lookup failure — including a holding referencing a missing asset — after `tracing::warn!(target: BACKEND, …)`. **PMV-014**: the fetch is never aborted or altered by reporting.
- **This is how PMV-020's frozen rates are threaded**: the rate map is captured here, before the price loop, and the same map is reused for the "after" reading, so the FX refresh that FXR-075 piggybacks _after_ the loop can never leak into either reading.

#### h. Dispatcher — `src-tauri/src/use_cases/asset_price_fetch/dispatcher.rs` (modify)

- `Dispatcher::new` gains `movement_capture: Arc<PriceMovementCapture>` (6 deps — `#[allow(clippy::too_many_arguments)]` is permitted on a production constructor per **B32**). Grep `Dispatcher::new` and update every construction site, including the test helpers in `orchestrator.rs`.
- `spawn(self, scope, fx_pairs, lease, trigger: FetchTrigger)`.
- Inside the spawned task, **before** the per-asset loop:
  - `let scope_asset_ids: HashSet<String> = scope.iter().map(|(a, _)| a.id.clone()).collect();`
  - `let baseline = if trigger == FetchTrigger::Manual { self.movement_capture.capture(&scope_asset_ids, today).await } else { None };`
  - Capturing at the _top of the spawned task_ rather than in the orchestrator keeps `fetch_all_asset_prices` returning `Ok(())` immediately (MKT-112 acknowledgement contract); it is the earliest point that is still "refresh start" for the reading.
- Inside the loop, on each successful upsert, record the written price: `fetched.insert(asset.id.clone(), record.clone());` — the map the PMV-022 guard consumes. `unpriced` already accumulates the MKT-171 skip set.
- Immediately **before** the existing `Event::AssetPriceFetchCompleted` publish (currently line ~133):
  - `let movement = baseline.map(|b| build_report(b, &fetched, &unpriced_ids));`
  - Publish `Event::AssetPriceFetchCompleted { ok, skipped, unpriced, movement }`.
  - The FX refresh (FXR-075) stays **after** the publish, exactly where it is — the readings already hold a frozen rate map, so ordering cannot contaminate them, and PMV-014 is preserved.
- `resolve_observation_date` and every existing MKT behaviour are untouched (**PMV-014**).

#### i. Serde guard — `src-tauri/src/use_cases/asset_price_fetch/serde_check.rs` (modify)

- Add a case asserting `AssetPriceFetchCompleted` serializes `movement` as the contract declares (present object for `Manual`, `null`/absent otherwise), alongside the existing checks.

---

### 2.2 Type Synchronization

`just generate-types` regenerates `src/bindings.ts`:

- `commands.fetchAllAssetPrices(trigger)` — **one positional arg** (Critical Pattern: match COUNT, ORDER, NAMES from `bindings.ts`; never object-wrap).
- `FetchTrigger` → `"Launch" | "Manual"`.
- `PriceMovementReport`, `PriceMovementRow` exported types.
- `AssetPriceFetchCompleted` payload gains `movement: PriceMovementReport | null`.

`src/bindings.ts` is **generated — never hand-edited** (F22: never redeclare Specta enum values on the FE).

---

### 2.3 Frontend

> **Dumb-UI constraint** (stricter than F27): the panel renders exactly what the backend sends. No FE recomputation of `movement_pct`, no FE re-sorting of `rows`, no FE derivation of `incomplete`. The only FE logic is formatting (micros → locale string), i18n key selection, and the "did anything move" empty-state check over the values the backend already supplied.

#### Pass 1 — the `FetchTrigger` call sites (ships in **PR #1**)

Tests first (`test-writer-frontend` pass 1), then implementation. These three files are what make the regenerated binding compile and the existing suite pass; they are **not** optional cleanup.

##### a1. `src/features/accounts/gateway.ts` (modify) + `gateway.test.ts` (modify)

- `fetchAllAssetPrices(trigger: FetchTrigger): Promise<Result<null, FetchAllAssetPricesError>>` → `commands.fetchAllAssetPrices(trigger)`. **This is the file that calls the regenerated binding** — without it `tsc` fails.
- `gateway.test.ts:218` currently asserts `mockInvoke).toHaveBeenCalledWith("fetch_all_asset_prices")` — must become the trigger-carrying form.

##### b1. `src/App.tsx` (modify) + `src/App.test.ts` (modify)

- `maybeLaunchAutoFetch()` passes `"Launch"` (**PMV-010 / PMV-015**) — the launch auto-fetch must **not** report movement.
- `App.test.ts:36` currently asserts `toHaveBeenCalledWith()` (zero args) — must assert `"Launch"`.

##### c1. `src/features/accounts/refresh_prices/useRefreshGlobalPrices.ts` (modify) + `.test.ts` (modify)

- `refresh()` passes `"Manual"` (**PMV-010 / PMV-015**). Snackbar behaviour unchanged (MKT-115).
- `useRefreshGlobalPrices.test.ts:156` currently asserts `toHaveBeenCalledWith()` (zero args) — must assert `"Manual"`.

#### Pass 2 — the report surface (ships in **PR #2**)

##### d. `src/features/accounts/gateway.ts` (modify) + `gateway.test.ts` (modify)

- Add `subscribeToPriceFetchCompleted(callback: (payload) => void): Promise<() => void>` — `events.event.listen`, filtered to `payload.type === "AssetPriceFetchCompleted"`, passing the **full payload**. The existing `subscribeToEvents` only forwards `type`, so it cannot carry `movement`. Gateway is the only file allowed to touch `commands.*` / `events.*` (**F3**).

##### e. `src/features/accounts/price_movement/usePriceMovementReport.ts` **(new)** + `.test.ts` **(new)**

- Subscribes via `accountGateway.subscribeToPriceFetchCompleted` on mount; stores `payload.movement` in local state when non-null; exposes `{ report, dismiss }`.
- **PMV-016 falls out of the mount lifecycle**: the hook is mounted only by `AccountManager`. Navigate away → unmount → listener removed → nothing captured, nothing kept for the user's return. **Do not route this through `src/lib/store.ts`** — the store is the app's always-on sink and would retain the report across navigation, violating PMV-016. `store.ts` needs **no change**; its existing handler ignores the additive `movement` field.
- **PMV-061**: `dismiss()` clears local state; no persistence, no restore path. A fresh refresh replaces it.
- F20: per-effect `let isMounted = true`, never a shared ref. F19: stable references outside `renderHook`.

##### f. `src/features/accounts/price_movement/PriceMovementPanel.tsx` **(new)** + `.test.tsx` **(new)**

- Dismissible, non-blocking inline panel — **not** a modal, never a scrim (**PMV-013**). Independent of `UnpricedPricesModalMount` (MKT-172), which keeps behaving exactly as today.
- Layout adapted from the visual reference (`https://claude.ai/code/artifact/d0f31e88-a02b-4df2-8b42-5d7114c961e7`) from modal chrome to an inline M3 surface: `bg-m3-surface-container-high rounded-[28px] shadow-elevation-1`, tokens from `src/ui/global.css`.
- States, all driven by backend-supplied values:
  - **Nothing moved** (`rows.every(r => r.before === r.after)`) — plain sentence, no table; plus the incompleteness sentence when `report.incomplete` (**PMV-060**). No count of its own.
  - **Moved** — the table: name / before / after / movement. Unmoved rows render `—` in the movement cell, never `0.00 %` (**PMV-031**). Row values formatted in `row.currency` (**PMV-034**).
  - **Partially complete** — an `incomplete` marker on affected rows and on the total row (**PMV-032 / PMV-043**).
  - **Dates** — both present → `from → to`; only `observed_from` → that single date; only `observed_to` → that single date (first-ever-priced case, **PMV-052** as amended); neither → no date labels on the value columns.
  - **Error** — none. A missing `movement` simply renders nothing (**PMV-014**).
- Total row: `total_before` / `total_after` / `total_movement_pct` in `total_currency` (**PMV-040/041/044/045**).
- **PMV-017**: a header line framing the figures as a dated before/after of _this refresh_, not current values.
- Stable `id` on the panel root, the dismiss button and each row (**E1/E4**): `price-movement-panel`, `price-movement-dismiss`, `price-movement-row-{account_id}`, `price-movement-total`.
- i18n on every string including `aria-label` / `title` (**F16 / F24**).

##### g. `src/features/accounts/shared/presenter.ts` (modify) + `presenter.test.ts` (modify)

Pure functions, no React, no `useTranslation` (**F27 layer 3**), reusing `microToFormatted` from `@/lib/microUnits` and the existing `formatAccountRowYtdPerformancePct` signed-percent idiom:

- `formatPriceMovementValue(micros: number): string` — 2 decimals.
- `formatPriceMovementPct(pct: number | null): string` — signed micro-percent, `"—"` when `null`.
- `priceMovementDateLabel(from: string | null, to: string | null): I18nMessage | null` — picks the two-date / from-only / to-only / no-date key (**PMV-050/051/052**) from the values as given; it never invents or compares dates beyond null-checks.

##### h. `src/features/accounts/AccountManager.tsx` (modify) + `AccountManager.test.tsx` (modify)

- Call `usePriceMovementReport()`; render `<PriceMovementPanel …/>` above the account rows. This is a **modification of an existing component** → covered by `modified_functions` in pass 2.
- `src/ui/components/layout/ManagerLayout.tsx` (modify): add an optional `banner?: ReactNode` slot rendered between `ManagerHeader` and the scrolling `table` container (~4 LOC, additive, default-undefined). Verified the only other consumers are `AssetManager.tsx` and `CategoryManager.tsx`, both unaffected. **Endorsed by plan-reviewer over the table-slot alternative**, so the panel does not scroll away with the rows.

##### i. i18n — `src/i18n/locales/en/common.json` + `fr/common.json` (modify)

New `pmv` namespace beside the existing `mkt` one. Both locales updated in the same commit; `src/i18n/locales.test.ts` enforces key parity. Indicative keys:
`pmv.title`, `pmv.subtitle_frozen` (PMV-017), `pmv.dismiss`, `pmv.column_account`, `pmv.column_before`, `pmv.column_after`, `pmv.column_movement`, `pmv.total_row`, `pmv.dates_range` (`{{from}} → {{to}}`), `pmv.dates_single`, `pmv.incomplete_marker`, `pmv.incomplete_explainer`, `pmv.nothing_moved`, `pmv.nothing_moved_incomplete`.

---

### 2.4 E2E — `e2e/accounts/price_movement.test.ts` **(new)**

One deterministic, pyramid-friendly scenario, reusing the seeding strategy proven in `e2e/account_details/manual_price_fill.test.ts`:

- Seed one account + one asset whose reference (`ZZ-NOPE-*`) Yahoo cannot resolve → the fetch attempts it and skips it, **online or offline**.
- From the accounts list, click `#account-manager-refresh-prices`.
- Assert the panel `#price-movement-panel` appears (**PMV-013**), shows the "nothing moved" statement together with the incompleteness sentence (**PMV-060 + PMV-032**), and that it coexists with — does not queue behind — the unupdated-prices modal (**PMV-013 / MKT-172**).
- Dismiss via `#price-movement-dismiss`; assert it does not return (**PMV-061**).
- Selectors use stable `id`s only (**E1/E4**); every `waitForExist` / `waitForEnabled` passes an explicit `{ timeout }` (**E10**); click a leaf `td`, never a `<tr>`.
- Header block documents spec + contract + this plan, matching the house style.

---

### 3. Rules Coverage

| Rule    | Layer              | Task                                                                                                                                      | Notes                                                                     |
| ------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| PMV-010 | frontend + backend | `dispatcher.rs` trigger gate; §2.3b1 `App.tsx:maybeLaunchAutoFetch` → `"Launch"`; §2.3c1 `useRefreshGlobalPrices.ts:refresh` → `"Manual"` | `[unit-test-needed]` — FE pass 1                                          |
| PMV-011 | backend            | `dispatcher.rs` — one `build_report` call, after the loop, before the publish                                                             | exactly one per task                                                      |
| PMV-012 | backend            | `orchestrator.rs::fetch_all` — rejections return before `spawn`                                                                           | no new code; assert existing flow                                         |
| PMV-013 | frontend           | §2.3f `PriceMovementPanel.tsx` (new) + §2.3h `AccountManager.tsx:AccountManager` + `ManagerLayout.tsx` `banner` slot                      | `[unit-test-needed]` — FE pass 2                                          |
| PMV-014 | backend            | `movement_capture.rs::capture` returns `None` on failure; FX refresh untouched                                                            | `tracing::warn!`, fetch never aborted                                     |
| PMV-015 | frontend + backend | `trigger.rs::FetchTrigger`; `api.rs::fetch_all_asset_prices`; §2.3a1/b1/c1 FE call sites                                                  | `[unit-test-needed]` — FE pass 1; 2 variants only, SPF is a separate path |
| PMV-016 | frontend           | §2.3e `usePriceMovementReport.ts` — mount-scoped listener                                                                                 | not in `lib/store.ts`                                                     |
| PMV-017 | frontend           | `PriceMovementPanel.tsx` header + `pmv.subtitle_frozen`                                                                                   | frozen-comparison framing                                                 |
| PMV-020 | backend            | `capture` — one rate map, resolved at refresh start, reused by both readings                                                              | frozen before the FX refresh (FXR-075)                                    |
| PMV-021 | backend            | `capture` → `account_global_value` on the baseline snapshot                                                                               | ADR-001 (i64 micros)                                                      |
| PMV-022 | backend            | `build_report` — overlay guarded latest-date-wins against the baseline price date                                                         | **ADR-012**; past-dated write leaves nothing in place                     |
| PMV-023 | backend            | `shared/global_value.rs::account_global_value`, shared with `AccountSummaryUseCase`                                                       | **two-step truncation preserved verbatim**; golden test                   |
| PMV-024 | frontend + backend | `build_report` computes `movement_pct`; `presenter.ts::formatPriceMovementPct` renders it                                                 | micro-percent, ADR-001                                                    |
| PMV-025 | backend            | `build_report` — `None` when `before <= 0`                                                                                                |                                                                           |
| PMV-026 | backend            | `build_report` is pure over `fetched` + baseline — no DB re-read                                                                          | excludes SPF-023 / manual / sync writes                                   |
| PMV-030 | backend            | `build_report` — one row per account                                                                                                      | includes unmoved + price-free                                             |
| PMV-031 | frontend + backend | `build_report` → `None` when `after == before`; panel renders `—`                                                                         | never `0.00 %`                                                            |
| PMV-032 | frontend + backend | `build_report` — `unpriced_asset_ids` ∪ `rate_missing_accounts`; panel marker                                                             | cash + locked never mark                                                  |
| PMV-033 | backend            | `build_report` — sort by `name` ascending                                                                                                 | FE never re-sorts                                                         |
| PMV-034 | frontend + backend | `PriceMovementRow.currency`; `formatPriceMovementValue`                                                                                   | no conversion                                                             |
| PMV-040 | backend            | `build_report` totals + `total_currency` = `REFERENCE_CURRENCY`                                                                           | const moved to `shared/global_value.rs` (GPF-011)                         |
| PMV-041 | backend            | `build_report` — total pct from the two totals                                                                                            | never from row pcts                                                       |
| PMV-042 | backend            | `build_report` — no `acct → EUR` rate ⇒ contributes 0 to both totals, row `incomplete`                                                    | FXR-034 / GPF                                                             |
| PMV-043 | frontend + backend | `report.incomplete = rows.any(incomplete)`; panel total marker                                                                            |                                                                           |
| PMV-044 | backend            | `build_report` — total pct `None` when `total_before <= 0`                                                                                | mirrors PMV-025                                                           |
| PMV-045 | backend            | `build_report` — total pct `None` when totals equal                                                                                       | mirrors PMV-031                                                           |
| PMV-050 | frontend + backend | `capture` → `observed_from`; `build_report` → `observed_to`; `priceMovementDateLabel`                                                     | max over in-scope holdings                                                |
| PMV-051 | frontend + backend | `build_report` — `observed_to = None` unless strictly later than a present `observed_from`                                                | says nothing about movement                                               |
| PMV-052 | frontend + backend | no prior price ⇒ `observed_from = None`, **`observed_to` still stated** when the refresh produced one; both absent ⇒ no dates             | amended spec wording                                                      |
| PMV-060 | frontend + backend | panel empty-state from `rows.every(before === after)` + `report.incomplete`                                                               | no backend flag, no count                                                 |
| PMV-061 | frontend           | `usePriceMovementReport.ts:dismiss`                                                                                                       | no persistence, no restore                                                |

**`modified_functions` handoffs for `test-writer-frontend`** — two passes, because the call-site rules ship in PR #1 and the report surface in PR #2:

```
pass 1 (PR #1) → docs/contracts/asset-contract.md § "Asset Price Fetch Tasks"
  + modified_functions: [gateway.ts:fetchAllAssetPrices,
                         App.tsx:maybeLaunchAutoFetch,
                         useRefreshGlobalPrices.ts:refresh]

pass 2 (PR #2) → docs/contracts/asset-contract.md § Events / § Shared Types
  + modified_functions: [AccountManager.tsx:AccountManager]
```

Both passes run **before** the corresponding implementation step — test-first holds for every `[unit-test-needed]` rule.

---

## 4. PR Plan

- **Strategy**: **2 PRs**

- **Estimate** (from the verified file set):
  - **PR #1 — backend + call sites**: ~12 Rust files / ~800–950 LOC churn (`price_movement.rs` ≈ 230 + 260 test LOC; `global_value.rs` ≈ 130 + tests incl. the truncation golden; `movement_capture.rs` ≈ 140; `dispatcher.rs` +60; `event.rs` +50; orchestrator/api/mod/trigger/serde_check/account_summary/global_performance ≈ 80), plus generated `bindings.ts` and 3 FE files + their 3 tests (~60 LOC).
  - **PR #2 — frontend + E2E + closure**: ~13 files / ~450–550 LOC (new hook + panel + their tests ≈ 300; gateway subscription + presenter + AccountManager + ManagerLayout + i18n ≈ 180), plus screenshots, 1 E2E spec (~150 LOC) and `ARCHITECTURE.md` / `spec-index.md` / `todo.md`.

  A single PR would land ~1400–1600 LOC of churn and tell two stories (a backend valuation extraction + a new UI surface), breaching the ≤1000 LOC target. Three PRs would over-slice: the E2E layer is one spec file plus doc closure — not enough to justify its own review cycle on top of the FE PR it validates.

- **PR list**:
  1. **`feat: report what a price refresh did to each account's value`**
     - **Scope**: all backend files (§2.1a–i), regenerated `src/bindings.ts`, **and the three FE call sites with their tests** (§2.3a1/b1/c1: `gateway.ts` + `gateway.test.ts`, `App.tsx` + `App.test.ts`, `useRefreshGlobalPrices.ts` + `useRefreshGlobalPrices.test.ts`). **No UI work.**
     - **Why the call sites must be in this PR**: `gateway.ts` is the file that calls the regenerated binding, so `tsc` fails without it; and three existing tests assert zero-argument calls (`src/App.test.ts:36`, `src/features/accounts/refresh_prices/useRefreshGlobalPrices.test.ts:156`, `src/features/accounts/gateway.test.ts:218`). `quality.yml:115` runs `npm run test:coverage` on every PR, so omitting them turns PR #1 red no matter what `just check` reports — `just check` skips tests by design and is the wrong mergeability gate.
     - Terminates at the Workflow TaskList checkpoint **"💾 Commit: backend layer"**.
     - **Dependency**: none — branches off `main`. Mergeable alone: `Manual` reports arrive on an event nobody renders yet; the launch fetch keeps its exact behaviour.
     - **Branch suffix**: `feat/price-fetch-result-be`

  2. **`feat: show the price movement panel after a manual refresh`**
     - **Scope**: frontend pass 2 (§2.3d–i), **+ E2E** (`e2e/accounts/price_movement.test.ts`) **+ closure** (`ARCHITECTURE.md`, `docs/spec-index.md`, `docs/todo.md`, `spec-checker`), plus screenshots.
     - Terminates at **"💾 Commit: tests & docs"** → final `/create-pr`.
     - **Dependency**: rebase off `main` after PR #1 merges — the panel needs the `PriceMovementReport` binding.
     - **Branch suffix**: `feat/price-fetch-result-fe`

> The strategy above is the planner's call from the measured file set. `AskUserQuestion` was not available in this agent context — if the user prefers a single PR or a 3-way split, adjust this section and the two `/create-pr` checkpoints in §1 before `test-writer-backend` runs.

---

## 5. Decisions settled in review

1. **Overlay is guarded latest-date-wins**, not unconditional (§2.1f) — ADR-012 + MKT-118 make a past-dated fetch write invisible to `get_latest`, so an unconditional overlay would report movement that did not happen. `ValuationSnapshot.prices` therefore carries `AssetPrice` (micros **and** date), not bare micros.
2. **Two-step truncation preserved verbatim** (§2.1e) — a fused `quantity × price × rate / MICRO²` is a different number from the existing `account_summary` and `valuation.rs:505-508` arithmetic. Guarded by a golden test with a deliberately truncating triple.
3. **Extraction scope is `account_summary` + PMV only** — `account_details/orchestrator.rs` keeps its own inlined accumulator by design; do not widen.
4. **Stateful `capture()` lives in `use_cases/asset_price_fetch/`**, pure `build_report()` + `global_value.rs` in `use_cases/shared/` — `shared/` is stateless free functions; B18 forbids importing another use case, it does not mandate `shared/`.
5. **`ManagerLayout.banner` slot** endorsed over wrapping the panel in the `table` slot.
6. **`FetchTrigger` has exactly two variants** — verified that the Scheduled fetch (`use_cases/scheduled_fetch/orchestrator.rs`) runs its own sweep and never reaches `Dispatcher::spawn`.
7. **PMV-052 as amended** — no prior price suppresses only the _earlier_ date; a date the refresh produced is still stated.
