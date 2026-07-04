# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (spec) — INT: per-asset interest-bearing eligibility flag

Interest (INT, shipped 2026-07-04) currently accepts any held asset plus the cash line as a credit target. Real-world eligibility is narrower: the cash line and cash-like fund lines ("fonds en euros" — a dedicated asset line that behaves like cash). Asset class is not a reliable signal, so the precise design is a per-asset `interest_bearing` opt-in flag (migration + Asset field + checkbox on the asset form + filter on the interest modal's selector — the same shape as the account-level `management_fees_enabled` gate). Deliberately deferred: single-user app, the open selector's misuse cost is low. Pick up when the selector gets noisy or a second eligibility consumer appears.

## (spec) — PFD (Portfolio Dashboard) unblocked, no spec written

`docs/spec-index.md` lists PFD as `planning — paused — blocked on cash-tracking spec`. Cash-tracking shipped on 2026-05-06, so the blocker is lifted, but no `docs/spec/portfolio-dashboard.md` has been written yet. Next step when picked up: run `/spec-writer portfolio-dashboard` to author the cross-account aggregate-view spec (KPIs + per-account list, per the registry description), then the standard `/contract` → `feature-planner` flow. Update `docs/spec-index.md` to drop the "paused — blocked on cash-tracking spec" suffix at the same time.

## (backend) — Promote BC application services to traits, mock with mockall

`AccountService` and `AssetService` are concrete structs, so cross-BC orchestrators (`HoldingTransactionUseCase`, `ArchiveAssetUseCase`, `DeleteAssetUseCase`, `AccountDetailsUseCase`, …) cannot mockall-mock them and instead test against real services + in-memory SQLite. That's against the spirit of `docs/backend-rules.md` B34 ("Tests for services and orchestrators SHOULD mock external dependencies using mockall-generated mocks") — repositories already follow B34 via `#[cfg_attr(test, mockall::automock)]` on each domain.rs trait, but the service layer above them does not.

Extract a trait per service (e.g. `AccountServiceContract`, `AssetServiceContract`) listing the methods orchestrators call, annotate with `#[cfg_attr(test, mockall::automock)]`, and have orchestrators inject `Arc<dyn AccountServiceContract>` / `Arc<dyn AssetServiceContract>`. Then rewrite the orchestrator inline tests to use the generated `MockAccountService` / `MockAssetService` instead of `setup_pool` + real repositories — true unit isolation, faster, no DB dependency. Surfaced during PR #4 review (2026-05-06).

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.

## (deps) — Accepted risk: RUSTSEC-2026-0185 (quinn-proto, not compiled)

`cargo audit` flags `quinn-proto 0.11.14` (RUSTSEC-2026-0185, remote memory exhaustion via unbounded out-of-order stream reassembly, 7.5 high, fixed in ≥0.11.15). It is only an **optional** dependency of `reqwest 0.13.3` behind the `http3` feature, which is **not enabled** — `cargo tree -i quinn-proto` is empty, confirming it is not compiled into the shipped binary. Non-applicable; flagged at the v0.28.0 release. The v0.29.0 T6 reqwest 0.13 upgrade did **not** prune it (the earlier expectation was wrong): `quinn` is reqwest 0.13's own optional `http3` dependency, resolved into `Cargo.lock` regardless of activation but never compiled. It will only clear if reqwest drops the optional `quinn` entry upstream. Re-evaluate if `http3` is ever enabled.
