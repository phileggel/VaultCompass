# ADR 016 — Stooq supports an optional user-selected keyless fetch mode

**Date**: 2026-06-12
**Status**: Accepted — supersedes ADR-015

## Context

ADR-015 (2026-06-08) recorded that Stooq's surviving daily-download endpoint (`q/d/l/`) **requires a user-supplied API key**, and on that basis decided every price provider is BYOK-keyed with **no key-less automated default**. That conclusion rested on a live probe run from a single network during a day of heavy testing.

A later hands-on session surfaced a correction. From a residential Windows machine, `q/d/l/` served CSV **anonymously** — no key, only the proof-of-work browser-verification gate (L-005) — and the "Get your apikey: …" message did **not** appear. The key offer surfaces only **after the anonymous per-IP daily limit is exceeded**; under the limit, anonymous access works. Re-probing the same endpoint from a VPN/datacenter exit IP returned "Access denied" for the keyless request, an expired key, and a previously-valid key alike — i.e. the rejection was about the **request origin**, not the key (the same datacenter-IP blocking Yahoo exhibited on 2026-06-08).

So the real picture is network-dependent, not absolute: anonymous Stooq access works from some IP ranges and is blocked or rate-limited on others, while a stored key reverses that on yet others. A single global "keyed-only" decision strands whichever set of users the chosen mode doesn't fit. ADR-015's premise ("a key is mandatory") was too strong; this ADR records the corrected decision. The companion KEY spec (`docs/spec/api-key-management.md`, KEY-050–054) builds the user-facing surface.

## Decision

Make the Stooq fetch **dual-mode and user-selectable**, defaulting to keyed:

1. **Keyed (default)** — the ADR-015 BYOK path, unchanged: solve the proof-of-work **and** present the stored Stooq key on `q/d/l/`. The robust, recommended path; it is what existing users keep without any action.
2. **Keyless (opt-in)** — solve the proof-of-work and issue the **anonymous** `q/d/l/` request with no key. For networks where anonymous access works; subject to Stooq's per-IP daily limit.

Both modes solve the proof-of-work (L-005 is unchanged). The user chooses the mode with a device-local setting; the default stays keyed, so ADR-015's "no key-less **default**" survives — keyless is an explicit escape hatch, not the out-of-the-box behavior. Finnhub remains the documented keyed fallback and Manual the always-available override, both unchanged from ADR-015.

Alternatives considered:

- **Keep ADR-015 (keyed-only)** — rejected. Empirically anonymous access works on some networks; forcing a key there is needless friction, and worse, on networks where the keyed path's origin is itself blocked the user is stuck with no working fetch at all. The motivating session hit exactly this: anonymous worked on a home IP while the keyed probe was origin-blocked on a VPN.
- **Make keyless the default** — rejected. Anonymous access is rate-limited per IP and of uncertain reliability; the keyed BYOK path is the more dependable default and preserves ADR-015's onboarding story. Keyless is the fallback for constrained networks, not the baseline.
- **Auto-detect / auto-fallback (try keyless, fall back to keyed on failure)** — rejected for now. It adds hidden network probing and non-deterministic behavior; an explicit user toggle is predictable and matches the per-network reality the user already understands and controls. Auto-selection can be revisited if the manual toggle proves cumbersome.

## Consequences

- **Pros**: users on networks where anonymous Stooq works get price fetching with **zero key setup**; users on networks where only the keyed path works keep BYOK; the toggle adapts to the per-IP/VPN reality that no single global decision can capture. The correction also retires L-006's overstated "key is mandatory" claim and restores a zero-friction path for the common case.
- **Cons**: the fetch layer now carries two code paths selected by a mode flag threaded from the frontend; keyless is rate-limited and can fail per IP (surfaced through the existing fetch-outcome snackbar, never silently); there is a second user-facing setting to understand alongside auto-fetch; and the provider-chain story is now mode-conditional rather than a single line, so docs and future provider work must account for both modes.
