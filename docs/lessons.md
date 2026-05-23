# Lessons

Empirical failures the codebase has already paid for. Citable as `L-NNN` from commit messages, CLAUDE.md, reviewer findings.

Append-only; supersede in place if the underlying ecosystem changes.

---

## L-001 — Tauri NSIS bundler walks `src-tauri/src/bin/`

**First observed**: 2026-05-08 (v0.11.0 Windows release)
**Recurrences**: 2026-05-13 (PatientManager v0.18.0), 2026-05-20 (v0.12.0)
**Resolved by**: `243a184` (VaultCompass), `d79a245` (PatientManager)

**Symptom** — Windows release bundle fails with:
> `failed to bundle project when getting size of …/release/{name}.exe: The system cannot find the file specified.`

**Trigger** — A `.rs` file in `src-tauri/src/bin/` whose binary won't be in `target/release/` at bundle time. `[[bin]]` + `required-features` is the common way to trip it: cargo skips the build, bundler still expects the artifact.

**Root cause** — The NSIS bundler enumerates `src-tauri/src/bin/` on disk and expects every entry to produce a bundled `.exe`. It does not consult `Cargo.toml`'s `required-features` flag.

**Mitigation** — Dev-only binaries live in `src-tauri/dev/`, declared as `[[bin]] path = "dev/{name}.rs"`. Feature gating is orthogonal and does not solve the bundler problem.

**Guardrail** (optional CI lint):
```bash
[ -z "$(ls src-tauri/src/bin/ 2>/dev/null)" ] \
  || { echo "src-tauri/src/bin/ must be empty — see L-001"; exit 1; }
```

---

## L-002 — `taiki-e/install-action` tool versions float when unpinned

**First observed**: 2026-05-22 (v0.12.1 CI backend job)
**Resolved by**: this commit (pin `sqlx-cli@0.8.6` in `.github/workflows/ci.yml`)

**Symptom** — Backend CI fails instantly with:
> `error: \`--database-url\` or \`DATABASE_URL\` must be set`
>
> at `cargo sqlx prepare --check`, despite `SQLX_OFFLINE=true` being set in `src-tauri/.cargo/config.toml [env]`.

**Trigger** — `taiki-e/install-action` with `tool: <name>` (no `@version` suffix). Each CI run resolves to whatever's latest on the tool's GitHub releases at run time. A point release of the tool flips behaviour silently between two otherwise-identical pipeline runs.

**Root cause** — `sqlx-cli` 0.9.0 (released between v0.12.0 and v0.12.1 CI runs) requires `DATABASE_URL` for `prepare --check` even when `SQLX_OFFLINE=true`. v0.8.6 honoured the offline flag. The action SHA is pinned for supply-chain safety; the tool name is not.

**Mitigation** — Always pin tool versions to match the runtime dependency: `tool: sqlx-cli@0.8.6` matches `sqlx = "0.8"` in `Cargo.toml`. Bump deliberately when the dependency moves. Same discipline applies to any other tool installed via `taiki-e/install-action` (`cargo-tarpaulin`, etc.) — currently uses default; consider pinning when next surprise lands.

