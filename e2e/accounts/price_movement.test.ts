/**
 * E2E tests — Price Movement (PMV)
 *
 * Spec:     docs/spec/price-movement.md (PMV-010–061)
 * Contract: docs/contracts/asset-contract.md § "Asset Price Fetch Tasks",
 *           § Shared Types PriceMovementReport / PriceMovementRow,
 *           § Events AssetPriceFetchCompleted
 * Plan:     docs/plan/price-movement-plan.md §2.4
 *
 * Spec rules exercised by this file:
 *   PMV-010/015 — a Global refresh (trigger="Manual") is the only fetch path
 *                 that produces a report; the panel appearing at all proves
 *                 the trigger reached the backend and the report round-tripped
 *                 through the real event bus, not a mock.
 *   PMV-013     — the report appears as a dismissible panel on the accounts
 *                 list, coexisting with (never queuing behind) the unupdated-
 *                 prices modal (MKT-172).
 *   PMV-032/060 — an account whose only holding could never be priced is
 *                 marked incomplete; when that leaves nothing moved in the
 *                 whole portfolio the panel states that plainly, together with
 *                 the incompleteness sentence, instead of an empty table.
 *   PMV-061     — dismissing the panel removes it and it does not return.
 *
 * Seed strategy (determinism):
 *   Each E2E spec file runs in its own session against a fresh ephemeral DB
 *   (wdio.conf.ts beforeSession/afterSession — new VAULT_COMPASS_E2E_DATA_DIR
 *   per worker), so no other test file's accounts are present when this
 *   file's Global refresh runs; the report's rows are exactly what this file
 *   seeds.
 *
 *   One account holding one asset whose reference (ZZ-NOPE-PMV) Yahoo Finance
 *   cannot resolve (no exchange set, so MKT-110 branch 2 uses the bare
 *   reference as the symbol — the same technique as
 *   account_details/manual_price_fill.test.ts). Yahoo returns no data for it
 *   either online or offline (offline: the request times out), so the asset
 *   always lands in the MKT-114 skip set and the account's reading is always
 *   marked incomplete (PMV-032), regardless of network state. The holding has
 *   never had a price recorded, so both the "before" and "after" readings
 *   value it at 0 (a missing price contributes 0) and the account's value
 *   cannot move — "nothing moved" (PMV-060) is therefore the deterministic
 *   panel state, online or offline.
 *
 * Why one scenario:
 *   The core cross-layer contract at E2E is:
 *     click Global refresh → trigger="Manual" reaches Rust → real
 *     capture/fetch/report pipeline → AssetPriceFetchCompleted carries a real
 *     PriceMovementReport → panel renders it → dismiss discards it for good.
 *   A single scenario that traverses this full path, while also observing
 *   that the panel coexists with the unrelated MKT-172 modal rather than
 *   queuing behind it, covers the critical integration points without
 *   duplicating the existing Vitest-level coverage of the 31 PMV rules (17
 *   panel-rendering tests, 8 hook tests) or the Rust resolution matrix (1263
 *   lib tests).
 */

import assert from "node:assert";
import { $ } from "@wdio/globals";
import { dismissLeftoverModal } from "../helpers/modal";
import { navigateToAccounts, navigateToAssets } from "../helpers/navigation";
import { seedAccount, seedAsset, seedBuy, seedCategory } from "../helpers/seed";

// ---------------------------------------------------------------------------
// Timeout constants — explicit on every wait call (E2E rule E10)
// ---------------------------------------------------------------------------

/**
 * Upper bound for the panel to appear after clicking "Refresh prices".
 * Mirrors MODAL_APPEARS_TIMEOUT in manual_price_fill.test.ts: the fetch task
 * calls Yahoo once for the single bogus symbol (10 s per-request timeout,
 * yahoo_client.rs) plus IPC + event propagation plus the "before"/"after"
 * valuation reads. In practice this completes in a few seconds because the
 * bogus symbol fails fast; the wide ceiling guards offline CI where the TCP
 * handshake itself may time out.
 */
const PANEL_APPEARS_TIMEOUT = 35_000;

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("price_movement", () => {
  let accountId: string;
  let assetId: string;

  // Seed shared prerequisites once via IPC — no UI interaction needed for
  // setup (seed in before(), never inside it() blocks).
  before(async () => {
    const catId = await seedCategory("E2E Cat PMV");
    accountId = await seedAccount("E2E PMV Account");

    // Bogus reference guarantees Yahoo can never resolve it (see file header).
    assetId = await seedAsset("E2E PMV Unresolvable Asset", catId, {
      reference: "ZZ-NOPE-PMV",
    });

    // Buy 10 units so the asset is an active holding in the Global refresh's
    // scope (MKT-130). seedBuy also seeds the deposit CSH-041 requires, so the
    // account carries a cash holding too — system cash is excluded from fetch
    // scope (MKT-116) and never contributes to incompleteness (PMV-032).
    await seedBuy(accountId, assetId, "2020-06-01", 10);
  });

  beforeEach(async () => {
    await dismissLeftoverModal();
    await navigateToAccounts();
  });

  it("PMV-010/013/032/060/061: Global refresh reports nothing moved, marks it incomplete, coexists with the unpriced-prices modal, and dismisses for good", async () => {
    // -----------------------------------------------------------------------
    // Step 1 — Trigger the Global refresh (PMV-010/015: trigger="Manual").
    //   The button already exists on the accounts list header.
    // -----------------------------------------------------------------------
    const refreshBtn = await $("#account-manager-refresh-prices");
    await refreshBtn.waitForExist({ timeout: 10_000 });
    await refreshBtn.click();

    // -----------------------------------------------------------------------
    // Step 2 — Wait for the panel to appear (PMV-013). Its mere appearance
    //   proves the real round trip: the trigger reached Rust, the fetch task
    //   ran, AssetPriceFetchCompleted carried a real PriceMovementReport, and
    //   the FE rendered it from the live event — no mocking at any layer.
    // -----------------------------------------------------------------------
    const panel = await $("#price-movement-panel");
    await panel.waitForExist({ timeout: PANEL_APPEARS_TIMEOUT });

    // PMV-060 — nothing moved (the only holding never had a price, so both
    //   readings value it at 0); PMV-032 — that account is incomplete, so the
    //   panel states both in one sentence rather than an empty/zeroed table.
    const panelText = await panel.getText();
    assert.ok(
      panelText.includes("No account's value changed") &&
        panelText.includes("could not be read at their current price"),
      `Panel must state both that nothing moved and that a holding was incomplete (got: "${panelText}")`,
    );

    // No table is rendered in the nothing-moved state — the per-account row
    // and total row must be absent (distinguishes this from the Moved state).
    assert.ok(
      !(await $(`#price-movement-row-${accountId}`).isExisting()),
      "Nothing-moved state must not render a per-account row",
    );
    assert.ok(
      !(await $("#price-movement-total").isExisting()),
      "Nothing-moved state must not render a total row",
    );

    // -----------------------------------------------------------------------
    // Step 3 — PMV-013: the panel coexists with the unupdated-prices modal
    //   (MKT-172), which auto-opens because the same fetch left the asset
    //   unpriced. Neither displaces nor queues behind the other.
    // -----------------------------------------------------------------------
    const unpricedRow = await $(`#unpriced-row-${assetId}`);
    await unpricedRow.waitForExist({ timeout: 8_000 });
    assert.ok(
      await panel.isExisting(),
      "Price-movement panel must still be present while the unpriced-prices modal is open",
    );

    // Close the modal (skip the only row — MKT-176/177 closes it
    // automatically once every row is resolved) so the rest of the page is
    // interactable again for the next step.
    const skipBtn = await $(`#unpriced-skip-${assetId}`);
    await skipBtn.waitForExist({ timeout: 5_000 });
    await skipBtn.click();
    await unpricedRow.waitForExist({ timeout: 8_000, reverse: true });

    const dialog = await $('[role="dialog"]');
    await dialog.waitForExist({ timeout: 8_000, reverse: true });

    // The panel is untouched by the modal closing — it belongs to the
    // accounts list, not to the modal's lifecycle (PMV-013).
    assert.ok(
      await panel.isExisting(),
      "Price-movement panel must remain after the unpriced-prices modal closes",
    );

    // -----------------------------------------------------------------------
    // Step 4 — PMV-061: dismiss the panel; it is gone for good.
    // -----------------------------------------------------------------------
    const dismissBtn = await $("#price-movement-dismiss");
    await dismissBtn.waitForExist({ timeout: 5_000 });
    await dismissBtn.click();
    await panel.waitForExist({ timeout: 5_000, reverse: true });

    // Navigating away and back must not resurrect it — the report is held in
    // mount-scoped state with no persistence and no restore path (PMV-061).
    await navigateToAssets();
    await navigateToAccounts();
    assert.ok(
      !(await $("#price-movement-panel").isExisting()),
      "Dismissed panel must not reappear after navigating away and back",
    );
  });
});
