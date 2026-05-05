# TODO

<!-- Add new tech debt and backlog items here. Format: ## (domain) — Short title -->

## (backend) — Consolidate transaction-recording command contracts

After the `use_cases/holding_transaction/` consolidation, two pre-existing contract inconsistencies remain in the moved commands. Surfaced during the cash-tracking spec review (2026-05-05); deliberately deferred from the consolidation refactor to keep that PR scope-minimal.

**1. Per-command error enums.** `buy_holding`, `sell_holding`, `correct_transaction`, `cancel_transaction` share one permissive `TransactionCommandError` enum whose variants are a union of every error any of them might emit. `account-contract.md` already documents per-command subsets (e.g. `cancel_transaction` only emits `TransactionNotFound`/`DbError`) but the type system can't enforce that. Split into `BuyHoldingCommandError`, `SellHoldingCommandError`, `CorrectTransactionCommandError`, `CancelTransactionCommandError` matching what the contract already claims. `open_holding` already follows this pattern. Frontend impact: error-handler types in gateway/forms regenerate.

**2. Parameter style.** `correct_transaction(id: String, account_id: String, dto: CorrectTransactionDTO)` and `cancel_transaction(id: String, account_id: String)` mix primitives + DTO; the rest are DTO-only. Move `id`/`account_id` into the DTOs for consistency. Frontend impact: gateway call sites change.

Work order: do (1) first; (2) is optional and lower value. (1) becomes notably more relevant once cash lands, because `InsufficientCash { current_balance_micros, currency }` would otherwise pollute the shared enum and bleed into `cancel_transaction`'s TS type even though it only fires from a replay-violation edge case.

## (backend) — Introduce dependency injection container for service wiring

`lib.rs` manually constructs and wires all repositories, services, and use cases in a single `block_on` closure. As the number of bounded contexts grows this becomes hard to maintain. Introduce a lightweight DI approach (e.g. a dedicated `AppContainer` struct or a builder pattern) to decouple service construction from app bootstrap, make the dependency graph explicit, and simplify testing of the wiring itself.

## (deps) — Upgrade reqwest to 0.13

`reqwest 0.12.28` is a major version behind (`0.13.3` available). Breaking changes: TLS default switches from native-tls to rustls+aws-lc; `query()`/`form()` are now optional features; several deprecated methods removed. Current feature flags (`rustls-tls-native-roots`, `json`) need review against the new defaults before upgrading. See `docs/dep-audit-2026-05-05.md`.

## (deps) — serialize-javascript CVE in @wdio/mocha-framework (GHSA-5c6j-r48x-rmvq, CVE-2026-34043)

`@wdio/mocha-framework@9.27.1` depends on `mocha` which pins `serialize-javascript <=7.0.4`. Two high-severity CVEs: RCE via RegExp.flags (GHSA-5c6j-r48x-rmvq) and CPU exhaustion DoS (CVE-2026-34043, fixed in 7.0.5). devDependency only — E2E test runner, not in the production bundle. Upstream fix tracked in [mocha#5872](https://github.com/mochajs/mocha/issues/5872). Do NOT run `npm audit fix --force` (downgrades @wdio to v6, breaking). Re-evaluate when mocha releases with serialize-javascript 7.0.5+.

## (deps) — Update specta to rc.23

`tauri-specta rc.21` pins `specta = "=2.0.0-rc.22"` (exact version). Wait for `tauri-specta rc.22+` before upgrading to `specta rc.23` + `specta-typescript 0.0.10`.
Status (2026-04-27): `specta rc.23` available, `tauri-specta` still blocked at `rc.21`.

## (backend) — Add required-features guard to generate_bindings bin

`src-tauri/Cargo.toml` `[[bin]] name = "generate_bindings"` has no `required-features` guard. Without it, `cargo build --all-targets` and `tauri-action` in CI both compile this dev-only codegen tool on every release build, wasting minutes. Fix: add a `[features]` section with a `generate-bindings` feature, set `required-features = ["generate-bindings"]` on the bin entry, and update the `generate-types` justfile recipe + `generate-types.sh` script to pass `--features generate-bindings`.

## (ci) — Standardise ubuntu runner version across all workflows

`coverage.yml` uses `ubuntu-latest` (currently resolves to 24.04 on GitHub-hosted runners) while `e2e.yml` and `release-manual.yml` (linux path) use `ubuntu-22.04`. If webkit2gtk or other apt packages behave differently between versions, coverage and e2e jobs may diverge silently. Pin all jobs to `ubuntu-22.04` until 24.04 compatibility is validated, then move all to 24.04 together.

## (ci) — Fix Cargo cache key in release-windows.yml

`release-windows.yml` uses `prefix-key: windows-rust-` with no `Cargo.lock` hash. A dependency update mid-release can silently reuse a stale cache. Add `${{ hashFiles('src-tauri/Cargo.lock') }}` to the prefix-key, matching the pattern already used in `coverage.yml` and `e2e.yml`.

## (ci) — Remove npm ci from security-audit.yml npm-audit job

`npm audit` (npm 7+) reads `package-lock.json` directly and does not require a full install. Removing the `npm ci` step and `cache: npm` from the npm-audit job in `security-audit.yml` cuts ~30 s of network I/O from every weekly run.

## (scripts) — Fix #!/bin/bash shebang in scripts/build.sh

`scripts/build.sh` uses `#!/bin/bash` instead of `#!/usr/bin/env bash`. This will fail on systems where bash is not at `/bin/bash` (NixOS, some BSDs). Change to `#!/usr/bin/env bash` to match all other scripts in the project.

## (deps) — Accepted risk: RUSTSEC-2023-0071 (rsa Marvin Attack)

`cargo audit` flags `rsa 0.9.10` (timing sidechannel, CVSS 5.9 medium) with no upstream fix. Pulled transitively via `sqlx-mysql 0.8.6` because the `sqlx` macro crate compiles all backends regardless of enabled features. We only enable `sqlite`, so the vulnerable RSA path is never reached at runtime. Re-evaluate when sqlx ships a fix or when we change DB backend.
