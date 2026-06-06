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

---

## L-003 — Stooq serves an anti-bot challenge to clients without a browser `User-Agent`

**First observed**: 2026-06-05 (v0.17.0 prod, GH #69)
**Resolved by**: this commit (Stooq client sends a browser `User-Agent` + non-CSV body guard)

**Symptom** — Asset prices silently fail to update after a price fetch (FX rates update fine). Backend logs show, per symbol:

> `asset_price_fetch: provider fetch failed; skipping (MKT-114) … err=Stooq response parse failed … Caused by: close not numeric ("\"0\")).join(\"\");if(x.startsWith(t))break;n++}const r=await fetch(\"/__verify\""): invalid float literal`

Intermittent — a restart can appear to "fix" it.

**Trigger** — A Stooq CSV request (`stooq.com/q/l/?s=…&e=csv`) sent without a browser-like `User-Agent`. `reqwest` sends no `User-Agent` by default, so every Stooq request was exposed.

**Root cause** — Stooq returns a JavaScript proof-of-work anti-bot challenge page **with HTTP 200 and content-type `text/csv`** when it suspects a bot. The body is HTML/JS (`…const r=await fetch("/__verify")…`), not CSV. The status and content-type both look healthy, so the CSV parser ran and grabbed line 2 / column 6 of the _JavaScript_, failing the float parse with a misleading "close not numeric" error. The gate is heuristic, not purely UA-based, which is why a restart sometimes slipped through — masking the real cause. FX providers (Frankfurter/ECB) have no such gate, so they were unaffected.

**Mitigation** — Send a real browser `User-Agent` on the Stooq `reqwest::Client` (`STOOQ_USER_AGENT` in `stooq_client.rs`). Verified live: empty UA → challenge page; browser UA → CSV. Belt-and-suspenders: `parse_close_micros` rejects any body not starting with the `Symbol,Date,Time` CSV header up front, so a future challenge surfaces as a clear "non-CSV response (likely an anti-bot challenge page)" error instead of a float-parse red herring. **Content-type cannot be trusted to detect this — the challenge is also served as `text/csv`; gate on the body.**

---

## L-004 — `just sync-kit` ships a tab-indented `visual-proof-capture.mjs` that fails the project's biome

**First observed**: 2026-06-06 (sync to claude-kit v4.18.0)
**Resolved by**: `just format` after every sync (the kit-sync workflow already includes this step)

**Symptom** — Immediately after `just sync-kit`, `just check` / the pre-commit hook fails with a single biome **error** (amid unrelated pre-existing warnings):

> `× Some errors were emitted` — a formatter diff on `scripts/visual-proof-capture.mjs` (`- → await·…` tab vs `+ ··await·…` two spaces).

**Trigger** — Syncing a kit version whose `scripts/visual-proof-capture.mjs` is tab-indented, into this project whose `biome.json` sets `formatter.indentStyle: "space"` and includes `scripts/**`.

**Root cause** — The kit ships that `.mjs` with tab indentation, but the downstream biome config (which the kit also ships the convention for) mandates spaces. biome scans `scripts/*.mjs`, so the synced file is flagged. The change is **purely cosmetic** — reformatting to spaces reverts the file exactly to the prior version, so the fix is net-zero in the commit.

**Mitigation** — Run `just format` after `just sync-kit` (already a step in the kit-update workflow); it reformats the file to spaces and `just check` goes green. Self-healing but **recurs every sync** until the kit ships the file space-indented (or adds a biome override for `scripts/`). Don't burn time re-diagnosing — if biome errors on that one `.mjs` right after a sync, just run `just format`.
