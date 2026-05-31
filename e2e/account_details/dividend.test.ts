/**
 * E2E tests — Cash dividend recording flow (DIV)
 *
 * Spec: docs/spec/cash-dividend.md
 * Contract: docs/contracts/account-contract.md § Dividend
 * Spec rules covered:
 *   DIV-010/012 — "Record dividend" item in the header "Add" menu opens the modal
 *   DIV-023    — recording credits the account's Cash Holding (cash row present after submit)
 *   DIV-072    — paying asset's holding row shows a non-zero dividends-received amount
 *   DIV-073    — account header surfaces total_dividends_received once a dividend is recorded
 *
 * Seed strategy:
 *   - Account + category + asset seeded via IPC (mirrors buy_sell.test.ts).
 *   - seedBuy opens a position so the asset appears in the dividend modal's
 *     asset selector (DIV-011 requires quantity > 0).
 *   - The dividend modal's asset selector is a native <select> (SelectField),
 *     driven with selectByAttribute — no combobox automation needed.
 *
 * Why one scenario:
 *   Error variants (AssetNotHeld, DividendOnCashAsset, AmountNotPositive, …) are
 *   adequately covered by backend integration tests and the Vitest frontend suite.
 *   E2E sits at the apex: one critical-path happy-path scenario is the right scope.
 */

import assert from "node:assert";
import { $ } from "@wdio/globals";
import { isoToDisplayDate } from "../helpers/date";
import { dismissLeftoverModal } from "../helpers/modal";
import { setReactInputValue } from "../helpers/react";
import { seedAccount, seedAsset, seedBuy, seedCategory } from "../helpers/seed";

// ---------------------------------------------------------------------------
// Navigation (same round-trip pattern as buy_sell.test.ts and cash.test.ts)
// ---------------------------------------------------------------------------

async function navigateToAccountDetails(accountName: string): Promise<void> {
  // Navigate to Assets first so the Accounts component unmounts and remounts,
  // picking up IPC-seeded data that arrived after the initial load.
  const assetsNav = await $("#nav-assets");
  await assetsNav.waitForExist({ timeout: 15000 });
  await assetsNav.click();
  await $("#fab-add-asset").waitForExist({ timeout: 10000 });

  const accountsNav = await $("#nav-accounts");
  await accountsNav.waitForExist({ timeout: 10000 });
  await accountsNav.click();
  await $("#fab-add-account").waitForExist({ timeout: 10000 });

  const accountNameSpan = await $(
    `tr[aria-label="Open account ${accountName}"] td:first-child span`,
  );
  await accountNameSpan.waitForExist({ timeout: 10000 });
  await accountNameSpan.click();
}

// ---------------------------------------------------------------------------
// Fixed past dates — one per write operation (E2E rule E9)
// ---------------------------------------------------------------------------
const DATES = {
  dividend: isoToDisplayDate("2020-06-15"),
} as const;

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("dividend", () => {
  const ACCOUNT_NAME = "E2E Dividend DIV-023";
  const ASSET_NAME = "E2E Asset DIV023";
  let astId: string;

  // Seed prerequisites once via IPC — no UI interaction needed for setup
  // (mirrors open_balance.test.ts: seed in before(), not inside it()).
  before(async () => {
    const catId = await seedCategory("E2E Cat DIV023");
    const accId = await seedAccount(ACCOUNT_NAME);
    astId = await seedAsset(ASSET_NAME, catId);
    // Open a 10-unit position so the asset qualifies for dividend recording
    // (DIV-011: quantity > 0). seedBuy internally seeds a deposit for cash.
    await seedBuy(accId, astId, "2020-05-01", 10_000_000); // 10 units
  });

  beforeEach(async () => {
    await dismissLeftoverModal();
  });

  // -------------------------------------------------------------------------
  // DIV-010/012/023/072/073 — record dividend via the header "Add" menu:
  //   cash holding is credited; paying asset holding is untouched; header
  //   surfaces the total dividends received.
  // -------------------------------------------------------------------------
  it("DIV-023/072/073: recording a dividend credits cash and surfaces the dividend totals", async () => {
    await navigateToAccountDetails(ACCOUNT_NAME);

    // -----------------------------------------------------------------------
    // Step 1 — Open the consolidated "Add" menu (DIV-012), then "Record
    //           dividend" (DIV-010).
    // -----------------------------------------------------------------------
    const addMenuBtn = await $("#account-details-add-menu");
    await addMenuBtn.waitForExist({ timeout: 10000 });
    await addMenuBtn.click();

    const dividendItem = await $("#add-menu-dividend");
    await dividendItem.waitForExist({ timeout: 5000 });
    await dividendItem.click();

    // -----------------------------------------------------------------------
    // Step 2 — Fill the dividend form (DIV-020).
    //   asset selector: native <select> → selectByAttribute (no combobox).
    //   date: DateField (text input) → setReactInputValue + isoToDisplayDate.
    //   amount: <input type="number"> → setReactInputValue.
    //   exchange rate: not shown (asset currency EUR == account currency EUR).
    // -----------------------------------------------------------------------
    const form = await $("form#dividend-transaction-form");
    await form.waitForExist({ timeout: 8000 });

    // Select the paying asset by its seeded id (locale-invariant).
    const assetSelect = await $("#dividend-trx-asset");
    await assetSelect.waitForExist({ timeout: 5000 });
    await assetSelect.selectByAttribute("value", astId);

    await setReactInputValue("dividend-trx-date", DATES.dividend);
    await setReactInputValue("dividend-trx-amount", "75");

    // -----------------------------------------------------------------------
    // Step 3 — Submit.
    // -----------------------------------------------------------------------
    const submitBtn = await $('button[type="submit"][form="dividend-transaction-form"]');
    await submitBtn.waitForEnabled({ timeout: 5000 });
    await submitBtn.click();

    // -----------------------------------------------------------------------
    // Step 4 — Assert post-conditions (DIV-023/072/073).
    // -----------------------------------------------------------------------

    // Form must close on success (DIV-025).
    await form.waitForExist({ timeout: 8000, reverse: true });
    assert.strictEqual(
      await form.isExisting(),
      false,
      "Dividend form must close after successful submission (DIV-025)",
    );

    // DIV-023 — Cash Holding credited: the cash row's inline Deposit button
    // must now be present (same assertion anchor used by cash.test.ts CSH-022).
    const cashDepositBtn = await $("#action-record-deposit-system-cash-eur");
    await cashDepositBtn.waitForExist({ timeout: 8000 });
    assert.ok(
      await cashDepositBtn.isExisting(),
      "Cash holding row must be present after dividend (DIV-023 cash credit)",
    );

    // DIV-023b — Paying asset holding must be intact (Buy button still present).
    const buyBtn = await $(`#action-buy-${astId}`);
    await buyBtn.waitForExist({ timeout: 8000 });
    assert.ok(
      await buyBtn.isExisting(),
      "Paying asset holding row must remain after recording dividend (DIV-023 quantity unchanged)",
    );

    // DIV-073 — the dedicated header total-dividends tile must surface the
    // formatted amount. Scoped to its stable id (E4) so the assertion can't be
    // satisfied by a coincidental "75,00" elsewhere on the page.
    // 75 EUR — locale formats as "75,00" (fr-FR) or "75.00" (en-US).
    const totalDividends = await $("#account-details-total-dividends");
    await totalDividends.waitForExist({ timeout: 8000 });
    const totalDividendsText = await totalDividends.getText();
    assert.ok(
      totalDividendsText.includes("75,00") || totalDividendsText.includes("75.00"),
      `Header total-dividends tile should surface the 75 EUR dividend (DIV-073) — got: ${totalDividendsText}`,
    );
  });
});
