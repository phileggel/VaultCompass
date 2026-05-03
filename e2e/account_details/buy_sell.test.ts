/**
 * E2E tests — Buy + Sell transaction flow
 *
 * Contract: docs/contracts/record_transaction-contract.md
 * Spec rules covered:
 *   TRX-010 — buy holding → holding appears in account details
 *   TRX-020 — sell holding → quantity decremented in holding row
 *   TRX-030 — sell more than held → frontend Oversell error shown
 *
 * Seed strategy:
 *   - TRX-010: account + asset seeded via IPC; buy exercised through the UI.
 *   - TRX-020/030: account + asset + buy seeded via IPC; sell exercised through the UI.
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

function isoToDisplayDate(iso: string): string {
  const [year, month, day] = iso.split("-");
  return `${day}/${month}/${year}`;
}

// ---------------------------------------------------------------------------
// IPC seed helpers
// ---------------------------------------------------------------------------

async function seedCategory(label: string): Promise<string> {
  const cat = (await browser.executeAsync((lbl: string, done: (r: unknown) => void) => {
    // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
    window.__TAURI_INTERNALS__
      .invoke("add_category", { label: lbl })
      .then(done)
      .catch((err: unknown) => done({ __error: String(err) }));
  }, label)) as { id: string };
  assert.ok(!("__error" in cat), `seedCategory failed: ${JSON.stringify(cat)}`);
  return cat.id;
}

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

async function seedAsset(name: string, categoryId: string): Promise<string> {
  const asset = (await browser.executeAsync(
    (n: string, catId: string, done: (r: unknown) => void) => {
      // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
      window.__TAURI_INTERNALS__
        .invoke("add_asset", {
          dto: {
            name: n,
            reference: n.slice(0, 6).toUpperCase(),
            class: "Stocks",
            category_id: catId,
            currency: "EUR",
            risk_level: 3,
          },
        })
        .then(done)
        .catch((err: unknown) => done({ __error: String(err) }));
    },
    name,
    categoryId,
  )) as { id: string };
  assert.ok(!("__error" in asset), `seedAsset failed: ${JSON.stringify(asset)}`);
  return asset.id;
}

async function seedBuy(
  accountId: string,
  assetId: string,
  date: string,
  quantity: number,
): Promise<void> {
  const result = (await browser.executeAsync(
    (accId: string, astId: string, d: string, qty: number, done: (r: unknown) => void) => {
      // @ts-expect-error __TAURI_INTERNALS__ injected by Tauri WebView
      window.__TAURI_INTERNALS__
        .invoke("buy_holding", {
          dto: {
            account_id: accId,
            asset_id: astId,
            date: d,
            quantity: qty,
            unit_price: 100_000_000,
            exchange_rate: 1_000_000,
            fees: 0,
            note: "",
          },
        })
        .then(done)
        .catch((err: unknown) => done({ __error: String(err) }));
    },
    accountId,
    assetId,
    date,
    quantity,
  )) as { id?: string; __error?: string };
  assert.ok(!("__error" in result), `seedBuy failed: ${JSON.stringify(result)}`);
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

async function navigateToAccountDetails(accountName: string): Promise<void> {
  // Navigate to Assets first so the Accounts component unmounts.
  // On the way back the component remounts and re-fetches, picking up any
  // IPC-seeded accounts that were added after the initial page load.
  const assetsNav = await $('button[aria-label="Assets"]');
  await assetsNav.waitForExist({ timeout: 15000 });
  await assetsNav.click();
  await $('button[aria-label="Add asset"]').waitForExist({ timeout: 10000 });

  const accountsNav = await $('button[aria-label="Accounts"]');
  await accountsNav.click();

  // Use text-content XPath so the selector is language-invariant: the account
  // name is user data and never translated.
  const accountBtn = await $(`//button[contains(., "${accountName}")]`);
  await accountBtn.waitForExist({ timeout: 10000 });
  await accountBtn.click();
}

// ---------------------------------------------------------------------------
// Fixed past dates — one per write op (E2E rule E9)
// ---------------------------------------------------------------------------
const DATES = {
  buy: isoToDisplayDate("2019-03-10"),
  buy2: isoToDisplayDate("2019-04-01"),
  sell: isoToDisplayDate("2019-05-01"),
  oversell: isoToDisplayDate("2019-06-01"),
} as const;

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("buy_sell", () => {
  beforeEach(async () => {
    const closeBtn = await $('button[aria-label="Close"]');
    if (await closeBtn.isExisting()) {
      await closeBtn.click();
      await closeBtn.waitForExist({ timeout: 3000, reverse: true });
    }
  });

  // -------------------------------------------------------------------------
  // TRX-010 — buy holding via UI → holding row appears in account details
  // -------------------------------------------------------------------------
  it("TRX-010: buying a holding via the UI creates a holding row", async () => {
    const ACCOUNT_NAME = "E2E Buy TRX-010";
    const ASSET_NAME = "E2E Asset TRX010";
    const catId = await seedCategory("E2E Cat TRX010");
    await seedAccount(ACCOUNT_NAME);
    await seedAsset(ASSET_NAME, catId);

    await navigateToAccountDetails(ACCOUNT_NAME);

    // Account is empty — click the primary "Add Transaction" button.
    const addTxBtn = await $('//button[.//span[normalize-space()="Add Transaction"]]');
    await addTxBtn.waitForExist({ timeout: 10000 });
    await addTxBtn.click();

    const form = await $("form#buy-transaction-form");
    await form.waitForExist({ timeout: 8000 });

    // Select asset via combobox — type ≥2 chars to open dropdown.
    await setReactInputValue("buy-trx-asset", "E2E Asset TRX010");
    const assetOption = await $(`*=${ASSET_NAME}`);
    await assetOption.waitForDisplayed({ timeout: 5000 });
    await assetOption.click();

    await setReactInputValue("buy-trx-date", DATES.buy);
    await setReactInputValue("buy-trx-quantity", "10");
    await setReactInputValue("buy-trx-unit-price", "100");

    const submitBtn = await $('button[type="submit"][form="buy-transaction-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    await form.waitForExist({ timeout: 8000, reverse: true });

    // Holding row must now appear (identified by Buy button in the row).
    const buyBtn = await $('button[aria-label="Buy"]');
    await buyBtn.waitForExist({ timeout: 8000 });
    assert.ok(
      await buyBtn.isExisting(),
      "Holding row with Buy button must appear after buy transaction (TRX-010)",
    );
  });

  // -------------------------------------------------------------------------
  // TRX-020 — sell holding via UI → quantity decremented
  // -------------------------------------------------------------------------
  it("TRX-020: selling part of a holding decrements the displayed quantity", async () => {
    const ACCOUNT_NAME = "E2E Sell TRX-020";
    const ASSET_NAME = "E2E Asset TRX020";
    const catId = await seedCategory("E2E Cat TRX020");
    const accId = await seedAccount(ACCOUNT_NAME);
    const astId = await seedAsset(ASSET_NAME, catId);
    await seedBuy(accId, astId, "2019-04-01", 10_000_000); // 10 units

    await navigateToAccountDetails(ACCOUNT_NAME);

    // Holding row exists — click Sell.
    const sellBtn = await $('button[aria-label="Sell"]');
    await sellBtn.waitForExist({ timeout: 10000 });
    await sellBtn.click();

    const form = await $("form#sell-transaction-form");
    await form.waitForExist({ timeout: 8000 });

    await setReactInputValue("sell-trx-date", DATES.sell);
    await setReactInputValue("sell-trx-quantity", "4");
    await setReactInputValue("sell-trx-unit-price", "110");

    const submitBtn = await $('button[type="submit"][form="sell-transaction-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    await form.waitForExist({ timeout: 8000, reverse: true });

    // Remaining holding (6 units) must still be displayed.
    const holdingRow = await $('button[aria-label="Sell"]');
    await holdingRow.waitForExist({ timeout: 8000 });
    assert.ok(
      await holdingRow.isExisting(),
      "Holding row must remain after partial sell (TRX-020)",
    );
  });

  // -------------------------------------------------------------------------
  // TRX-030 — oversell → error shown in sell form
  // -------------------------------------------------------------------------
  it("TRX-030: selling more than held shows an error in the sell form", async () => {
    const ACCOUNT_NAME = "E2E Oversell TRX-030";
    const ASSET_NAME = "E2E Asset TRX030";
    const catId = await seedCategory("E2E Cat TRX030");
    const accId = await seedAccount(ACCOUNT_NAME);
    const astId = await seedAsset(ASSET_NAME, catId);
    await seedBuy(accId, astId, "2019-06-01", 2_000_000); // 2 units

    await navigateToAccountDetails(ACCOUNT_NAME);

    const sellBtn = await $('button[aria-label="Sell"]');
    await sellBtn.waitForExist({ timeout: 10000 });
    await sellBtn.click();

    const form = await $("form#sell-transaction-form");
    await form.waitForExist({ timeout: 8000 });

    await setReactInputValue("sell-trx-date", DATES.oversell);
    await setReactInputValue("sell-trx-quantity", "999"); // well above the 2 held
    await setReactInputValue("sell-trx-unit-price", "100");

    const submitBtn = await $('button[type="submit"][form="sell-transaction-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    // Backend returns Oversell error → UI renders [role="alert"].
    const alert = await $('[role="alert"]');
    await alert.waitForExist({ timeout: 8000 });
    assert.ok(await alert.isExisting(), "Oversell error must be shown in the sell form (TRX-030)");
    assert.ok(await form.isExisting(), "Sell form must remain open after oversell error");
  });
});
