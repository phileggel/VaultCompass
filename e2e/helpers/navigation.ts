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
 * Round-trips through the Assets page so the Accounts component remounts and
 * re-fetches, picking up any IPC-seeded accounts added after the initial load.
 * Leaves the Accounts list active (its FAB is the route-ready signal).
 */
export async function navigateToAccounts(): Promise<void> {
  const assetsNav = await $("#nav-assets");
  await assetsNav.waitForExist({ timeout: 15000 });
  await assetsNav.click();
  await $("#fab-add-asset").waitForExist({ timeout: 10000 });

  const accountsNav = await $("#nav-accounts");
  await accountsNav.waitForExist({ timeout: 10000 });
  await accountsNav.click();
  await $("#fab-add-account").waitForExist({ timeout: 10000 });
}

/**
 * Opens an account's details view from the Accounts table, selecting the row by
 * its stable `id` (E1/E4). Clicks the name cell (first column) rather than the
 * `<tr>` — the row centre overlaps action buttons that stopPropagation; the name
 * cell bubbles to the row's onClick. Assumes `navigateToAccounts()` ran first.
 * Waits for the header Add menu (DIV-012), present on every account details view.
 */
export async function navigateToAccountDetails(accountId: string): Promise<void> {
  const nameCell = await $(`#account-row-${accountId} td:first-child`);
  await nameCell.waitForExist({ timeout: 10000 });
  await nameCell.click();
  await $("#account-details-add-menu").waitForExist({ timeout: 10000 });
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
