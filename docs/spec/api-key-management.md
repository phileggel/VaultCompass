# Business Rules — API Key Management (KEY)

> **⛔ RETIRED — superseded by [ADR-017](../adr/017-yahoo-finance-keyless-price-source.md).** The price provider moved to keyless Yahoo Finance; no provider requires an API key, so the entire BYOK/KEY feature (connection bounded context, Connections dialog, key storage, fetch-mode toggle, refresh key-gate) is being removed. **Every KEY-NNN rule below is inactive** — do not treat them as constraints. This file is retained only until the migration PR deletes it alongside the implementation.

## Context

VaultCompass fetches asset prices from external providers (see [ADR-016](../adr/016-stooq-optional-keyless-fetch-mode.md), which supersedes ADR-015 → ADR-008). The **bring-your-own-key (BYOK)** path is the default and the robust option: the user supplies a free Stooq key obtained from the provider's signup page (see `docs/lessons.md` L-006). But Stooq's daily-download endpoint also serves **anonymously** — subject to a per-IP daily limit — and whether anonymous access works depends on the user's network (it works from some IPs, is blocked or rate-limited on others). The app therefore also offers an optional **keyless** fetch mode (KEY-050). With a key (the default) or in keyless mode, the app has a working automated price source; the two modes are the user's lever for whichever path their network allows.

This feature gives the user a place to supply, test, store, and remove those keys, following the **bring-your-own-key (BYOK)** model: VaultCompass never bundles a key and never transmits a key anywhere except the provider it belongs to. Keys live only on the user's machine, stored as securely as the host allows via the storage-tier ladder defined in [ADR-011](../adr/011-byok-api-keys-os-keychain.md).

The first slice covers **Stooq only**. The surface is built so additional providers (Finnhub per ADR-016, OpenFIGI per the WEB spec) slot in as further rows later without rework.

This is a **feature spec**. Key storage is a cross-cutting infrastructure concern consumed by the asset price-fetch path (MKT). It is surfaced to the user through a **Connections** dialog reached from the side menu, and it gates the price-refresh actions defined in `docs/spec/market-price.md` (MKT-130/131).

---

## Entity Definition

### ProviderApiKey

Represents a single external provider's API key as the user manages it. The secret value is **not** stored in the application database — it lives in the OS keychain, in session memory, or in an opt-in plaintext file per the storage-tier ladder (KEY-011). Only the non-secret state below is ever surfaced to the user interface.

| Field         | Business meaning                                                                                                                                                                  |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `provider`    | Which external provider this key authenticates (e.g. `Stooq`). Identifies the row in the Connections dialog and the fetch path that consumes the key.                             |
| `has_key`     | Whether a key is currently stored for this provider. Derived state surfaced to the UI so it can show "Key set" vs "No key". The secret value itself is never read back to the UI. |
| `active_tier` | Where the stored key currently lives: OS keychain, session memory, or opt-in plaintext file (KEY-011). Surfaced so the user always knows where their key resides (KEY-015).       |

> The secret value is write-only from the UI's perspective: the user can set it, test it, and remove it, but the application never returns a stored key to the frontend (KEY-018). Testing (KEY-021) is the one inbound flow that carries a secret: the user's candidate key travels UI → backend for a one-shot probe and is never persisted by that path.

---

## Business Rules

### Storage and Lifecycle (010–019)

**KEY-010 — Save a provider key (frontend + backend)**: The user supplies a key for a provider by pasting it into the provider's row in the Connections dialog and confirming. The key is persisted via the active storage tier (KEY-011) and the save is acknowledged. A blank or whitespace-only key is rejected with a specific error and nothing is stored. Saving when a key already exists for the provider replaces it: the prior value is first cleared from every tier (per KEY-013) and the new value written to the selected tier, so no stale key survives in a lower tier.

**KEY-011 — Storage-tier ladder (backend)**: A key is stored using the first available tier, in priority order per ADR-011: (1) the OS keychain; (2) session-only in-memory storage when the keychain cannot be initialised; (3) an opt-in plaintext file. The tier in use for a given key is its `active_tier`. The selection is determined at runtime from host capability and the user's tier-3 opt-in (KEY-012). Session memory (tier 2) is the guaranteed in-process floor: when the keychain is unavailable and the user has not opted into plaintext, the key is always stored for the current session, so a save never fails for lack of an available tier. A write that does fail on the selected tier (e.g. the keychain is present but errors, or an opted-in plaintext write fails) surfaces a specific storage error rather than silently losing the key.

**KEY-012 — Plaintext tier requires explicit opt-in (frontend + backend)**: Storing a key in the plaintext-file tier (tier 3) is never silent. It is offered **only when the OS keychain (tier 1) is unavailable** — as a persistent alternative to the session-memory fallback (tier 2) for users who want the key to survive a restart on a keyring-less host — and proceeds only after the user confirms a warning acknowledging that the key will be readable on disk. When the keychain is available, tier 3 is not offered (there is no path to elect plaintext over a working keychain). Without the confirmation the key is not written to a file; it remains in session memory for the current session.

**KEY-013 — Removing a key clears every tier (frontend + backend)**: Removing a provider's key clears it from the OS keychain, session memory, **and** the plaintext file — not only the tier currently in use. This prevents a stale key in a lower tier from resurrecting after removal. After removal, `has_key` is false for that provider.

**KEY-014 — Keys never appear in diagnostics (backend)**: A key value is never written to logs, error messages, crash reports, or any telemetry. Diagnostic output about key operations refers to the provider and outcome only, never the secret.

**KEY-015 — Active tier is surfaced (frontend)**: For each provider that has a key, the Connections dialog shows which storage tier the key lives in (KEY-011), so the user always knows where their key resides.

**KEY-016 — Key-set status is surfaced (frontend + backend)**: The application can report, per provider, whether a key is currently stored (`has_key`) without exposing the value. The Connections dialog uses this to show "Key set" vs "No key", and the refresh-gating check (KEY-040) uses it to decide whether to dispatch a fetch. A failure to read stored-key status (e.g. the keychain errors at query time) is reported as a specific storage error, **not** as `has_key = false` — a read fault is never silently mistaken for "no key", so the refresh gate (KEY-040) does not wrongly route the user to set up a key that already exists.

**KEY-017 — Session-memory keys do not persist (backend)**: A key held in the session-memory tier (tier 2) is cleared when the application exits and is not written to disk. On the next launch the provider shows "No key" until the user pastes it again.

**KEY-018 — Stored key is never returned to the frontend (backend)**: The application never reads a stored key back to the frontend. The frontend only ever observes `has_key` and `active_tier`. Editing a key means overwriting it with a freshly pasted value, not retrieving and displaying the existing one.

### Testing a Key (020–029)

**KEY-020 — Test-key action (frontend)**: Each provider row in the Connections dialog offers a "Test key" action; its probe semantics are defined in KEY-021. The action is enabled only when the row's key field holds a non-empty value; with an empty field it is disabled.

**KEY-021 — Test probe (backend)**: Testing performs a minimal live request to the provider using the supplied key and reports whether the provider accepted it. The probe uses a fixed, well-known symbol the provider is guaranteed to cover (rather than a symbol drawn from the user's holdings), so the test is deterministic and works even before the user has any derivable holdings. The test operates on the key value currently entered in the dialog, allowing the user to verify a freshly pasted key before saving.

**KEY-022 — Test does not change stored state (backend)**: Running a test neither stores, replaces, nor removes any persisted key. It is a read-only check; the stored key (if any) is untouched regardless of the test outcome.

**KEY-023 — Test feedback (frontend)**: The test result is shown inline in the provider row: a success state when the provider accepted the key, a distinct invalid-key state when the provider rejected it, and a distinct unreachable state when the provider could not be contacted (network failure). The three outcomes are visually distinguishable so the user knows whether to fix the key or retry later.

### Connections Dialog (030–039)

**KEY-030 — Side-menu entry point (frontend)**: A "Connections" entry in the side menu opens the Connections dialog. It is reachable from anywhere in the app.

**KEY-031 — Provider list (frontend)**: The Connections dialog lists the supported providers, one row each. In this slice the list contains a single row: Stooq. The layout accommodates additional provider rows without redesign.

**KEY-032 — Provider row contents (frontend)**: Each provider row shows the provider name, the key-set status (KEY-016), the active storage tier when a key is set (KEY-015), a field to paste or replace the key, a "Test key" action (KEY-020), and a "Remove key" action (KEY-013). A link to the provider's key-signup page is shown so the user knows where to obtain a key.

**KEY-033 — Save feedback (frontend)**: On a successful save (KEY-010) the row updates to show "Key set" with its active tier, and a snackbar confirms the key was saved. On a rejected save (blank key, or a storage failure) an inline error is shown in the row and the prior state is preserved.

**KEY-034 — Remove confirmation (frontend)**: The Remove action is shown only when a key is stored for the provider (`has_key` is true); it is absent on a "No key" row. Removing a key (KEY-013) requires explicit confirmation identifying the provider. On confirmed removal the row returns to "No key" and a snackbar confirms the removal.

**KEY-035 — In-flight state (frontend)**: While a save, test, or remove request is in progress, the corresponding action in the row is disabled and shows a progress indicator to prevent double-submission.

### Price-Fetch Gating (040–049)

**KEY-040 — Refresh gated on a key (frontend)**: When the user triggers a price refresh (global refresh MKT-130 or account refresh MKT-131) and no key is stored for the price provider, the refresh is not dispatched; instead the Connections dialog opens, focused on the provider that needs a key. Once a key is saved the user can trigger the refresh again. This replaces dispatching a fetch that would fail for lack of a key.

**KEY-041 — Launch auto-fetch is skipped without a key (frontend)**: The launch auto-fetch (MKT-121), when enabled but no provider key is stored, is silently skipped: the frontend reads the key-set status (KEY-016) and, finding no key, does **not** dispatch the fetch task and makes no provider request. Unlike the explicit refresh (KEY-040), it does **not** open the Connections dialog — a cold start is never interrupted by a setup prompt. Because no fetch task runs, no fetch-outcome snackbar (MKT-145) fires; the absence of fetched prices surfaces only through the per-holding "no price" diagnostics (MKT-032). The Connections dialog opens only from an explicit user-clicked refresh (KEY-040) or the side-menu entry (KEY-030), never automatically on launch.

**KEY-042 — Manual price entry is never gated (frontend)**: Manual price actions — "Enter price" (MKT-010) and price-history add/edit/delete (MKT-070+) — remain fully available whether or not a key is stored. They write user-typed values and make no provider request, so a missing key never blocks them.

**KEY-043 — Stooq fetch uses the stored key (backend)**: The Stooq price-fetch path authenticates with the stored Stooq key on the surviving daily-download endpoint and reads the latest available daily close for each symbol. The apikey does not by itself bypass Stooq's proof-of-work browser-verification gate, so the path solves that challenge (as it already did) **and** presents the key — both are required in keyed mode (see [ADR-016](../adr/016-stooq-optional-keyless-fetch-mode.md)). As the download returns the full daily history, the latest (last) row's close is taken. The observation-date handling defined in MKT-117/118 is unchanged — the provider's quote date dates the recorded price, falling back to today when absent or invalid.

**KEY-044 — No key means no fetched price in keyed mode (backend)**: **When the fetch mode is keyed (KEY-050)** and no Stooq key is stored, a dispatched fetch task detects the absent key **once** at task start and skips the entire scope without issuing any per-asset provider call — every asset is reported as skipped per MKT-119 (`ok = 0`). It does not make N doomed authenticated calls. This is a separate, earlier check than the empty-scope rejection (MKT-111, which fires only when no derivable holding exists at all — symbol derivation per MKT-110 does not depend on a key). In keyed mode both frontend paths already avoid dispatching in this state (the refresh gate KEY-040 and the launch skip KEY-041); this rule guarantees backend correctness if a dispatch occurs regardless. **This short-circuit does not apply in keyless mode (KEY-053)** — there the absent key is intentional, and the fetch proceeds anonymously.

### Fetch Mode — Keyed vs Keyless (050–059)

**KEY-050 — Stooq fetch-mode setting (frontend)**: The user can choose how Stooq price fetches authenticate, via a **"Use Stooq API key"** setting alongside the existing auto-fetch preference (MKT-120). It is a device-local preference that persists across sessions and defaults to **on (keyed)** — the BYOK behavior of KEY-040/043/044 is the default and is unchanged for existing users. When **off (keyless)**, fetches use Stooq's anonymous daily-download path: no key is required or sent. The setting exists because anonymous access works from some networks/IPs while others are blocked or rate-limited, and a stored key reverses that on yet others — the user keeps both paths and switches to whichever their network allows.

**KEY-051 — Keyless mode bypasses the refresh gate (frontend)**: When the fetch mode is keyless (KEY-050 off), the price-refresh key gate (KEY-040) does **not** apply: triggering a global or account refresh dispatches the fetch directly, and a missing key never opens the Connections dialog. The Connections dialog remains reachable from the side menu (KEY-030) so the user can still save or test a key for later use in keyed mode.

**KEY-052 — Keyless mode does not skip launch auto-fetch (frontend)**: When the fetch mode is keyless (KEY-050 off), the launch auto-fetch no-key skip (KEY-041) does **not** apply: if auto-fetch on launch is enabled (MKT-121), the task dispatches normally regardless of whether a key is stored. As a consequence, a keyless launch fetch that the network rate-limits surfaces the standard fetch-outcome snackbar (MKT-145) at cold start — unlike keyed mode, whose KEY-041 skip keeps a keyless-key cold start silent. This divergence is intended: in keyless mode the user has opted into anonymous fetching and a launch-time outcome (success or rate-limit) is the expected signal.

**KEY-053 — Keyless Stooq fetch omits the key (backend)**: When the fetch mode is keyless, the Stooq price-fetch path solves the proof-of-work browser-verification gate (as in KEY-043) but presents **no apikey** — the anonymous daily-download request. The KEY-044 no-key short-circuit is suppressed in this mode: the fetch runs and reads the latest daily close per symbol exactly as the keyed path does (MKT-117/118 date handling unchanged). The chosen mode reaches the backend with the fetch request; the backend does not read the device-local setting itself. Anonymous access is subject to Stooq's per-IP daily limit, so a keyless fetch can fail or rate-limit where a keyed one would not — such failures surface through the existing fetch-outcome snackbar (MKT-145) and per-holding "no price" diagnostics (MKT-032), never silently.

**KEY-054 — Keyed mode is the unchanged default (frontend + backend)**: When the fetch mode is keyed (KEY-050 on, the default), KEY-040 (refresh gate), KEY-041 (launch skip), KEY-043 (fetch with the stored key), and KEY-044 (no-key short-circuit) all apply exactly as specified. Switching the setting flips the whole feature between the two coherent modes; the modes never partially mix.

**KEY-055 — Mode is fixed per fetch task (frontend + backend)**: The fetch mode is read at the moment a fetch task is dispatched and travels with that request (KEY-053). A task already in flight (MKT-113 permits one at a time) completes under the mode it was dispatched with, even if the user flips the setting before it finishes; the new mode takes effect on the next dispatched task. The backend never re-reads the device-local setting mid-task, and a mode change never aborts a running fetch.

---

## Workflow

```
Side menu
  └─ "Connections" (KEY-030) ─▶ Connections dialog (KEY-031)
                                  Stooq row (KEY-032):
                                    status: Key set ✓ / No key   (KEY-016)
                                    active tier label             (KEY-015)
                                    [paste / replace field]
                                    [Test key]  ─▶ live probe      (KEY-020, KEY-021)
                                                   inline result   (KEY-023)
                                    [Save]      ─▶ store via tier   (KEY-010, KEY-011)
                                                   tier 3 → opt-in confirm (KEY-012)
                                                   snackbar         (KEY-033)
                                    [Remove]    ─▶ confirm (KEY-034)
                                                   clear all tiers  (KEY-013)

Refresh prices (global MKT-130 / account MKT-131)
  └─ fetch mode? (KEY-050)
       ├─ keyless ─▶ dispatch fetch; Stooq fetches anonymously, no key (KEY-051, KEY-053)
       └─ keyed   ─▶ key stored?  (KEY-016)
                       ├─ yes ─▶ dispatch fetch; Stooq uses key (KEY-043, KEY-054)
                       └─ no  ─▶ open Connections dialog focused on Stooq (KEY-040)

Launch auto-fetch (MKT-121, enabled)
  └─ fetch mode? (KEY-050)
       ├─ keyless ─▶ dispatch fetch anonymously, key or no key (KEY-052)
       └─ keyed, no key ─▶ no provider request; no dialog; miss shown via snackbar MKT-145 (KEY-041)

Manual "Enter price" / price history (MKT-010 / MKT-070+)
  └─ always available, key or no key (KEY-042)
```

---

## UX Draft

### Entry Point

A "Connections" entry in the side menu (KEY-030). Opens the Connections dialog over the current view; no navigation away from the current page.

The fetch-mode setting (KEY-050) lives separately, on the **Settings** page beside the existing auto-fetch preference (MKT-120) — not in the Connections dialog, since it governs how every Stooq fetch behaves rather than a per-provider key.

### Fetch-mode setting (Settings page)

| Element | Content                                                                                                  |
| ------- | -------------------------------------------------------------------------------------------------------- |
| Toggle  | "Use Stooq API key" — on (keyed, default) / off (keyless)                                                |
| Help    | A short hint that keyless fetching is anonymous and may be rate-limited, and that a key is more reliable |

- **Default (keyed)**: toggle on; price fetches use the stored key and the refresh gate (KEY-040) applies — first-run prompts the user to the Connections dialog when no key is stored.
- **Keyless**: toggle off; price fetches go out anonymously, the refresh gate is bypassed (KEY-051), and launch auto-fetch runs without a key (KEY-052). The persisted choice survives a restart (KEY-050).

### Main Component

A dialog listing providers, one row per provider (KEY-031). In this slice: a single Stooq row.

### Provider Row

| Element       | Content                                                                  |
| ------------- | ------------------------------------------------------------------------ |
| Provider name | "Stooq"                                                                  |
| Status        | "Key set" (with active-tier label) or "No key"                           |
| Key field     | Password-style paste field; replaces the existing key on save            |
| Signup link   | Link out to the provider's key page so the user can obtain a free key    |
| Test key      | Probes the provider with the entered value; inline result (KEY-023)      |
| Save          | Stores the key via the tier ladder; tier-3 path adds a risk-confirm step |
| Remove        | Confirmation, then clears all tiers                                      |

### States

- **No key**: status reads "No key"; Test and Remove are unavailable until a value is entered (Test operates on the entered value; Remove is hidden when nothing is stored).
- **Key entered, not saved**: Test and Save are enabled.
- **Testing in-flight**: Test shows a spinner; row actions disabled (KEY-035).
- **Test success / invalid / unreachable**: three distinct inline results (KEY-023).
- **Saved**: status flips to "Key set" with the active-tier label; snackbar confirms (KEY-033).
- **Tier-3 opt-in**: a warning + acknowledgement step precedes a plaintext write (KEY-012).
- **Remove confirmation**: dialog identifies the provider; on confirm, status returns to "No key" (KEY-034).
- **Save rejected**: inline error in the row; previous state preserved (KEY-033).

### User Flow — first-time Stooq key setup

1. User clicks "Refresh prices" on the dashboard; no key is stored.
2. The Connections dialog opens focused on the Stooq row (KEY-040).
3. User follows the signup link, obtains a free key, and pastes it into the field.
4. User clicks "Test key"; the row shows a success result (KEY-023).
5. User clicks "Save"; the key is stored in the OS keychain (tier 1); the row shows "Key set · OS keychain"; a snackbar confirms (KEY-033).
6. User clicks "Refresh prices" again; the fetch now dispatches and prices populate (KEY-043, MKT flow).

### User Flow — minimal-Linux fallback

1. On a host with no keyring service, the user pastes a key and saves.
2. The keychain tier is unavailable, so the app offers the session-memory tier (or, if the user opts in, the plaintext tier with a risk warning) (KEY-011, KEY-012).
3. With session-memory, the row shows "Key set · Session only"; the key works for this session and must be re-entered next launch (KEY-017).

### User Flow — remove a key

1. User opens Connections from the side menu (KEY-030).
2. User clicks "Remove" on the Stooq row; a confirmation identifies the provider (KEY-034).
3. On confirm, the key is cleared from every tier (KEY-013); the row returns to "No key"; a snackbar confirms.

---

## Open Questions

- [x] **ADR-008 premise change** — resolved: ADR-015 superseded ADR-008's _Stooq-primary_ decision point, and [ADR-016](../adr/016-stooq-optional-keyless-fetch-mode.md) in turn supersedes ADR-015 (Stooq is dual-mode: keyed by default, optional keyless). ADR-008's Finnhub-fallback, Manual-override, and `AssetPriceSource` decisions carry forward through both and remain valid.
- [x] **Connections entry label** — resolved: "Connections" (matches ADR-011's "Connections panel"; provider-neutral). Reflected in KEY-030.
- [x] **Test-probe symbol** — resolved: a fixed well-known symbol the provider is guaranteed to cover, not a holding-derived one. Reflected in KEY-021.
- [x] **Keyless fetch mode** — resolved: [ADR-016](../adr/016-stooq-optional-keyless-fetch-mode.md) adds an optional, user-selected keyless (anonymous) Stooq fetch mode, default keyed. Reflected in KEY-050–055.
- [x] **Keyless rate-limit → suggest keyed?** — resolved (deferred): when a keyless fetch is rate-limited (KEY-053), v1 surfaces the standard MKT-145 snackbar; an actionable "enable your API key" upsell is left to a later UX refinement, out of v1 scope.

None — all questions have been resolved.
