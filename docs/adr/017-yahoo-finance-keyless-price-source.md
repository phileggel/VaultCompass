# ADR 017 — Yahoo Finance is the sole keyless price source; BYOK is retired

**Date**: 2026-06-12
**Status**: Accepted — supersedes ADR-011, ADR-016

## Context

The asset-price provider chain has churned three times: ADR-008 chose key-less Stooq primary with a Finnhub-BYOK fallback; ADR-015 reversed that to "every provider is BYOK-keyed, no key-less default" after Stooq locked down; ADR-016 corrected ADR-015 to a dual-mode (keyed default / keyless opt-in) toggle once anonymous Stooq turned out to be network-dependent rather than universally blocked.

That dual-mode shipped in v0.19.0 and then failed in the field. For the primary user, **neither** Stooq mode works: the keyless path is denied (their network's IP is not on Stooq's anonymous allow-list) and the keyed path is unreachable too — the Stooq signup is captcha-gated and no key could be obtained. ADR-016's premise was that _some_ mode fits _some_ network; the user is stranded in the gap where neither does, with no automated price source at all. The proof-of-work solver, the BYOK key surface, the keychain storage ladder (ADR-011), and the dual-mode toggle are now substantial machinery serving a provider that does not function.

ADR-008 surveyed key-less alternatives in 2026 and **rejected Yahoo Finance** outright, citing active blocking of unofficial endpoints (URL rotation, CAPTCHA gates, IP throttling). That rejection is the reason Yahoo was never adopted. A fresh empirical probe (2026-06-12) revises the picture for the specific endpoint that matters: `https://query1.finance.yahoo.com/v8/finance/chart/{symbol}` returns structured JSON, needs no API key and no cookie/"crumb" handshake, and **served data from the same VPN/datacenter exit IP that Stooq denied on the same day**. ADR-008's concern was real for the `/v7/quote` endpoints and for high-volume scrapers; the `/v8/chart/` endpoint under VaultCompass's usage profile (a cached on-launch burst of ~5–50 requests/day) is a materially different and currently-accessible case.

## Decision

Adopt **Yahoo Finance's `/v8/finance/chart/` JSON endpoint as the sole automated price source**, key-less, and **retire the entire BYOK/API-key feature**.

- `AssetPriceSource` collapses to `Manual | YahooFinance`. Stooq and Finnhub variants are removed.
- The Stooq adapter, its proof-of-work gate, and the Stooq symbol/exchange mappers are deleted. The price-fetch path no longer threads a key or a fetch-mode flag.
- The BYOK surface is removed wholesale: the connection bounded context and its commands, the Connections UI, the "use API key" setting, the refresh-time key gate, and the OS-keychain storage ladder of ADR-011. With no provider requiring a credential, none of it has a consumer.
- Yahoo symbols are derived per venue: bare ticker for US (`AAPL`), exchange suffix elsewhere (`VOD.L`, `BMW.DE`, `MC.PA`) — replacing Stooq's `.us` / class-share scheme.
- An unknown symbol (Yahoo returns HTTP 200 with `chart.error.code = "Not Found"`) maps to a typed not-found outcome, not a hard error.

Alternatives considered:

- **Keep ADR-016's dual-mode Stooq** — rejected. Both modes fail from the motivating user's network and the key is unobtainable; maintaining a proof-of-work solver plus a BYOK surface for a dead provider is cost with no benefit.
- **A keyed API (Finnhub / Alpha Vantage / Twelve Data)** — rejected. Each reintroduces exactly the key-acquisition friction this change exists to eliminate, and the BYOK machinery to manage it. Alpha Vantage's 25 req/day free tier is also incompatible with multi-holding portfolios (ADR-008).
- **Literal HTML scraping** (Google/Investing.com) — rejected. Brittle against markup changes, trips anti-bot defenses, and raises ToS concerns. The `/v8/chart/` JSON endpoint gives the same "no key" benefit with structured data.
- **Keep ADR-008's standing rejection of Yahoo** — rejected. It was evidence-based for its time and endpoint; the contrary empirical result for `/v8/chart/` under this app's low-volume profile supersedes it. The residual risk (Yahoo may rotate or block later) is accepted and isolated behind the `AssetPriceSource` enum + a single adapter, so a future provider swap stays local.

## Consequences

- **Pros**: zero-friction onboarding is restored — first launch shows live prices with no key, no captcha, no proof-of-work; ~23 files of BYOK complexity (connection BC, PoW solver, key UI, keychain ladder, dual-mode toggle) are deleted, not maintained; data arrives as structured JSON (not scraped HTML) with current price, currency, and daily history in one response; broad global coverage including EU venues.
- **Cons**: the app now depends on an **undocumented** Yahoo endpoint that can change shape, rotate, or rate-limit without notice — precisely the fragility ADR-008 feared, now accepted deliberately and confined to one adapter; **pence-quoted venues are a correctness hazard** — Yahoo reports LSE prices in `GBp` (and `ZAc`, `ILA` elsewhere), so the adapter must normalize to the major ISO unit (`÷100`) at the boundary or valuations against GBP holdings break; multi-provider / BYOK extensibility is gone — re-adding a keyed provider later means rebuilding the connection surface from git history; and if Yahoo ever blocks the user's IP, only Manual entry remains (no provider fallback survives this simplification).
