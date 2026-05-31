import { $ } from "@wdio/globals";

/** Navigates to the Assets page and waits for the Add Asset FAB to confirm the route is active. */
export async function navigateToAssets(): Promise<void> {
  const nav = await $("#nav-assets");
  await nav.waitForExist({ timeout: 15000 });
  await nav.click();
  const fab = await $("#fab-add-asset");
  await fab.waitForExist({ timeout: 10000 });
}

/**
 * Opens the Account Details header "Add" dropdown (DIV-012) and clicks one of
 * its menu items by stable id (e.g. `add-menu-deposit`, `add-menu-withdraw`,
 * `add-menu-open-balance`, `add-menu-dividend`). Replaces the pre-DIV-012
 * standalone header buttons (`#action-deposit` / `#action-withdraw` /
 * `#action-open-balance`), which the consolidated menu superseded.
 */
export async function openAddMenuItem(menuItemId: string): Promise<void> {
  const addMenuBtn = await $("#account-details-add-menu");
  await addMenuBtn.waitForExist({ timeout: 15000 });
  await addMenuBtn.click();
  const item = await $(`#${menuItemId}`);
  await item.waitForExist({ timeout: 5000 });
  await item.click();
}
