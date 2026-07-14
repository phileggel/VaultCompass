# Next-Batch Plan — post-SPF, pre-release (2026-07-14)

Branch `next`. One commit per task at its close (plan-first-commit pattern). Release follows the batch.

## Tasks

### T0 — Kit sync v5.0.0 → v5.1.0

Both filed issues shipped upstream (phileggel/claude-kit#85 F26 rewrite, #86 titles-only changelog). Sync, then: verify `scripts/release.py` converges with our local titles-only patch (drop the patch-tracking todo), confirm `docs/frontend-rules.md` carries the rewritten F26 (close the F26 todo; trim the project-side cross-feature rule copy from CLAUDE.md § Standards), `/kit-discover` if the delta is non-trivial.

### T1 — CTO account: yearly performance rows missing (bug/investigation)

User report: the CTO account (EUR) shows no yearly performance although transactions were retro-added back to 2019; several holdings are USD-denominated. Hypotheses to check first: (a) period valuation bails when no FX rate exists at historical period ends (FXR resolution for 2019–2024 dates) so year rows are dropped; (b) windowed/OB handling (PRF-086) interacts with retro-added history; (c) year-row gating on some `None` value. Reproduce with a seeded EUR account + USD asset + 2019 transactions; fix at the root; regression test named for the rule it pins.

### T2 — Market-price global view not scrollable (bug)

Older market-price entries cannot be reached to correct them. Identify the view (price-history surface / currency-rates-style list) and give the list a bounded, scrollable container consistent with sibling views; visual proof both themes; E2E only if a selector exists to pin.

### T3 — Dividend entry in account currency for a foreign-currency asset (feature)

Today a dividend for a USD asset cannot be typed directly in euros. Extend the DIV spec (new DIV-0xx rules): an account-currency entry mode (the credited cash is account-currency anyway), explicit about what is stored and how the attributed asset's dividend total is reported. Spec addendum → contract touch-up if the wire changes → BE → FE → tests.

### T4 — DI container for service wiring (todo)

`lib.rs` builds every repository/service/use case in one block_on closure. Introduce an `AppContainer` (or builder) under `src-tauri/src/` that owns construction; `lib.rs` setup and `scheduled_fetch/headless.rs` both consume it (the headless wiring duplication is the proof-of-value). No behavior change; wiring unit-testable.

### TD1 — SPL/HNO `tests/` integration parity (techdebt 2026-07-11)

Add `src-tauri/tests/split_crud.rs` + `holding_note_crud.rs` mirroring `free_shares_crud.rs` (real SQLite wiring). Mechanical.

### TD2 — `correct_transaction` splice rollback (techdebt 2026-07-11)

Snapshot/restore the replaced transaction on the error path in `context/account/domain/account.rs::correct_transaction`, aligning with `apply_split`/`apply_free_shares`. Small + test.

### TD3 — Cash-OB perf-bridge guard (techdebt 2026-07-06)

Reject a cash-line OpeningBalance in the domain (the latent −(typed cost) pnl distortion becomes unreachable by construction). Guard + test; close the techdebt entry.

## Closure

Housekeeping commit: remove this plan, close the three techdebt entries + the two kit todos, update ARCHITECTURE.md if T4 adds a module. Then the release sequence (separate step, user-driven): manual systemd check (SPF), `/dep-audit`, E2E, `just release --preview`.
