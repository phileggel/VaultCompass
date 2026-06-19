# Implementation Plan — Unupdated-Price Manual Fill (MKT-170–179)

> Scope: ONLY the new `market-price` rules **MKT-170 → MKT-179** (the "Unupdated-Price
> Manual Fill" block). All other MKT rules are already shipped and out of scope.
> Spec: `docs/spec/market-price.md` · Contract: `docs/contracts/asset-contract.md`
> No new Tauri command (per-row fill reuses `record_asset_price`). No DB migration.

After a fetch task leaves assets unpriced (the MKT-114 skip set), its completion signal
now carries the unpriced list (MKT-170/171). The frontend auto-opens a modal (MKT-172)
listing each asset (name / last value / ticker / ISIN / empty input); a per-row confirm
records a `Manual` price via the existing `record_asset_price` path (MKT-175), skip leaves
it stale (MKT-176), and the modal supersedes the MKT-145 snackbar (MKT-173).

---

## 1. Workflow TaskList

**Setup**

- [ ] 📖 Read spec: `docs/spec/market-price.md` (MKT-170–179 only)
- [ ] 📖 Read contract: `docs/contracts/asset-contract.md` (`UnpricedAsset` type + `AssetPriceFetchCompleted` event)
- [ ] 📖 Read constraining ADRs: `docs/adr/001-use-i64-for-monetary-amounts.md` (last_price i64 micros), `docs/adr/012-latest-write-wins-source-as-metadata.md` (manual write overwrite semantics)
- [ ] 📖 Read conventions: `ARCHITECTURE.md`, `docs/backend-rules.md`, `docs/ddd-reference.md`, `docs/error-model.md`, `docs/frontend-rules.md`, `docs/i18n-rules.md`, `docs/frontend-visual-proof.md`, `docs/test_convention.md`

**Backend phase**

- [ ] ✍️ Backend test stubs (`test-writer-backend` — dispatcher collects the unpriced list; event payload carries it; red confirmed)
- [ ] 🏗️ Backend Implementation (minimal — make failing tests pass)
- [ ] 🔍 Backend Review (`reviewer-backend` + `reviewer-arch` in parallel → `/review-triage` → apply Follow-ups)
- [ ] 🔗 Type Synchronization (`just generate-types` → `src/bindings.ts`)
- [ ] 🔧 `npx tsc --noEmit` → fix TS errors from new bindings only (no UI work)
- [ ] 🧹 `just format`
- [ ] 💾 (no commit yet — single-PR; continue to FE)

**Frontend phase**

- [ ] ✍️ Frontend test stubs (`test-writer-frontend`; pass `modified_functions` list below; red confirmed)
- [ ] 💻 Frontend Implementation (minimal — make failing tests pass)
- [ ] 📸 Visual proof (`/visual-proof` on `UnpricedPricesModal` — idle / single-row / multi-row / row-error, light + dark; stage screenshots)
- [ ] 🔍 Frontend Review (`reviewer-frontend` → `/review-triage` → apply Follow-ups)
- [ ] 🧹 `just format`

**Closure**

- [ ] ✍️ E2E scenario (`test-writer-e2e`): fetch-with-failure → modal auto-opens → enter one price → skip one → assert holding/price reflects the entered value and the modal closes
- [ ] ▶️ Run E2E suite (`just test-e2e-headless` → green; main agent triages any failure)
- [ ] 🔍 Cross-cutting Review (`reviewer-e2e` — E2E test files added; `reviewer-security` NOT required — no new command/capability) → `/review-triage`
- [ ] 📚 Documentation Update: `docs/todo.md` (none open for this); `ARCHITECTURE.md` (register the `unpriced_prices` feature + the `UnpricedPricesModalMount` shell mount; note `store.ts` stashes the unpriced list)
- [ ] ✅ Spec check (`spec-checker` on MKT-170–179) [HARD GATE]
- [ ] 🧹 `just format`
- [ ] 💾 Commit: whole feature via `/smart-commit`
- [ ] 🔀 Land per PR Plan (single PR → `just merge` or `/create-pr`)

---

## 2. Detailed Implementation Plan

### Backend (`src-tauri/src/`)

**`core/event_bus/event.rs`** — extend the event payload + define the payload type:

- Add `unpriced: Vec<UnpricedAsset>` to the `AssetPriceFetchCompleted` variant (MKT-170).
- Define `pub struct UnpricedAsset` co-located in this module (keeps `core` free of context imports — matches the existing primitive-only `Event` enum). Derive the same trait set the enum needs: `Debug, Clone, Eq, PartialEq, Serialize, specta::Type`. Fields per contract: `asset_id: String`, `name: String`, `reference: String`, `isin: Option<String>`, `currency: String`, `last_price: Option<i64>` (i64 micros, ADR-001), `last_price_date: Option<String>`.

**`core/specta_builder.rs`** — register `UnpricedAsset` with `.typ::<UnpricedAsset>()` if it is not auto-exported through the `tauri_specta::Event` collection of `Event` (verify after `just generate-types`; add only if the type is missing from `bindings.ts`).

**`use_cases/asset_price_fetch/dispatcher.rs`** — collect the unpriced list (MKT-170/171):

- In the per-asset loop, when an asset is skipped (the existing `skipped += 1` arms: `Ok(None)`, `Err`, and the upsert-failure arm), also push an `UnpricedAsset` built from the in-hand `Asset` (name, reference, isin, currency).
- Populate `last_price` / `last_price_date` from the asset's most recently recorded price via the existing `AssetPriceRepository::get_latest(asset_id)` (confirmed present; returns `None` when the asset has never had a price). No new repo method needed.
- Include the accumulated `Vec<UnpricedAsset>` in the published `Event::AssetPriceFetchCompleted { ok, skipped, unpriced }`. Invariant: `unpriced.len() == skipped` (MKT-171).
- Cash (MKT-116) and refresh-locked (MKT-151) assets are already excluded from `scope` upstream, so they never enter the list — no extra filtering here.

> No new command. `record_asset_price` (already in the contract) is reused unchanged by the FE per-row fill. `record_asset_price` already publishes `AssetPriceUpdated` (MKT-179) — no backend change for reactivity.

### Frontend (`src/`)

**`src/lib/store.ts`** — extend the `AssetPriceFetchCompleted` handler (around line 151):

- Stash `payload.unpriced` into a new store slice (`unpricedAssets: UnpricedAsset[]`) with a clear/dismiss action (MKT-172, MKT-177).
- Gate the MKT-145 snackbar: when `unpriced` is non-empty, do NOT show the snackbar (the modal supersedes it — MKT-173); when empty, snackbar behaves exactly as today.

**`src/features/unpriced_prices/`** (new feature, F0/F28 layout):

- `gateway.ts` — thin wrapper over `commands.recordAssetPrice(asset_id, date, price)` (MKT-175); own gateway (no cross-feature import, F26).
- `useUnpricedPrices.ts` — row state machine: per-row record (calls gateway, dated to today's local ISO date), skip, resolve/remove, in-flight + inline-error handling (MKT-175–179). Reads the store slice; clears it on dismiss.
- `UnpricedPricesModal.tsx` — modal listing one row per asset: name, last value (formatted in asset currency, or "no previous price"), ticker, ISIN, empty price input + currency label, confirm + skip per row (MKT-174, 176, 177, 178). Reuse the shared money-formatting util used by the account-details price column (verify the shared util path; add a tiny presenter only if none is shared).
- Colocated tests: `UnpricedPricesModal.test.tsx`, `useUnpricedPrices.test.ts`, `gateway.test.ts`.

**`src/features/shell/UnpricedPricesModalMount.tsx`** (new) — reads the `unpricedAssets` store slice; renders `UnpricedPricesModal` when the slice is non-empty (MKT-172). Mirrors the existing `*Mount` pattern.

**`src/AppShell.tsx`** — wire `<UnpricedPricesModalMount />` alongside the existing mounts.

**i18n** — add modal strings (title, column headers, "no previous price", confirm/skip, per-row success/error) to the `en` + `fr` locale files under `src/i18n/locales/`; follow `docs/i18n-rules.md`.

#### Rules Coverage

| Rule    | Layer              | Task                                                                                | Notes                         |
| ------- | ------------------ | ----------------------------------------------------------------------------------- | ----------------------------- |
| MKT-170 | backend            | `event.rs` payload + `UnpricedAsset` type; `dispatcher.rs` collects list            | ADR-001 (last_price i64)      |
| MKT-171 | backend            | `dispatcher.rs` — list == skipped set; cash/locked excluded upstream                | invariant `len == skipped`    |
| MKT-172 | frontend           | `store.ts` slice + `UnpricedPricesModalMount` auto-render                           | `[unit-test-needed]` store.ts |
| MKT-173 | frontend           | `store.ts` — suppress MKT-145 snackbar when list non-empty                          | `[unit-test-needed]` store.ts |
| MKT-174 | frontend           | `UnpricedPricesModal.tsx` row layout                                                |                               |
| MKT-175 | frontend + backend | `useUnpricedPrices.ts` + `gateway.ts` → reuse `record_asset_price` (today, Manual)  | ADR-012; no new command       |
| MKT-176 | frontend           | `useUnpricedPrices.ts` skip → remove row, no write                                  |                               |
| MKT-177 | frontend           | `useUnpricedPrices.ts` resolve/dismiss → close when empty; dismiss = skip remaining |                               |
| MKT-178 | frontend           | `UnpricedPricesModal.tsx` / `useUnpricedPrices.ts` per-row in-flight/success/error  |                               |
| MKT-179 | frontend + backend | reactivity via existing `AssetPriceUpdated` from `record_asset_price`               | no backend change             |

**`modified_functions`** (for `test-writer-frontend`): `[store.ts:AssetPriceFetchCompleted handler (buildPriceFetchFeedback + the payload.type === "AssetPriceFetchCompleted" branch)]` — MKT-172/173 modify existing store logic.

---

## 3. PR Plan

- **Strategy**: `1 PR`
- **Estimate**: BE ~4 files / ~130 LOC; FE ~7 files / ~260 LOC; E2E 1 file. Total ~12 files / ~390 LOC — under the 20-file / 500-LOC split threshold, and the FE depends on the new event payload + regenerated bindings, so the layers are tightly coupled.
- **PR list**:
  - **Title**: `feat(asset): manual fill for unupdated prices`
  - **Scope**: all layers (BE + FE + E2E + closure); terminates at the closure `/smart-commit` + land step.
  - **Dependency**: none.
  - **Branch**: `feat/manual-price-fill` (already created).

> Implementation is minimal: build only what makes the failing `test-writer-*` stubs pass —
> no defensive code, no anticipation of rules outside MKT-170–179.
