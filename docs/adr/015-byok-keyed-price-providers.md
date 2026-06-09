# ADR 015 — All asset-price providers are BYOK-keyed; there is no key-less default source

**Date**: 2026-06-08
**Status**: Accepted — supersedes ADR-008

## Context

ADR-008 chose Stooq as the **key-less** primary auto-fetch source, explicitly because "Stooq's no-key path lets the app produce useful results immediately for the typical user" — Finnhub (BYOK) was the keyed fallback, Manual the always-available override. That decision rested on Stooq being freely queryable without credentials.

That premise no longer holds. Through 2026 Stooq incrementally locked down programmatic access (documented in `docs/lessons.md` L-003 → L-005 → L-006): the formerly key-less light-quote endpoint (`q/l/`) now returns HTTP 404, and the surviving download endpoint (`q/d/l/`) requires an API key the user obtains from a captcha-gated signup page. The proof-of-work scraping mitigation built for L-005 targets a now-dead endpoint. As of v0.17.4 the app has **no working automated price source at all** until the user supplies a key.

This forces a decision: with Stooq no longer free, what is the provider chain, and is there still a "works out of the box" automated default? The companion KEY feature (`docs/spec/api-key-management.md`, trigram KEY) builds the credential surface; this ADR records the chain-level consequence.

## Decision

Treat **every** external price provider as BYOK-keyed. The chain remains, in order, **Stooq → Finnhub → Manual**, but its first tier changes character:

1. **Stooq** — primary auto-fetch source, now **requiring a user-supplied API key** (BYOK) on the surviving `q/d/l/` daily-download endpoint (the light `q/l/` quote endpoint that returned a single row is gone — it 404s even with a valid key). A live probe (2026-06-08) established that the apikey does **not** bypass Stooq's JavaScript proof-of-work browser-verification gate (L-005): a fresh request is challenged regardless of the key. The fetch path therefore **retains** the proof-of-work solver and **adds** the apikey — both are required. Because the download endpoint returns the full daily history rather than a single quote, the adapter reads the latest (last) row's close.
2. **Finnhub** — keyed fallback, unchanged from ADR-008 (free user-supplied key).
3. **Manual** — user-entered prices; always available, needs no key.

The consequence ADR-008 ruled out is now accepted: **there is no key-less automated default**. When no key is stored, automated fetch yields nothing for every symbol and the chain degrades to Manual entry only. Onboarding therefore requires the user to supply at least the Stooq key (or rely on Manual). Key storage, the Connections surface, and the refresh-gating that routes a key-less user to set one up all live in the KEY spec / ADR-011's storage ladder.

Alternatives considered:

- **Find another key-less provider to preserve zero-friction onboarding** — rejected. ADR-008 already surveyed the field and rejected the key-less options (Yahoo unofficial endpoints are actively blocked; the remaining reputable free APIs all issue keys). Chasing another unauthenticated source repeats the Stooq fragility that L-003→L-006 just demonstrated.
- **Drop Stooq, make Finnhub the primary** — rejected for now. Stooq's keyed daily-close endpoint has broad EU+US coverage (including Euronext Paris) that the app already depends on; Finnhub's free-tier EU coverage is uneven. Keeping Stooq primary preserves coverage; the key requirement is the only thing that changed.
- **Keep ADR-008 and treat the Stooq key as an implementation detail** — rejected. ADR-008's central rationale ("works without setup") is now false; leaving it Accepted would mislead any future reader into thinking a key-less default still exists. Superseding it makes the reversal explicit.

## Consequences

- **Pros**: a single current source of truth for the provider chain that matches reality; Stooq's EU+US coverage (including Euronext Paris) is preserved by keeping it primary; the BYOK key restores an authorized data path after the key-less `q/l/` endpoint was withdrawn.
- **Cons**: onboarding friction is now substantial — the user must obtain a Stooq key behind a captcha, and the fetch path must _both_ solve the proof-of-work challenge _and_ present the key; the zero-friction "first run shows live prices" story ADR-008 valued is gone (mitigated by KEY's refresh-gating that surfaces the Connections dialog on demand); the download endpoint returns the full daily history, so the adapter reads it under a raised body cap and takes the last row — heavier than a single-quote call (a date-range trim is a possible later optimization); free-tier Stooq keys appear tightly rate-limited and of uncertain lifespan, so a user who supplies no key — or whose key lapses — sees only Manual prices, which the failure snackbar must communicate clearly. These compounding costs are why **Finnhub is slated as the next provider** (a clean single-quote `/quote` call, no proof-of-work) once the BYOK surface lands.
