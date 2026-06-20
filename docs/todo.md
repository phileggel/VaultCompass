# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (feature) — Eager cash line at account creation (no more lazy creation)

Today the Cash Holding is lazily created on the first cash-credit transaction (CSH-012) and auto-deleted when its balance returns to zero (CSH-013); a freshly created account has no cash row and shows the "No cash recorded yet" banner (CSH-095). The desired behaviour: every account gets its Cash Holding **at creation time, seeded at quantity 0**, and the cash row is **always visible** with its own Deposit/Withdraw/History actions. Consequently the header "Record" dropdown's **Deposit / Withdraw** entries are removed (the cash row owns those actions); the no-cash banner and the hide-at-zero rule go away.

Decided approach (confirmed 2026-06-20):

- **Real domain eager-create** (not a frontend display-only row): persist a 0-balance Cash Holding at account creation, and the cash line is **never auto-deleted**.
- **Backfill via migration**: every existing account gets a 0-balance Cash Holding (and the per-currency Cash Asset / `system-cash-category` rows) if absent.

Spec amendments required (cash-tracking CSH): CSH-010 (seed Cash Asset at account creation, not lazily), CSH-012 (eager create at qty 0), CSH-013 (never auto-delete — drop the TRX-034 cash cleanup), CSH-019 / DIV-012 (remove header-menu Deposit/Withdraw), CSH-022(b)/CSH-024/CSH-034/CSH-090 (reword — holding always present), CSH-095 (remove no-cash banner), CSH-097 (always show cash row). Also account.md (ACC: creation seeds cash) and account-details.md (ACD: cash row always rendered).

Backend approach: new `use_cases/account_creation/{api,orchestrator,mod}.rs` (cross-context, mirrors `account_deletion`/`holding_transaction`), injecting `AccountService` + `AssetService`; the `add_account` command moves there keeping its exact signature (so `bindings.ts` and the FE gateway are unchanged). `Account::new` seeds the 0-qty cash holding; `replay_cash_holding` drops its CSH-013 delete branch. Frontend: remove the two add-menu cash buttons + delete `NoCashBanner.tsx`; cash row always rendered. Open question to settle when picked up: SQL backfill migration vs idempotent Rust startup backfill. Full plan was drafted in conversation on 2026-06-20.

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
