/**
 * E2E tests — Opening Balance (open_holding)
 *
 * Contract: docs/contracts/record_transaction-contract.md (open_holding command)
 * Spec rules covered:
 *   TRX-055 — modal accessible from Account Details header "Open Balance" button
 *   TRX-042 — form exposes date, quantity, total-cost only (no fees/exchange-rate/unit-price)
 *   TRX-043 — happy path: fill all fields + select asset → submit → transaction created, modal closes
 *   TRX-046 — future date keeps submit button disabled (frontend guard in useOpenBalance.isFormValid)
 *   TRX-044 — QuantityNotPositive: backend rejects quantity=0 (exercised via IPC; frontend guard
 *             prevents the form from submitting so the error path is verified at the IPC layer)
 *
 * Seed strategy: account + asset + one buy_holding seeded via IPC in before() so the
 * Account Details header renders the "Open Balance" button (non-empty, non-all-closed state).
 */

import assert from "node:assert";
import { $, browser } from "@wdio/globals";
import { isoToDisplayDate } from "../helpers/date";
import { setReactInputValue } from "../helpers/react";
import { seedAccount, seedAsset, seedCategory } from "../helpers/seed";

// ---------------------------------------------------------------------------
// Fixed past dates — one constant per write operation (E2E rule E9)
// ---------------------------------------------------------------------------
const DATES = {
  openBalance: isoToDisplayDate("2020-06-15"),
} as const;

/**
 * Seed a buy_holding so the account has an active (non-closed) holding.
 * Required for the "Open Balance" button to appear in the header
 * (only shown when !isEmpty && !isAllClosed — AccountDetailsView.tsx:126).
 */
async function seedBuyHolding(accountId: string, assetId: string, date: string): Promise<void> {
  type SeedResult = { id: string } | { __error: string };
  const result = (await browser.executeAsync(
    (accId: string, astId: string, d: string, done: (r: SeedResult) => void) => {
      // @ts-expect-error __TAURI_INTERNALS__ is injected by the Tauri WebView
      window.__TAURI_INTERNALS__
        .invoke("buy_holding", {
          dto: {
            account_id: accId,
            asset_id: astId,
            date: d,
            quantity: 10_000_000, // 10 units in micro-units
            unit_price: 100_000_000, // 100.00 per unit in micro-units
            exchange_rate: 1_000_000, // 1:1 rate in micro-units
            fees: 0,
            note: "", // empty string avoids Tauri 2 null-deserialization issue for Option<String>
          },
        })
        .then((r: SeedResult) => done(r))
        .catch((err: unknown) => done({ __error: JSON.stringify(err) }));
    },
    accountId,
    assetId,
    date,
  )) as SeedResult;
  if ("__error" in result) {
    throw new Error(`seedBuyHolding failed: ${result.__error}`);
  }
}

// ---------------------------------------------------------------------------
// Navigation helper — Accounts list → Account Details (E2E rule E8: no browser.url())
// ---------------------------------------------------------------------------

async function navigateToAccountDetails(accountName: string): Promise<void> {
  // Navigate to the Accounts section via the sidebar nav button.
  const accountsNav = await $('button[aria-label="Accounts"]');
  await accountsNav.waitForExist({ timeout: 15000 });
  await accountsNav.click();

  // Confirm Accounts page is active — wait for the FAB.
  const fab = await $('button[aria-label="Add account"]');
  await fab.waitForExist({ timeout: 10000 });

  // Click the named account row — aria-label is "Open account {name}"
  // (en/common.json key account.open_account = "Open account {{name}}"; AccountTable.tsx:165).
  // WebKitGTK: <tr> elements are not interactable via .click(); click the span inside td.
  const accountNameSpan = await $(
    `tr[aria-label="Open account ${accountName}"] td:first-child span`,
  );
  await accountNameSpan.waitForExist({ timeout: 10000 });
  await accountNameSpan.click();

  // Confirm Account Details view has loaded — the account details container div
  // is always rendered once the route activates (before the async fetch resolves).
  // Wait for the summary header area which holds the "Open Balance" button.
  // The button text is "Open Balance" (en/common.json account_details.action_open_balance).
  // The account has an active holding (seeded via buy_holding), so the button renders
  // once get_account_details resolves with isEmpty=false && isAllClosed=false.
  const summaryHeader = await $("div.px-6.py-4.bg-m3-surface-container-high");
  await summaryHeader.waitForExist({ timeout: 10000 });

  // The "Open Balance" button is inside the summary header — wait for it.
  // Use XPath contains() to tolerate any whitespace introduced by the Button
  // component's <span> wrapper: //button[.//span[normalize-space()='Open Balance']]
  const openBalanceBtn = await $('//button[.//span[normalize-space()="Open Balance"]]');
  await openBalanceBtn.waitForExist({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("open_balance", () => {
  let accountId: string;
  let assetId: string;

  // Stable names — reused across all tests in the suite.
  const ACCOUNT_NAME = "E2E OB Account";
  const ASSET_NAME = "E2E OB Asset";

  // Seed once — shared by all it() blocks (E2E rule: seed in before(), not it()).
  before(async () => {
    const categoryId = await seedCategory("E2E OB Category");
    accountId = await seedAccount(ACCOUNT_NAME, "EUR");
    assetId = await seedAsset(ASSET_NAME, categoryId, { reference: "E2E-OB-REF" });
    // Make the account non-empty so the "Open Balance" button is visible.
    // The ids are stored in closure variables and passed explicitly to avoid
    // any serialization issue with executeAsync argument passing.
    const storedAccountId = accountId;
    const storedAssetId = assetId;
    await seedBuyHolding(storedAccountId, storedAssetId, "2019-12-01");
  });

  // Navigate to Account Details and dismiss any leftover modal before each test.
  beforeEach(async () => {
    const closeBtn = await $('button[aria-label="Close"]');
    if (await closeBtn.isExisting()) {
      await closeBtn.click();
      await closeBtn.waitForExist({ timeout: 3000, reverse: true });
    }
    await navigateToAccountDetails(ACCOUNT_NAME);
  });

  // -------------------------------------------------------------------------
  // TRX-055 — "Open Balance" button in Account Details header opens the modal
  // -------------------------------------------------------------------------
  it("TRX-055: clicking Open Balance header button opens ob-form modal", async () => {
    const openBalanceBtn = await $('//button[.//span[normalize-space()="Open Balance"]]');
    await openBalanceBtn.waitForExist({ timeout: 10000 });
    await openBalanceBtn.click();

    const form = await $("form#ob-form");
    await form.waitForExist({ timeout: 8000 });
    assert.ok(await form.isExisting(), "form#ob-form must be present after clicking Open Balance");
  });

  // -------------------------------------------------------------------------
  // TRX-042 — form contains date, quantity, total-cost; no fees/exchange-rate
  // -------------------------------------------------------------------------
  it("TRX-042: ob-form exposes account, asset-select, date, quantity, total-cost — no fees", async () => {
    const openBalanceBtn = await $('//button[.//span[normalize-space()="Open Balance"]]');
    await openBalanceBtn.waitForExist({ timeout: 10000 });
    await openBalanceBtn.click();

    const form = await $("form#ob-form");
    await form.waitForExist({ timeout: 8000 });

    // Required fields (TRX-042)
    for (const fieldId of [
      "ob-account",
      "ob-asset-select",
      "ob-date",
      "ob-quantity",
      "ob-total-cost",
    ]) {
      const field = await $(`#${fieldId}`);
      await field.waitForExist({ timeout: 5000 });
      assert.ok(await field.isExisting(), `#${fieldId} must be present in ob-form`);
    }

    // Fields that must NOT exist — opening balance omits fees, exchange-rate, unit-price
    const feesField = await $('[id$="-fees"]');
    assert.ok(!(await feesField.isExisting()), "fees input must NOT exist in ob-form (TRX-042)");

    const exchangeRateField = await $('[id$="-exchange-rate"]');
    assert.ok(
      !(await exchangeRateField.isExisting()),
      "exchange-rate input must NOT exist in ob-form (TRX-042)",
    );
  });

  // -------------------------------------------------------------------------
  // TRX-046 — future date disables submit (frontend isFormValid guard)
  // -------------------------------------------------------------------------
  it("TRX-046: submit button stays disabled when date is in the future", async () => {
    const openBalanceBtn = await $('//button[.//span[normalize-space()="Open Balance"]]');
    await openBalanceBtn.waitForExist({ timeout: 10000 });
    await openBalanceBtn.click();

    const form = await $("form#ob-form");
    await form.waitForExist({ timeout: 8000 });

    // Select asset so assetId is set in form state — isFormValid also requires assetId.
    await setReactInputValue("ob-asset-select", "E2E OB");
    const assetOption = await $(`*=${ASSET_NAME}`);
    await assetOption.waitForDisplayed({ timeout: 5000 });
    await assetOption.click();

    // Provide valid quantity and total cost so only the date prevents enabling.
    await setReactInputValue("ob-quantity", "5");
    await setReactInputValue("ob-total-cost", "100");

    // Set a date in the far future — DateField fr-FR format: DD/MM/YYYY.
    await setReactInputValue("ob-date", isoToDisplayDate("2099-12-31"));

    // Submit must remain disabled because date > today (TRX-046 frontend guard).
    const submitBtn = await $('button[type="submit"][form="ob-form"]');
    await submitBtn.waitForExist({ timeout: 5000 });

    const isEnabled = await submitBtn.isEnabled();
    assert.strictEqual(
      isEnabled,
      false,
      "Submit button must be disabled when date is in the future (TRX-046)",
    );
  });

  // -------------------------------------------------------------------------
  // TRX-043 — happy path: all fields filled → transaction created → modal closes
  // -------------------------------------------------------------------------
  it("TRX-043: happy path — fill form, submit, modal closes and Account Details refreshes", async () => {
    const openBalanceBtn = await $('//button[.//span[normalize-space()="Open Balance"]]');
    await openBalanceBtn.waitForExist({ timeout: 10000 });
    await openBalanceBtn.click();

    const form = await $("form#ob-form");
    await form.waitForExist({ timeout: 8000 });

    // Select the seeded asset via the combobox — type ≥2 chars to trigger dropdown
    // (ComboboxField.tsx:72 — showDropdown only when query.length >= 2).
    const assetInput = await $("#ob-asset-select");
    await assetInput.waitForExist({ timeout: 5000 });
    await setReactInputValue("ob-asset-select", "E2E OB");

    const assetOption = await $(`*=${ASSET_NAME}`);
    await assetOption.waitForDisplayed({ timeout: 5000 });
    await assetOption.click();

    // Set a fixed past date (E2E rule E9).
    await setReactInputValue("ob-date", DATES.openBalance);

    // Set positive quantity and total cost (TRX-044, TRX-045).
    await setReactInputValue("ob-quantity", "10");
    await setReactInputValue("ob-total-cost", "500");

    // waitForEnabled confirms React state has updated from all setReactInputValue calls (E2E rule E10).
    const submitBtn = await $('button[type="submit"][form="ob-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    // On success the modal closes (TRX-043 — handleOpenBalanceSuccess calls setOpenBalanceOpen(false)).
    await form.waitForExist({ timeout: 8000, reverse: true });
    assert.ok(
      !(await form.isExisting()),
      "form#ob-form must close after successful opening balance submission (TRX-043)",
    );
  });

  // -------------------------------------------------------------------------
  // TRX-044 — QuantityNotPositive: backend rejects quantity=0 via direct IPC
  //
  // The frontend isFormValid guard blocks qty <= 0 before submit, so we exercise
  // the backend constraint directly via IPC to confirm the Rust backend enforces
  // TRX-044 independently of the UI guard.
  // -------------------------------------------------------------------------
  it("TRX-044: open_holding IPC returns QuantityNotPositive for quantity=0", async () => {
    type IpcResult = { __error: string } | { code: string };
    const result = (await browser.executeAsync(
      (accId: string, astId: string, done: (r: IpcResult) => void) => {
        // @ts-expect-error __TAURI_INTERNALS__ is injected by the Tauri WebView
        window.__TAURI_INTERNALS__
          .invoke("open_holding", {
            dto: {
              account_id: accId,
              asset_id: astId,
              date: "2020-08-01",
              quantity: 0, // zero — must trigger QuantityNotPositive (TRX-044)
              total_cost: 100_000_000,
            },
          })
          .then((r: unknown) => done({ __error: `unexpected success: ${JSON.stringify(r)}` }))
          .catch((err: unknown) => done(err as IpcResult));
      },
      accountId,
      assetId,
    )) as IpcResult;

    assert.ok(
      "code" in result,
      `Expected a typed error from backend, got: ${JSON.stringify(result)}`,
    );
    assert.strictEqual(
      (result as { code: string }).code,
      "QuantityNotPositive",
      `Expected QuantityNotPositive error from backend, got: ${JSON.stringify(result)}`,
    );
  });
});
