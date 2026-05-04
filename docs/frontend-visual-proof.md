# Frontend Visual Proof Rules

> ⚠️ **AI AGENT MUST NEVER UPDATE THIS DOCUMENT**

Any change that touches a `.tsx`, `.css`, or visual asset file **MUST** include a committed
screenshot in `screenshots/` before merging — whether the branch is merged via `just merge` or a
PR is opened. If a PR is opened, also embed the screenshot in the description. If you cannot
produce a screenshot, explain why in the chat instead.

---

## When visual proof is not required

State it explicitly at the top of the PR description:

> No visual impact — internal refactor / Rust-only change.

Then screenshot at least one screen that _consumes_ the modified code as a non-regression proof.

---

## What to capture

| Change type                                                          | Required artefact                      |
| -------------------------------------------------------------------- | -------------------------------------- |
| New component or layout change                                       | Screenshot of every affected state     |
| Interaction (hover, animation, modal open/close, loading transition) | Playwright video clip saved as `.webm` |
| Shared / design-system component                                     | Screenshot of 2–3 distinct call sites  |
| Dark mode (if ever added)                                            | Both modes side by side                |

**States to cover for every component panel:** idle · loading · results/content · empty · error.

---

## Process

### 1 — Capture "before" at task start

Before writing any code, screenshot the current state of the affected component or screen.
Skip if the component is new (no "before" exists).

```bash
# Start Vite dev server (not the full Tauri app)
npx vite --port 1422 --host 127.0.0.1
```

Then use the Playwright script (step 3) against the unmodified code.

### 2 — Create a preview entry

Create two temporary files (delete them before the final commit):

**`preview.html`** at the project root:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/__preview__/main.tsx"></script>
  </body>
</html>
```

**`src/__preview__/main.tsx`** — renders the component in every relevant state with hardcoded
mocked data (no Tauri `invoke()` calls needed; the gateway pattern already keeps components
decoupled from the IPC layer).

Import the real i18n config and global CSS so the screenshot reflects the actual app styles:

```tsx
import "../i18n/config";
import "../ui/global.css";
```

### 3 — Take the screenshot with Playwright

Playwright's Chromium binary is cached at:
`~/.cache/ms-playwright/chromium-1217/chrome-linux64/chrome`

Run the Vite dev server on port 1422 (different from the Tauri dev port 1420), then:

```js
import { chromium } from "/tmp/pw-test/node_modules/playwright/index.mjs";
// launch with executablePath pointing to the cached binary
// setViewportSize({ width: 1600, height: 900 })
// page.goto("http://127.0.0.1:1422/preview.html", { waitUntil: "networkidle" })
// page.screenshot({ path: "screenshots/<name>.png", fullPage: true })
```

For interaction clips, use Playwright's video recording:

```js
const context = await browser.newContext({
  recordVideo: { dir: "screenshots/" },
});
```

### 4 — Commit the artefact and embed it in the PR

Save screenshots to `screenshots/<ComponentName>-preview.png`.
Commit them on the feature branch before pushing.

Reference in the PR body using the raw GitHub URL:

```markdown
![ComponentName preview](https://raw.githubusercontent.com/phileggel/VaultCompass/<branch>/screenshots/<ComponentName>-preview.png)
```

`screenshots/` is intentionally tracked in git. Each commit to that file is a point-in-time
record of what the component looked like. Use `git log` to browse the visual history:

```bash
git log --oneline -- screenshots/SearchPanel-preview.png
git show <sha>:screenshots/SearchPanel-preview.png > /tmp/old.png
```

### 5 — Clean up the preview files

Delete `preview.html` and `src/__preview__/` before the final commit on the branch.

---

## Preview fidelity

The preview page imports the same files the real app uses, so design parity is very high:

| Design element             | Source                                | In preview?                                           |
| -------------------------- | ------------------------------------- | ----------------------------------------------------- |
| M3 color tokens            | `src/ui/global.css` `@theme` block    | ✅ (same import)                                      |
| Inter + Manrope fonts      | `@fontsource-variable/*` npm packages | ✅ (resolved by Vite)                                 |
| Tailwind utilities         | `@tailwindcss/vite` plugin            | ✅ (same plugin)                                      |
| Component code             | Direct import                         | ✅ (identical)                                        |
| i18n translations          | `src/i18n/config.ts`                  | ✅ (same import)                                      |
| Modal backdrop / elevation | App shell context                     | ⚠️ absent — preview is standalone                     |
| WebView font rendering     | Platform WebView (WebKit on Linux)    | ⚠️ preview uses Chromium — minor subpixel differences |

The two caveats are cosmetic and do not affect review usefulness. If a change specifically touches
modal chrome or backdrop blur, add a note in the PR description that those elements are not shown.

---

## PR description template (frontend changes)

```markdown
## What

<1–2 sentence summary>

## Visual proof

### Before

![before](raw-github-url)

### After

![after](raw-github-url)

### States covered

- Idle · Loading · Results · Empty · Error: <screenshot or note>

## How to test

1. <step>
2. <step>

## Checklist

- [ ] Screenshots for every modified component
- [ ] Edge states captured (empty, loading, error)
- [ ] No `invoke()` calls in presentational components
- [ ] `just check-full` passes
```

---

## Never do

- Open a frontend PR without visual proof and say "test it locally"
- Call `invoke()` directly inside a presentational component (prevents mocked rendering)
- Leave `preview.html` or `src/__preview__/` committed on the branch
