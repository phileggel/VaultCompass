/**
 * E2E tests — Stooq keyless fetch-mode toggle (KEY-050–055)
 *
 * Spec: docs/spec/api-key-management.md (KEY-050–055)
 * Plan: docs/plan/keyless-fetch-mode-plan.md § Closure / § E2E
 *
 * Spec rules covered:
 *   KEY-051 — keyless mode bypasses the KEY-040 refresh gate: Connections dialog
 *             must NOT open when #settings-use-api-key is OFF and Refresh is clicked
 *   KEY-040 — keyed mode (default) still gates refresh on a stored key: Connections
 *             dialog DOES open when #settings-use-api-key is ON and no key is stored
 *             (regression guard — proves the toggle did not break the existing gate)
 *   KEY-050 — the setting persists to localStorage (stooq_use_api_key) and defaults
 *             to true (keyed); the toggle flips it
 *
 * Deliberately NOT covered at this tier (and why):
 *
 *   KEY-052 (keyless launch auto-fetch) — exercises the mount-once useEffect in
 *     App.tsx on cold start; not reachable within a running session without
 *     restarting the Tauri WebView. Covered by FE unit test
 *     src/App.test.tsx "auto-fetch in keyless mode dispatches without the key gate".
 *
 *   KEY-053 (anonymous Stooq URL) — backend-only: the q/d/l/ URL omits &apikey
 *     in keyless mode. Covered by BE integration test
 *     src-tauri/tests/asset_price_fetch_crud.rs dispatcher_keyless_mode_fetches_without_key.
 *     No observable DOM surface.
 *
 *   KEY-054 (keyed mode unchanged) — the regression scenario below (KEY-040 gate
 *     still fires when toggle is ON) is the observable E2E proxy for KEY-054.
 *
 *   KEY-055 (mode fixed per task) — the mode is captured at dispatch and passed as
 *     a spawn() argument; the backend has no access to the device-local setting, so
 *     a mid-task re-read is structurally impossible. The "mode travels with the task"
 *     core is verified by the BE dispatcher tests (dispatcher_keyless_mode_fetches_
 *     without_key + dispatcher_with_key_threads_api_key_into_fetch_price), each of
 *     which passes a fixed mode into spawn and asserts the task honours it.
 *
 * Session-isolation:
 *   localStorage["stooq_use_api_key"] persists across scenarios. Every scenario
 *   restores the default (keyed / "true") in a finally block so the suite
 *   remains order-independent. The no-key baseline is established explicitly
 *   with the idempotent removeProviderKey("Stooq") — never assumed from order.
 *
 * No live Stooq traffic:
 *   KEY-051 is asserted by verifying the Connections dialog does NOT open. The
 *   fetch task is dispatched to the backend but completes immediately with
 *   NoFetchableHoldings (accounts seeded with cash-only holdings; cash is
 *   excluded from fetch scope per MKT-116 before any symbol resolution or
 *   network call). Zero outbound requests are made.
 */

import assert from "node:assert";
import { $, browser } from "@wdio/globals";
import { dismissLeftoverModal } from "../helpers/modal";
import { navigateToAccounts } from "../helpers/navigation";
import { removeProviderKey } from "../helpers/seed";

// ---------------------------------------------------------------------------
// Local helpers
// ---------------------------------------------------------------------------

/**
 * Navigates to the Settings page via the sidebar button and waits until the
 * #settings-use-api-key checkbox is present (route-ready signal).
 */
async function navigateToSettings(): Promise<void> {
  const settingsNav = await $("#nav-settings");
  await settingsNav.waitForExist({ timeout: 15000 });
  await settingsNav.click();
  const toggle = await $("#settings-use-api-key");
  await toggle.waitForExist({ timeout: 10000 });
}

/**
 * Reads the raw localStorage value for stooq_use_api_key.
 * Returns null when the key is absent (absent = default keyed per KEY-054).
 */
async function readUseApiKeyStorage(): Promise<string | null> {
  return browser.execute(() => localStorage.getItem("stooq_use_api_key"));
}

/**
 * Sets localStorage["stooq_use_api_key"] directly — used only in finally
 * blocks to restore the default without depending on UI nav state.
 */
async function restoreKeyedDefault(): Promise<void> {
  await browser.execute(() => localStorage.removeItem("stooq_use_api_key"));
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("settings — keyless fetch-mode toggle", () => {
  beforeEach(async () => {
    await dismissLeftoverModal();
  });

  // -------------------------------------------------------------------------
  // KEY-051 — keyless mode bypasses the KEY-040 refresh gate
  //
  // Steps:
  //   1. Remove any stored Stooq key (idempotent) — ensures the KEY-040 gate
  //      would fire in keyed mode, making the negative assertion meaningful.
  //   2. Navigate to Settings, read the initial toggle state, then click
  //      #settings-use-api-key to switch to keyless (OFF).
  //   3. Assert localStorage["stooq_use_api_key"] is now "false" (KEY-050).
  //   4. Navigate to Accounts, click #account-manager-refresh-prices.
  //   5. Wait a bounded period and assert #provider-key-input-Stooq never
  //      appears — the Connections dialog must NOT open (KEY-051).
  //   6. Restore the default (keyed / localStorage key removed) in finally.
  //
  // The dispatch lands on a cash-only account (no non-cash holdings are
  // seeded), so the backend rejects with NoFetchableHoldings before any
  // network call — zero outbound requests.
  // -------------------------------------------------------------------------
  it("KEY-051: keyless toggle OFF — Refresh prices does not open the Connections dialog", async () => {
    await removeProviderKey("Stooq");

    try {
      // Step 1 — navigate to Settings and switch to keyless mode.
      await navigateToSettings();

      const toggle = await $("#settings-use-api-key");
      await toggle.waitForExist({ timeout: 10000 });

      // Read the initial state so we know whether one or two clicks are needed.
      // The default is keyed (true), but a prior failed run might have left it
      // in keyless state. We normalise by reading and clicking only when needed.
      const initialValue = await readUseApiKeyStorage();
      const isCurrentlyKeyed = initialValue === null || initialValue === "true";

      if (isCurrentlyKeyed) {
        // Click once to switch to keyless (OFF).
        await toggle.click();
      }

      // Step 2 — assert localStorage is "false" (keyless, KEY-050).
      const storedValue = await readUseApiKeyStorage();
      assert.strictEqual(
        storedValue,
        "false",
        `localStorage["stooq_use_api_key"] must be "false" after switching to keyless — got: "${storedValue}"`,
      );

      // Step 3 — navigate to Accounts and click global Refresh prices.
      await navigateToAccounts();

      const refreshBtn = await $("#account-manager-refresh-prices");
      await refreshBtn.waitForExist({ timeout: 10000 });
      await refreshBtn.click();

      // Step 4 — the Connections dialog must NOT open (KEY-051: gate bypassed).
      // Use the SAME 10 s bound the dialog gets in the positive (keyed) scenario,
      // so a slow CI gate firing at 4–8 s cannot slip past a too-short window.
      const keyInput = await $("#provider-key-input-Stooq");
      await keyInput.waitForExist({ timeout: 10000, reverse: true });
      assert.ok(
        !(await keyInput.isExisting()),
        "Connections dialog must NOT open when the fetch mode is keyless (KEY-051: gate bypassed)",
      );
    } finally {
      // Restore the default keyed mode so subsequent scenarios are unaffected.
      await restoreKeyedDefault();
    }
  });

  // -------------------------------------------------------------------------
  // KEY-040 regression — keyed mode (default) still gates refresh on a key
  //
  // This scenario proves the toggle did not break the pre-existing KEY-040
  // behavior: with the toggle ON and no Stooq key stored, clicking Refresh
  // MUST open the Connections dialog (KEY-040 gate fires).
  //
  // Steps:
  //   1. Remove any stored Stooq key (idempotent).
  //   2. Ensure the toggle is ON (keyed / default). If localStorage is "false"
  //      (from a prior broken run) restore it via UI click.
  //   3. Navigate to Accounts, click #account-manager-refresh-prices.
  //   4. Assert #provider-key-input-Stooq appears — Connections dialog opened.
  //   5. Dismiss the dialog cleanly before the next test.
  // -------------------------------------------------------------------------
  // Canonical KEY-040 coverage lives in e2e/connections/connections.test.ts; this
  // duplicate is the regression guard proving the new toggle did not break the gate.
  it("KEY-040 regression: keyed toggle ON + no key — Refresh prices opens the Connections dialog", async () => {
    await removeProviderKey("Stooq");
    const keyInput = await $("#provider-key-input-Stooq");

    try {
      // Ensure the toggle is in keyed mode (ON / default).
      // If a prior run left it as keyless we fix it via UI rather than direct
      // localStorage write so the toggle component state stays in sync.
      const storedValue = await readUseApiKeyStorage();
      if (storedValue === "false") {
        await navigateToSettings();
        const toggle = await $("#settings-use-api-key");
        await toggle.waitForExist({ timeout: 10000 });
        await toggle.click();
        // After clicking, localStorage should be "true" (or removed — either means keyed).
        const afterFix = await readUseApiKeyStorage();
        assert.ok(
          afterFix === null || afterFix === "true",
          `After re-enabling keyed mode the stored value should be "true" or absent — got: "${afterFix}"`,
        );
      }

      // Navigate to Accounts and trigger global Refresh prices.
      await navigateToAccounts();

      const refreshBtn = await $("#account-manager-refresh-prices");
      await refreshBtn.waitForExist({ timeout: 10000 });
      await refreshBtn.click();

      // KEY-040: with no Stooq key and keyed mode ON the gate must open the
      // Connections dialog (URL becomes ?modal=connections; the dialog mounts
      // and renders #provider-key-input-Stooq).
      await keyInput.waitForExist({ timeout: 10000 });
      assert.ok(
        await keyInput.isExisting(),
        "Connections dialog must open when keyed mode is ON and no Stooq key is stored (KEY-040 regression)",
      );
    } finally {
      // Dismiss the Connections dialog cleanly even if the assertion above threw,
      // so a mid-failure never leaves the dialog open for the next test.
      await browser.keys("Escape");
      await keyInput.waitForExist({ timeout: 5000, reverse: true }).catch(() => undefined);
      await restoreKeyedDefault();
    }
  });
});
