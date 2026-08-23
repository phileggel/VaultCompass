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

**Update (2026-06-06)**: the browser-`User-Agent` mitigation no longer suffices — Stooq now serves the proof-of-work challenge to _every_ client regardless of `User-Agent`. See **L-005** for the escalation and the current mitigation (solve the proof-of-work in-client).

---

## L-004 — `just sync-kit` ships a tab-indented `visual-proof-capture.mjs` that fails the project's biome

**First observed**: 2026-06-06 (sync to claude-kit v4.18.0)
**Resolved by**: `just format` after every sync (the kit-sync workflow already includes this step)

**Symptom** — Immediately after `just sync-kit`, `just check` / the pre-commit hook fails with a single biome **error** (amid unrelated pre-existing warnings):

> `× Some errors were emitted` — a formatter diff on `scripts/visual-proof-capture.mjs` (`- → await·…` tab vs `+ ··await·…` two spaces).

**Trigger** — Syncing a kit version whose `scripts/visual-proof-capture.mjs` is tab-indented, into this project whose `biome.json` sets `formatter.indentStyle: "space"` and includes `scripts/**`.

**Root cause** — The kit ships that `.mjs` with tab indentation, but the downstream biome config (which the kit also ships the convention for) mandates spaces. biome scans `scripts/*.mjs`, so the synced file is flagged. The change is **purely cosmetic** — reformatting to spaces reverts the file exactly to the prior version, so the fix is net-zero in the commit.

**Mitigation** — Run `just format` after `just sync-kit` (already a step in the kit-update workflow); it reformats the file to spaces and `just check` goes green. Self-healing but **recurs every sync** until the kit ships the file space-indented (or adds a biome override for `scripts/`). Don't burn time re-diagnosing — if biome errors on that one `.mjs` right after a sync, just run `just format`.

---

## L-005 — Stooq escalated the anti-bot gate to a JavaScript proof-of-work challenge (User-Agent no longer enough)

**First observed**: 2026-06-06 (prod, GH #73)
**Supersedes the mitigation in**: L-003 (browser `User-Agent` no longer bypasses the gate)

**Symptom** — _Every_ asset price silently fails to update (FX rates still fine). Backend logs show, for every symbol:

> `asset_price_fetch: provider fetch failed; skipping (MKT-114) … err=Stooq response parse failed … Caused by: Stooq returned a non-CSV response (likely an anti-bot challenge page)`

Unlike L-003 this is **not intermittent** — it fails for all symbols, every launch.

**Trigger** — Any Stooq CSV request, regardless of `User-Agent`. Verified live 2026-06-06: empty UA, a Chrome UA, and the `stooq.pl` mirror all return the challenge page (HTTP 200) — the L-003 UA workaround is fully defeated.

**Root cause** — Stooq replaced the heuristic UA gate with a **JavaScript proof-of-work challenge** served to all clients. The page hands the client a challenge string `c` and difficulty `d` (=4), and requires finding an `n` such that `SHA-256(c + n)` starts with `d` hex zeros, POSTing `{c, n}` to `/__verify` (which sets an `auth` cookie), then reloading to receive the CSV. An HTTP-only client that doesn't solve the challenge only ever sees the HTML page.

**Mitigation** — Solve the proof-of-work in-client: on a non-CSV (challenge) response, parse `c`/`d`, brute-force the SHA-256 nonce (≈100k iterations, instant), POST `/__verify` with a `reqwest` cookie store enabled, then retry the original request with the now-set `auth` cookie. The cookie is reusable, so solve **once per app launch** and let every subsequent symbol ride the cookie — this also keeps request volume down (see surprises below). Verified live: solving the PoW yields real CSV (`MSFT.US,2026-06-05,…,416.67,…`).

**Surprises from the wider web (2026-06-06 research) — Stooq is actively tightening, treat as borrowed time:**

- **No official API, by design** — the CSV endpoint is an unsupported scrape of the web UI; no contract, no SLA. ([QuantStart](https://www.quantstart.com/articles/an-introduction-to-stooq-pricing-data/))
- **Low daily-hits quota** — returns `Exceeded the daily hits limit` after a relatively small number of requests/day; multi-symbol launch loops can trip it. Minimize requests (cookie reuse, no redundant fetches). ([AmiBroker forum](https://forum.amibroker.com/t/stooq-download-range-control-and-download-limit-violation/1167))
- **Drifting toward API keys** — as of March 2026 some users are served an HTML page directing them to _request an API key_ instead of CSV. ([pandas-datareader #1012](https://github.com/pydata/pandas-datareader/issues/1012)) Same direction as our own backlogged Finnhub-BYOK plan (ADR-008 + the unwritten KEY spec).

**Strategic note** — The PoW solver is a **short-term restoration**, not a durable fix. The daily cap + API-key drift mean the durable answer is a key-based provider (ADR-008 / KEY spec). Do not over-invest in the Stooq scrape; keep the fix scoped.

**Update (2026-06-08)**: borrowed time ran out — Stooq removed the quote endpoint entirely and now requires an API key. The PoW solver targets an endpoint that no longer exists. See **L-006**.

---

## L-006 — Stooq removed the free quote endpoint (404) and now requires a captcha-acquired API key

**First observed**: 2026-06-08 (prod, v0.17.4)
**Supersedes the mitigation in**: L-005 (the proof-of-work solver — the `q/l/` endpoint it targeted is gone)

**Symptom** — Every price fetch fails; FX rates still fine. Backend logs show, per symbol:

> `asset_price_fetch: provider fetch failed; skipping (MKT-114) … Stooq returned 404 Not Found for symbol …`

The frontend shows no error — the per-asset failure is silently skipped (MKT-114), so prices just stay stale and the user must read logs to discover it.

**Trigger** — Any request to the light-quote endpoint `stooq.com/q/l/?s=…&e=csv` (the one this app used).

**Root cause** — Stooq **retired the `q/l/` endpoint**: it now returns `HTTP 404` with the body _"The page you requested does not exist or has been moved"_. Verified live 2026-06-08 on both `stooq.com` and `stooq.pl`, with and without a browser `User-Agent`, with and without the `f=` param. The only surviving data endpoint, `q/d/l/` (daily download), responds (still behind the L-005 proof-of-work) with `Get your apikey: … https://stooq.com/q/d/l/?s=…&i=d&apikey=XXXX` — i.e. a **free API key is now mandatory**. This confirms the L-005 "drifting toward API keys" prediction; anonymous free scrape access is fully closed. The lockdown ladder: anonymous CSV → User-Agent gate (L-003) → JS proof-of-work (L-005) → endpoint removal + key requirement (this).

**Mitigation** — None without a key; the PoW solver cannot help (its endpoint is gone). The durable fix is the **KEY / BYOK** feature (ADR-011): the user obtains their own free Stooq key via the captcha page (`https://stooq.com/q/d/?s=spy.us&get_apikey`) and pastes it; the app then calls `q/d/l/?s=SYM&i=d&apikey=KEY` and takes the latest row's close. The key is free but **human-gated by a captcha**, so it cannot be automated — it is genuinely BYOK-shaped. Finnhub (ADR-008) is the documented fallback provider. Until BYOK ships there is no working price source; the immediate user-facing mitigation is to **surface the fetch failure on the frontend** instead of leaving prices silently stale.

**Correction (2026-06-12)**: the "a key is now mandatory" claim above was too strong. `q/d/l/` in fact still serves **anonymously** (proof-of-work only, no key) — the "Get your apikey" message appears only after the **per-IP anonymous daily limit** is exceeded; under the limit, anonymous access works. Whether it works at all is network-dependent (fine from some residential IPs, blocked/rate-limited from VPN/datacenter ranges). So both a keyed path and a keyless path exist; the right one depends on the user's network. [ADR-016](adr/016-stooq-optional-keyless-fetch-mode.md) records the resulting dual-mode decision (keyed default + optional keyless), superseding ADR-015.

**Update (2026-06-10) — resolved**: the KEY / BYOK feature shipped (PRs #77–#78, [ADR-015](adr/015-byok-keyed-price-providers.md) supersedes ADR-008). Two corrections to the paragraph above, both verified live during implementation: (1) the apikey does **not** replace the L-005 proof-of-work — `q/d/l/` requires **both** the PoW cookie and the apikey, so the solver was retained behind the shared `StooqGate`; (2) the throttle is a **per-key daily quota** (IP-independent), so the fetch was made windowed (`d1`+`d2`, ~10-day range → latest settled close) to keep payloads tiny and request volume polite. Price fetching works again end-to-end once the user pastes a key in the Connections dialog.

---

## L-007 — Local E2E green is not CI green when the code branches on a host service

**First observed**: 2026-06-11 (suite passed twice locally, failed on CI minutes after merge)

**Symptom** — An E2E spec green in repeated local headless runs fails on CI, on an assertion right after an action whose behavior depends on an OS service — here the keychain: with no Secret Service on the runner, the save fell back to a lower storage tier whose UI flow is legitimately different, and the spec had asserted the dev-host variant only.

**Mitigation** — (1) When a code path branches on host-service availability, assert only what is identical across all environment-legal variants (or accept any of them explicitly). (2) Before trusting local runs for such a path, reproduce the CI host: `DBUS_SESSION_BUS_ADDRESS=disabled: just test-e2e-headless` makes anything Secret-Service-dependent see "unavailable", exactly like CI. Generalizes to any host-coupled dependency — locale, display server, network: find the env knob that recreates the CI condition and run the suite under it. Fixed in `2091460`.

## L-008 — An external API's "access denied" can be origin-gated, not credential-gated

**First observed**: 2026-06-12 (a price provider that "needed a key" was actually blocking by IP)

**Symptom** — A read-only HTTP API returned access-denied for every request; the natural reading was "authentication required, get a key." Acquiring/sending a key changed nothing, because the gate keyed on the _request origin_ (IP/ASN allow-list), not on any credential. The same endpoint served data fine from a different network and rejected a valid key from a datacenter IP.

**Mitigation** — Before concluding an API needs auth, probe it from the _actual deployment network_ (a tool call's egress IP may differ from the user's — `[[feedback-bash-egress-uses-user-network]]`-style), and test the keyed and keyless requests from the _same_ origin to isolate the variable. When the gate turns out to be origin-based and no key fixes it, the credential machinery is wasted complexity: prefer a provider whose documented JSON endpoint is permissive over scraping/auth gymnastics. Drove the Stooq→Yahoo migration (ADR-017): a stable keyless JSON endpoint replaced a whole BYOK feature.

## L-009 — Font metrics differ between local and CI, so borderline flex layouts fail only in CI

**First observed**: 2026-07-04 (account-details header overflow → "element click intercepted" in 3 E2E specs)

**Symptom** — Repeated local headless E2E runs green; the same suite red on CI with WebDriver "element click intercepted" on buttons in a dense flex row. Same app, same window size, same xvfb wrapper. The variable was the runner's installed fonts: wider fallback glyph metrics pushed a borderline `whitespace-nowrap` stats block into overlapping the sibling button group — locally the same row fit by a few pixels.

**Mitigation** — (1) Treat "element click intercepted" appearing across several unrelated specs as a layout-overflow signal, not per-spec flakiness — look for what the failing clicks share spatially (here: all targets lived in one header row). (2) Make dense rows wrap-tolerant (`flex-wrap` on the row and its groups) instead of relying on the current viewport fitting; nowrap text inside a `min-w-0` flex child overflows _over_ siblings rather than clipping. (3) Gate a release tag on the CI E2E run of the merge commit, not only on local E2E — text-metric-sensitive layouts are exactly the class of breakage only CI reveals.

## L-010 — A CI job timeout must budget the cold cache-miss build, not the warm one

**First observed**: 2026-07-13 (PR #94 backend coverage job cancelled at 30m with every test green)

**Symptom** — A coverage job that runs ~21 minutes on a warm dependency cache was cancelled at its 30-minute limit; the log showed tests passing steadily right up to the cutoff, plus orphaned tooling processes at cleanup — which reads like a hang but is just whatever was mid-flight when the axe fell. The trigger: the PR changed the lockfile (new dependency + a feature flag on an existing one), invalidating the dependency cache and forcing a cold instrumented rebuild.

**Mitigation** — (1) Before diagnosing a "hung" CI job, check whether steady progress was still being logged at cancellation — a timeout mid-progress is a budget problem, not a deadlock. (2) Size `timeout-minutes` for the cold-cache path (lockfile changes are routine), keeping headroom of roughly the warm duration's half. (3) A local run of the same tool over the suspect tests separates "genuinely hangs" from "ran out of time" in minutes.

## L-011 — Before bisecting a local-only E2E failure, run a known-green tag on the same machine

**First observed**: 2026-08-23 (multi-device sync PR-E: every E2E spec that writes through IPC timed out locally while CI on the same commit was green)

**Symptom** — Write commands invoked from E2E (`execute/async` seeds, a form submit) never resolved locally: WebDriver script timeouts, a modal stuck in its submitting state. Backend probes showed the command completing in milliseconds; read-only specs passed. It looked like a regression in the freshly merged feature, and an hour went into instrumenting it.

**Mitigation** — (1) When CI is green on the same commit, first run one E2E spec from a known-green release tag in a `git worktree` on the same machine; if it fails the same way, the environment is the variable (here: the local WebKitGTK/driver stack losing IPC responses under load) and the bisect is pointless. (2) Kill stale drivers with `pkill -x <name>`, never `pkill -f <pattern>` — the pattern matches the shell running the command and kills it (exit 144), silently skipping everything after it. (3) Gate the merge on the CI E2E run (the suite runs on the main push) and fix forward if it reddens.
