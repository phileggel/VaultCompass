import { existsSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { browser } from "@wdio/globals";

// Real-app captures land here; the E2E workflow uploads the folder as a run
// artifact and links it from the pull request. Git-ignored, never committed —
// the committed visual proofs under screenshots/ are component previews, these
// are the running application.
const SCREENSHOT_DIR = fileURLToPath(new URL("../../screenshots/e2e", import.meta.url));

/**
 * Captures the current screen in both themes as
 * `screenshots/e2e/{name}-light.png` and `screenshots/e2e/{name}-dark.png`.
 *
 * Call once per spec file, after the wait that settles the screen under test
 * (the dialog open, the row showing its final value). The pair is the pull
 * request's evidence of what the real app shows.
 *
 * The theme is forced through the `dark` root class the theme hook itself
 * sets (`useThemeToggle.applyTheme`); the app's stored mode is untouched, so
 * only the header's theme-toggle icon may disagree with the forced colours.
 * The class is restored afterwards, even when a capture throws, so the rest
 * of the spec runs under the app's own theme.
 */
export async function captureScreen(name: string): Promise<void> {
  if (!existsSync(SCREENSHOT_DIR)) mkdirSync(SCREENSHOT_DIR, { recursive: true });

  const wasDark = await browser.execute(() => document.documentElement.classList.contains("dark"));

  try {
    for (const scheme of ["light", "dark"] as const) {
      await setDark(scheme === "dark");
      await waitForRepaint();
      await browser.saveScreenshot(resolve(SCREENSHOT_DIR, `${name}-${scheme}.png`));
    }
  } finally {
    await setDark(wasDark);
  }
}

async function setDark(dark: boolean): Promise<void> {
  await browser.execute((value: boolean) => {
    document.documentElement.classList.toggle("dark", value);
  }, dark);
}

// Two animation frames: the first is scheduled before the class flip has been
// styled, the second runs after that frame painted. Tracks the WebView's own
// paint instead of guessing a delay that headless GTK rendering may exceed.
async function waitForRepaint(): Promise<void> {
  await browser.executeAsync((done: () => void) => {
    requestAnimationFrame(() => requestAnimationFrame(() => done()));
  });
}
