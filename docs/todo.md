# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (mkt) — Surface fetch-task completion to FE for end-of-task user feedback

`fetch_all_asset_prices` and `fetch_account_asset_prices` return synchronously on dispatch; per-asset results stream via `AssetPriceUpdated` events. The user has no signal for "task finished" — whether successfully, with partial failures, or with full provider outage. Per-asset failures are currently logged BE-side per MKT-114 with no FE surface; the task-level summary is the missing layer.

**Proposed**: emit a task-completion signal (e.g. `FetchTaskCompleted { scope, ok: u32, skipped: u32 }`) and an FE snackbar/banner that summarizes — "12 prices updated, 3 skipped". Distinct from `AssetPriceUpdated` (which is per-asset) and complements MKT-115 (which currently only covers dispatch-time feedback).

Surfaced 2026-05-17 during `/contract market-price` triage. Spec amendment to MKT-114 (or a new MKT-117+) will be needed.

## (spec) — Write KEY spec (User API Key Management)

ADR-011 captures the BYOK + OS keychain + 3-tier fallback decision. The spec to write — trigram `KEY` — covers the Tauri command surface, state machine, Connections settings panel UX, link-out to provider signup, "test connection" probe, and the Linux-without-keyring detection + UX flow.

Cross-cutting enabler: every current and future external-provider feature depends on this. First downstream consumers (in expected build order): Finnhub price fallback per ADR-008 (`/quote`), Finnhub ISIN ↔ ticker enrichment for `Asset` (see `(asset) — Auto-fill ISIN ↔ ticker via Finnhub` below — uses `/stock/profile2`), and the OpenFIGI free-key uplift that lifts the WEB lookup search rate from ~5/min to ~100/min.

Workflow-A: `/spec-writer api-key-management` → `/contract` → `feature-planner` → implementation. ~1-2 day feature.

Surfaced 2026-05-16 during the asset-valuation ADR thread.

## (spec) — Write FXR spec (Foreign Exchange Rate) — ✅ Done

ADR-009 introduces a new `CurrencyRate` entity, but no spec defines its bounded context, repository contract, or read path. Trigram `FXR`. Covers the entity definition, EUR-base + cross-rate computation algorithm, source qualifier (per ADR-010), Frankfurter primary + ECB XML fallback flow, manual entry CRUD, and the cache/refresh policy that parallels MKT's.

Prerequisite for PFD (cross-currency rollup needs current FX).

Workflow-A: `/spec-writer fx-rate` → `/contract` → `feature-planner` → implementation.

Surfaced 2026-05-16 by `adr-reviewer` after ADR-009 was written.

**Shipped** (PRs #63–#66): `currency` bounded context (manual rate CRUD + Frankfurter/ECB provider fetch), multi-currency valuation lift (foreign holdings now value into the account currency), and the Currency Rates view. Only the FX-staleness wiring (FXR-090) remains, tracked in `docs/techdebt.md`.

## (asset) — Auto-fill ISIN ↔ ticker via Finnhub (BYOK)

PR #41 shipped `Asset.isin` as an optional field separate from `reference` (the ticker). On the web-lookup ISIN path both fields populate from the user's query and the OpenFIGI ticker; on the keyword path only `reference` gets filled because OpenFIGI's free `/v3` endpoints do not expose ISIN in any response shape (verified live 2026-05-24). Manual creation similarly leaves the other field empty.

Once the KEY spec ships (BYOK + OS keychain per ADR-011), Finnhub's `/api/v1/stock/profile2?symbol={SYMBOL}&token={KEY}` becomes the reference enrichment path: the documented response includes both `ticker` and `isin`, so one call can fill the missing side. UX entry point: an "Auto-fill" affordance on the Add/Edit Asset form next to whichever identifier field is blank, triggered on demand by the user (not automatic) to keep call volume aligned with the ~3-5 adds/session pattern.

**Coverage caveat — must validate before committing:** Finnhub's free-tier coverage for European ETFs (especially Amundi's `FR0014…` range, the original motivating case) is documented in the field schema but not verified end-to-end. Cheapest validation is a one-shot curl against `/stock/profile2?symbol=PE500.PA&token=…` with a free Finnhub key (~30s signup). If coverage gaps surface, EODHD `/api/fundamentals` is the secondary candidate (similar key model, stronger European coverage, $20/mo for all-markets).

**Dependencies:** KEY spec (above) must ship first — Finnhub gates every endpoint behind a token, so the BYOK key-storage layer is a hard prerequisite (verified 2026-05-24: keyless calls return `401 — Please use an API key.`).

**Subsumes:** the legacy "(mkt) Stooq fetch by ISIN returns N/D" issue — once the enrichment path fills `reference` (ticker) when the user only supplied ISIN, Stooq fetches the resolved ticker and the original symptom disappears. The ISIN-based dedup question (deferred during PR #41 — see asset spec § Future features) is independent of this entry.

Surfaced 2026-05-24 (post-PR #41 follow-up).

## (spec) — PFD (Portfolio Dashboard) unblocked, no spec written

`docs/spec-index.md` lists PFD as `planning — paused — blocked on cash-tracking spec`. Cash-tracking shipped on 2026-05-06, so the blocker is lifted, but no `docs/spec/portfolio-dashboard.md` has been written yet. Next step when picked up: run `/spec-writer portfolio-dashboard` to author the cross-account aggregate-view spec (KPIs + per-account list, per the registry description), then the standard `/contract` → `feature-planner` flow. Update `docs/spec-index.md` to drop the "paused — blocked on cash-tracking spec" suffix at the same time.

## (backend) — `correct_transaction` / `cancel_transaction` parameter style

`correct_transaction(id: String, account_id: String, dto: CorrectTransactionDTO)` and `cancel_transaction(id: String, account_id: String)` mix primitives + DTO; the rest of the holding-transaction commands are DTO-only. Move `id`/`account_id` into the DTOs for consistency. Frontend impact: gateway call sites change. Surfaced during cash-tracking spec review (2026-05-05); per-command-error-enums concern from the original entry is subsumed by `docs/plan/error-model-refactor.md` PR 3.

## (backend) — Promote BC application services to traits, mock with mockall

`AccountService` and `AssetService` are concrete structs, so cross-BC orchestrators (`HoldingTransactionUseCase`, `ArchiveAssetUseCase`, `DeleteAssetUseCase`, `AccountDetailsUseCase`, …) cannot mockall-mock them and instead test against real services + in-memory SQLite. That's against the spirit of `docs/backend-rules.md` B34 ("Tests for services and orchestrators SHOULD mock external dependencies using mockall-generated mocks") — repositories already follow B34 via `#[cfg_attr(test, mockall::automock)]` on each domain.rs trait, but the service layer above them does not.

Extract a trait per service (e.g. `AccountServiceContract`, `AssetServiceContract`) listing the methods orchestrators call, annotate with `#[cfg_attr(test, mockall::automock)]`, and have orchestrators inject `Arc<dyn AccountServiceContract>` / `Arc<dyn AssetServiceContract>`. Then rewrite the orchestrator inline tests to use the generated `MockAccountService` / `MockAssetService` instead of `setup_pool` + real repositories — true unit isolation, faster, no DB dependency. Surfaced during PR #4 review (2026-05-06).

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

## (deps) — Upgrade reqwest to 0.13

`reqwest 0.12.28` is a major version behind (`0.13.3` available). Breaking changes: TLS default switches from native-tls to rustls+aws-lc; `query()`/`form()` are now optional features; several deprecated methods removed. Current feature flags (`rustls-tls-native-roots`, `json`) need review against the new defaults before upgrading. See `docs/dep-audit-2026-05-05.md`.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.
