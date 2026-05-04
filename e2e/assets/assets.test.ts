/**
 * E2E tests — Asset lifecycle (create, archive, unarchive)
 *
 * Contract: docs/contracts/asset-contract.md
 * Spec rules covered:
 *   AST-001 — create asset → appears in asset table
 *   AST-002 — archive asset → moves to archived state
 *   AST-003 — unarchive asset → returns to active
 */

import assert from "node:assert";
import { $, browser } from "@wdio/globals";
import { setReactInputValue } from "../helpers/react";
import { seedAsset, seedCategory } from "../helpers/seed";

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

async function navigateToAssets(): Promise<void> {
  const nav = await $('button[aria-label="Assets"]');
  await nav.waitForExist({ timeout: 15000 });
  await nav.click();
  const fab = await $('button[aria-label="Add asset"]');
  await fab.waitForExist({ timeout: 10000 });
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("assets", () => {
  beforeEach(async () => {
    const closeBtn = await $('button[aria-label="Close"]');
    if (await closeBtn.isExisting()) {
      await closeBtn.click();
      await closeBtn.waitForExist({ timeout: 3000, reverse: true });
    }
    await navigateToAssets();
  });

  // -------------------------------------------------------------------------
  // Create asset → appears in table
  // -------------------------------------------------------------------------
  it("creates an asset manually and it appears in the asset table", async () => {
    const ASSET_NAME = "E2E Asset Create";
    await seedCategory("E2E Cat Create");

    const fab = await $('button[aria-label="Add asset"]');
    await fab.click();

    // WebLookupModal opens first — click Fill manually to go to the form.
    const fillManually = await $('button[aria-label="Fill manually"]');
    await fillManually.waitForDisplayed({ timeout: 8000 });
    await fillManually.click();

    const form = await $("form#add-asset-form");
    await form.waitForExist({ timeout: 8000 });

    await setReactInputValue("add-asset-name", ASSET_NAME);
    await setReactInputValue("add-asset-reference", "TSTCRT");
    await setReactInputValue("add-asset-currency", "EUR");

    // Select category from native <select>.
    const categorySelect = await $("#add-asset-category");
    await categorySelect.waitForExist({ timeout: 5000 });
    await categorySelect.selectByVisibleText("E2E Cat Create");

    // Select class from native <select>.
    const classSelect = await $("#add-asset-class");
    await classSelect.selectByVisibleText("Stocks");

    const submitBtn = await $('button[type="submit"][form="add-asset-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    // After success the router navigates to /assets. Navigate away and back
    // to force the assets list to remount and reflect the newly created asset.
    await form.waitForExist({ timeout: 8000, reverse: true });
    const accountsNav = await $('button[aria-label="Accounts"]');
    await accountsNav.waitForExist({ timeout: 10000 });
    await accountsNav.click();
    await $('button[aria-label="Add account"]').waitForExist({ timeout: 10000 });
    await navigateToAssets();

    const assetCell = await $(`*=${ASSET_NAME}`);
    await assetCell.waitForExist({ timeout: 10000 });
    assert.ok(
      await assetCell.isExisting(),
      `Asset "${ASSET_NAME}" must appear in table after creation`,
    );
  });

  // -------------------------------------------------------------------------
  // Archive asset → shows Archived badge; disappears from active list
  // -------------------------------------------------------------------------
  it("archiving an asset removes it from the active list", async () => {
    const ASSET_NAME = "E2E Asset Archive";
    const categoryId = await seedCategory("E2E Cat Archive");
    await seedAsset(ASSET_NAME, categoryId);
    await navigateToAssets();

    // Find and click the Archive icon button scoped to the asset's row,
    // to avoid matching a button in another row when multiple assets are present.
    const archiveBtn = await $(
      `//tr[.//td[contains(., "${ASSET_NAME}")]]//button[@aria-label="Archive"]`,
    );
    await archiveBtn.waitForExist({ timeout: 8000 });
    await archiveBtn.click();

    // Confirm in the dialog (confirmLabel = "Archive") — scoped to dialog.
    const confirmBtn = await $('//*[@role="dialog"]//button[normalize-space()="Archive"]');
    await confirmBtn.waitForDisplayed({ timeout: 5000 });
    await confirmBtn.click();

    // Asset disappears from the active (default) list.
    const assetCell = await $(`*=${ASSET_NAME}`);
    await assetCell.waitForExist({ timeout: 8000, reverse: true });
    assert.ok(
      !(await assetCell.isExisting()),
      `Asset "${ASSET_NAME}" must be hidden from active list after archiving`,
    );
  });

  // -------------------------------------------------------------------------
  // Unarchive asset → returns to active list
  // -------------------------------------------------------------------------
  it("unarchiving an asset returns it to the active list", async () => {
    const ASSET_NAME = "E2E Asset Unarchive";
    const categoryId = await seedCategory("E2E Cat Unarchive");
    const assetId = await seedAsset(ASSET_NAME, categoryId);

    // Archive the asset via IPC so we start in the archived state.
    await browser.executeAsync((id: string, done: (r: unknown) => void) => {
      // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
      window.__TAURI_INTERNALS__
        .invoke("archive_asset", { id })
        .then(done)
        .catch((err: unknown) => done({ __error: String(err) }));
    }, assetId);

    await navigateToAssets();

    // Show the archived list by checking the "Show archived" checkbox.
    // Use data-testid to avoid language-specific text matching.
    const showArchivedLabel = await $('[data-testid="show-archived-toggle"]');
    await showArchivedLabel.waitForExist({ timeout: 5000 });
    await showArchivedLabel.click();

    const assetCell = await $(`*=${ASSET_NAME}`);
    await assetCell.waitForExist({ timeout: 8000 });

    // Click Unarchive — scoped to the asset's row.
    const unarchiveBtn = await $(
      `//tr[.//td[contains(., "${ASSET_NAME}")]]//button[@aria-label="Unarchive"]`,
    );
    await unarchiveBtn.waitForExist({ timeout: 5000 });
    await unarchiveBtn.click();

    // Confirm in the dialog (confirmLabel = "Unarchive") — scoped to dialog.
    const confirmBtn = await $('//*[@role="dialog"]//button[normalize-space()="Unarchive"]');
    await confirmBtn.waitForDisplayed({ timeout: 5000 });
    await confirmBtn.click();

    // Uncheck "Show archived" and verify asset is now back in the active list.
    await showArchivedLabel.click();
    const activeAssetCell = await $(`*=${ASSET_NAME}`);
    await activeAssetCell.waitForExist({ timeout: 8000 });
    assert.ok(
      await activeAssetCell.isExisting(),
      `Asset "${ASSET_NAME}" must appear in active list after unarchiving`,
    );
  });
});
