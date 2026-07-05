# Business Rules — What's New Dialog

## Context

After the in-app updater (see [update.md](update.md)) installs a new version, the user has no visibility into what changed. This feature shows a one-time dialog on the first launch after a version upgrade, listing the CHANGELOG sections released between the last version the user acknowledged and the currently running version. The dialog content comes verbatim from the repo `CHANGELOG.md` (Keep a Changelog format) bundled into the frontend at build time.

---

## Business rules

### Persistence

**WNW-010 — Last-seen version persistence (frontend)**: The last acknowledged app version is persisted in `localStorage` under the key `whats_new_last_seen_version`.

### Trigger

**WNW-020 — Show condition (frontend)**: On launch, once the app version has resolved, the dialog is shown if and only if a stored last-seen version exists, it differs from the current version, and at least one changelog section falls in the interval (stored, current]. Until dismissed, the dialog reappears on every subsequent launch.

**WNW-030 — Fresh-install silent seeding (frontend)**: When no last-seen version is stored (fresh install), the current version is seeded silently and nothing is shown.

### Content

**WNW-040 — Version stacking (frontend)**: The dialog lists every changelog section strictly newer than the stored version and up to the current version (inclusive), newest first — skipped versions stack in one dialog. Each section shows its version, release date, subsection headings (Added/Fixed/Changed as written in the changelog), and bullet lines.

**WNW-060 — English-only content (frontend)**: The changelog content is displayed as written — English-only by design. Only the dialog chrome (title, dismiss action) is translated.

### Dismissal

**WNW-050 — Dismiss acknowledges the current version (frontend)**: The dialog offers a single dismiss action (also triggered by the close affordances). Dismissing writes the current version to storage and closes the dialog.

### Degraded cases

**WNW-070 — Malformed or missing changelog (frontend)**: If the bundled changelog cannot be parsed, the stored version cannot be interpreted, or no section falls in the interval, the dialog is silently not shown and the current version is seeded so the check does not repeat on every launch.

---

## E2E note

The webview's `localStorage` persists across E2E runs (only the SQLite data dir is redirected to an ephemeral location). A version bump between runs would therefore satisfy WNW-020 and open the dialog over the UI. The suite neutralizes this in the `wdio.conf.ts` `before` hook: it removes `whats_new_last_seen_version` and reloads, routing the launch through the WNW-030 fresh-install path.

---

## Open questions

None — all questions have been resolved.
