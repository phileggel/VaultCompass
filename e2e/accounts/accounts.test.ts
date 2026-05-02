/**
 * E2E tests — Account management (CRUD)
 *
 * Contract: docs/contracts/account-contract.md
 * Spec rules covered:
 *   ACC-001 — create account → appears in account list
 *   ACC-002 — edit account name → list reflects update
 *   ACC-003 — delete account → removed from list
 *   ACC-004 — create duplicate name → inline error shown
 */

import assert from "node:assert";
import { $, browser } from "@wdio/globals";

// ---------------------------------------------------------------------------
// Helpers (E2E rules E6, E7)
// ---------------------------------------------------------------------------

async function setReactInputValue(elementId: string, value: string): Promise<void> {
  await browser.execute(
    (id, val) => {
      const el = document.getElementById(id) as HTMLInputElement | null;
      if (!el) return;
      const nativeSetter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype,
        "value",
      )?.set;
      nativeSetter?.call(el, val);
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
    },
    elementId,
    value,
  );
}

// ---------------------------------------------------------------------------
// IPC seed helpers
// ---------------------------------------------------------------------------

async function seedAccount(name: string): Promise<string> {
  const acc = (await browser.executeAsync((n: string, done: (r: unknown) => void) => {
    // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
    window.__TAURI_INTERNALS__
      .invoke("add_account", { dto: { name: n, currency: "EUR", update_frequency: "ManualMonth" } })
      .then(done)
      .catch((err: unknown) => done({ __error: String(err) }));
  }, name)) as { id: string };
  assert.ok(!("__error" in acc), `seedAccount failed: ${JSON.stringify(acc)}`);
  return acc.id;
}

// ---------------------------------------------------------------------------
// Navigation helpers
// ---------------------------------------------------------------------------

async function navigateToAccounts(): Promise<void> {
  const nav = await $('button[aria-label="Accounts"]');
  await nav.waitForExist({ timeout: 15000 });
  await nav.click();
  const fab = await $('button[aria-label="Add account"]');
  await fab.waitForExist({ timeout: 10000 });
}

// Navigate away (to Assets) then back to Accounts, forcing the accounts
// component to remount and re-fetch from the store. Use this after IPC
// seeding when the accounts page is already mounted.
async function forceRefreshToAccounts(): Promise<void> {
  const assetsNav = await $('button[aria-label="Assets"]');
  await assetsNav.waitForExist({ timeout: 15000 });
  await assetsNav.click();
  await $('button[aria-label="Add asset"]').waitForExist({ timeout: 10000 });
  await navigateToAccounts();
}

// Language-invariant selector: finds the account row button by the account
// name text content rather than the translated aria-label.
async function findAccountButton(accountName: string) {
  return $(`//button[contains(., "${accountName}")]`);
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("accounts", () => {
  beforeEach(async () => {
    const closeBtn = await $('button[aria-label="Close"]');
    if (await closeBtn.isExisting()) {
      await closeBtn.click();
      await closeBtn.waitForExist({ timeout: 3000, reverse: true });
    }
    await navigateToAccounts();
  });

  // -------------------------------------------------------------------------
  // ACC-001 — create account → appears in account list
  // -------------------------------------------------------------------------
  it("ACC-001: creating an account shows it in the account list", async () => {
    const ACCOUNT_NAME = "E2E ACC-001 Account";

    const fab = await $('button[aria-label="Add account"]');
    await fab.click();

    const form = await $("form#add-account-form");
    await form.waitForExist({ timeout: 8000 });

    await setReactInputValue("add-account-name", ACCOUNT_NAME);
    await setReactInputValue("add-account-currency", "EUR");

    const submitBtn = await $('button[type="submit"][form="add-account-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    await form.waitForExist({ timeout: 8000, reverse: true });

    // After creation the app navigates to the new account's detail page.
    // Navigate back to the accounts list to verify the entry is there.
    await navigateToAccounts();

    const accountBtn = await findAccountButton(ACCOUNT_NAME);
    await accountBtn.waitForExist({ timeout: 10000 });
    assert.ok(
      await accountBtn.isExisting(),
      `Account "${ACCOUNT_NAME}" must appear in list after creation`,
    );
  });

  // -------------------------------------------------------------------------
  // ACC-002 — edit account name → list reflects update
  // -------------------------------------------------------------------------
  it("ACC-002: editing an account name updates the list", async () => {
    const ORIGINAL_NAME = "E2E ACC-002 Original";
    const UPDATED_NAME = "E2E ACC-002 Updated";

    await seedAccount(ORIGINAL_NAME);

    // Navigate away and back to force the accounts list to remount and pick
    // up the IPC-seeded account from the store.
    await forceRefreshToAccounts();

    const editBtn = await $('button[aria-label="Edit"]');
    await editBtn.waitForExist({ timeout: 8000 });
    await editBtn.click();

    const form = await $("form#edit-account-form");
    await form.waitForExist({ timeout: 8000 });

    await setReactInputValue("edit-account-name", UPDATED_NAME);

    const submitBtn = await $('button[type="submit"][form="edit-account-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    await form.waitForExist({ timeout: 8000, reverse: true });

    const updatedBtn = await findAccountButton(UPDATED_NAME);
    await updatedBtn.waitForExist({ timeout: 10000 });
    assert.ok(
      await updatedBtn.isExisting(),
      `Updated account "${UPDATED_NAME}" must appear in list`,
    );
  });

  // -------------------------------------------------------------------------
  // ACC-003 — delete account → removed from list
  // -------------------------------------------------------------------------
  it("ACC-003: deleting an account removes it from the list", async () => {
    const ACCOUNT_NAME = "E2E ACC-003 Delete";

    await seedAccount(ACCOUNT_NAME);

    // Force remount so the seeded account appears in the list.
    await forceRefreshToAccounts();

    const accountBtn = await findAccountButton(ACCOUNT_NAME);
    await accountBtn.waitForExist({ timeout: 8000 });

    const deleteBtn = await $('button[aria-label="Delete"]');
    await deleteBtn.waitForExist({ timeout: 5000 });
    await deleteBtn.click();

    // Confirm in the dialog (confirmLabel = "Delete")
    const confirmBtn = await $('//button[normalize-space()="Delete"]');
    await confirmBtn.waitForDisplayed({ timeout: 5000 });
    await confirmBtn.click();

    await accountBtn.waitForExist({ timeout: 8000, reverse: true });
    assert.ok(
      !(await accountBtn.isExisting()),
      `Account "${ACCOUNT_NAME}" must be removed from list after deletion`,
    );
  });

  // -------------------------------------------------------------------------
  // ACC-004 — duplicate name → inline error shown
  // -------------------------------------------------------------------------
  it("ACC-004: creating an account with a duplicate name shows an inline error", async () => {
    const DUPLICATE_NAME = "E2E ACC-004 Duplicate";

    await seedAccount(DUPLICATE_NAME);
    await forceRefreshToAccounts();

    const fab = await $('button[aria-label="Add account"]');
    await fab.click();

    const form = await $("form#add-account-form");
    await form.waitForExist({ timeout: 8000 });

    await setReactInputValue("add-account-name", DUPLICATE_NAME);
    await setReactInputValue("add-account-currency", "EUR");

    const submitBtn = await $('button[type="submit"][form="add-account-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    const alert = await $('[role="alert"]');
    await alert.waitForExist({ timeout: 8000 });
    assert.ok(await alert.isExisting(), "Inline error must appear for duplicate account name");
    assert.ok(await form.isExisting(), "Modal must remain open after duplicate name error");
  });
});
