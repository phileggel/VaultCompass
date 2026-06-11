/**
 * E2E tests — Connections dialog (BYOK API-key management)
 *
 * Contract: docs/contracts/connection-contract.md
 * Spec rules covered:
 *   KEY-016 — get_provider_connections → dialog shows "no key" status
 *   KEY-030 — sidebar "Connections" entry opens the Connections dialog
 *   KEY-031 — Connections dialog lists the Stooq provider row
 *   KEY-040 — price-refresh with no stored key opens the Connections dialog
 *   KEY-010 — save_provider_key → status flips to "key set" (Remove button appears)
 *   KEY-016 — persisted state survives dialog close/reopen
 *   KEY-013 — remove_provider_key → status returns to "no key" (Remove button gone)
 *   KEY-034 — remove requires explicit confirmation
 *
 * Deliberately NOT covered at this tier (and why):
 *
 *   KEY-021/023 (test_provider_key + inline test outcome) — test_provider_key performs
 *     a live network probe against stooq.com (PoW + apikey). E2E runs headless on CI
 *     with no live Stooq traffic; the outcome states (Accepted/Rejected/Unreachable)
 *     are covered by FE integration tests (ConnectionsModal.integration.test.tsx).
 *
 *   KEY-011/012/015 (storage-tier ladder + plaintext opt-in + tier label) — the
 *     headless Linux E2E host has no OS keychain (tier 1 unavailable); saves land in
 *     tier-2 session memory. The tier label and plaintext opt-in are covered by FE
 *     integration tests. Asserting a specific tier at E2E would couple the test to
 *     the execution environment.
 *
 *   KEY-040 via account_details refresh — the per-account gate
 *     (useRefreshAccountPrices) exercises the same gate logic as the global gate and
 *     is covered by FE unit tests. One gate scenario at E2E is sufficient.
 *
 *   KEY-014 (no secret in logs) — server-side tracing output; not observable via UI.
 *     Covered by reviewer-security pass.
 *
 *   KEY-017 (session-memory cleared on exit) — requires a full app restart; outside
 *     the scope of a single E2E session.
 *
 * Session-isolation note:
 *   A saved provider key persists for the app session (and across runs when an OS
 *   keychain is available). Every scenario that requires the no-key state therefore
 *   establishes it explicitly with an idempotent `removeProviderKey("Stooq")` —
 *   never by relying on test order. The save lifecycle is additionally
 *   self-cleaning (saves then removes).
 */

import assert from "node:assert";
import { $, browser } from "@wdio/globals";
import { dismissLeftoverModal } from "../helpers/modal";
import { navigateToAccountDetails, navigateToAccounts } from "../helpers/navigation";
import { setReactInputValue } from "../helpers/react";
import { removeProviderKey, seedAccount, seedDeposit } from "../helpers/seed";

// ---------------------------------------------------------------------------
// Navigation helper — open the Connections dialog via the sidebar entry
// (KEY-030). Waits for the Stooq key input field as the route-ready signal
// (the input always renders regardless of has_key state).
// ---------------------------------------------------------------------------

async function openConnectionsDialog(): Promise<void> {
  const nav = await $("#nav-connections");
  await nav.waitForExist({ timeout: 15000 });
  await nav.click();
  // Wait for the Stooq provider row key input — stable id, always present
  // when the modal is open (KEY-031/032).
  const keyInput = await $("#provider-key-input-Stooq");
  await keyInput.waitForExist({ timeout: 10000 });
}

async function closeConnectionsDialog(): Promise<void> {
  // FormModal header close button — data-testid="modal-close-btn", used by
  // the existing dismissLeftoverModal helper in the project.
  await browser.keys("Escape");
  const keyInput = await $("#provider-key-input-Stooq");
  await keyInput.waitForExist({ timeout: 5000, reverse: true });
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("connections", () => {
  beforeEach(async () => {
    await dismissLeftoverModal();
  });

  // -------------------------------------------------------------------------
  // KEY-030 + KEY-031 + KEY-016 (no-key initial state)
  //
  // Opens the Connections dialog via the sidebar and verifies the Stooq
  // provider row is visible with no stored key. The Remove button is absent
  // when has_key is false (KEY-034: Remove hidden on no-key row), which is the
  // stable-id proxy for the "No key" status state.
  //
  // The no-key baseline is established explicitly (idempotent remove) rather
  // than assumed from test order or a fresh store.
  // -------------------------------------------------------------------------
  it("KEY-030+KEY-031+KEY-016: sidebar opens Connections dialog; Stooq row shows no-key state", async () => {
    await removeProviderKey("Stooq");
    await openConnectionsDialog();

    // KEY-031 — the Stooq provider row key input is present.
    const keyInput = await $("#provider-key-input-Stooq");
    assert.ok(
      await keyInput.isExisting(),
      "Stooq provider row key input must be present in the Connections dialog",
    );

    // KEY-016 / KEY-034 proxy: Remove button is absent ⟺ has_key is false.
    // The button is rendered only when connection.has_key (see ConnectionsModal.tsx).
    const removeBtn = await $("#provider-Stooq-remove");
    assert.ok(
      !(await removeBtn.isExisting()),
      "Remove button must be absent when no key is stored (KEY-016 no-key state / KEY-034 hidden-when-no-key)",
    );

    await closeConnectionsDialog();
  });

  // -------------------------------------------------------------------------
  // KEY-040 — price-refresh gate with no stored key
  //
  // Clicks the global "Refresh prices" button on the AccountManager view with
  // no Stooq key stored. The hook (useRefreshGlobalPrices) detects the absent
  // key and navigates to ?modal=connections instead of dispatching a fetch.
  // Observable surface: the Connections dialog opens (Stooq key input appears).
  //
  // Precondition: no provider key is stored — established explicitly via the
  // idempotent remove, not via test order.
  // -------------------------------------------------------------------------
  it("KEY-040: Refresh prices with no Stooq key opens the Connections dialog", async () => {
    await removeProviderKey("Stooq");
    await navigateToAccounts();

    const refreshBtn = await $("#account-manager-refresh-prices");
    await refreshBtn.waitForExist({ timeout: 10000 });
    await refreshBtn.click();

    // After the gate detects no key, the URL becomes ?modal=connections and
    // ConnectionsModalMount renders the dialog. The Stooq key input appearing
    // is the stable observable proof.
    const keyInput = await $("#provider-key-input-Stooq");
    await keyInput.waitForExist({ timeout: 10000 });
    assert.ok(
      await keyInput.isExisting(),
      "Connections dialog must open when Refresh prices is clicked with no Stooq key (KEY-040)",
    );

    await closeConnectionsDialog();
  });

  // -------------------------------------------------------------------------
  // KEY-010 + KEY-016 + KEY-013 + KEY-034 — save / persist / remove lifecycle
  //
  // Self-cleaning: saves a key then removes it, restoring the no-key baseline.
  //
  // Step 1: open dialog, enter a key, click Save.
  //   Post-save wait is TIER-INDEPENDENT: with an OS keychain the row refreshes
  //   and Remove appears; without one (CI) the save lands in session memory and
  //   the row shows the KEY-012 plaintext opt-in instead — deliberately without
  //   refreshing the list until the user answers the offer. Waiting for either
  //   signal keeps the spec green on both hosts.
  // Step 2: close and reopen the dialog.
  //   Post-reopen assertion: Remove button present (KEY-016 persists) — the
  //   reopen reloads the list fresh, so this holds on every storage tier.
  // Step 3: click Remove → confirm → Remove button disappears (KEY-013/034).
  //
  // -------------------------------------------------------------------------
  it("KEY-010+KEY-016+KEY-013+KEY-034: save key → status persists → remove key → no-key state restored", async () => {
    await openConnectionsDialog();

    // Step 1 — enter a key value and save.
    // Use a syntactically valid-looking but non-live dummy key so save_provider_key
    // stores it (it validates non-empty only — KEY-010). No live Stooq probe.
    const TEST_KEY = "e2e-dummy-key-do-not-use";

    await setReactInputValue("provider-key-input-Stooq", TEST_KEY);

    // Save button should be enabled immediately (not gated on key length beyond
    // non-empty — the emptiness check is server-side via KEY-010).
    const saveBtn = await $("#provider-Stooq-save");
    await saveBtn.waitForEnabled({ timeout: 5000 });
    await saveBtn.click();

    // Post-save: wait for whichever tier-dependent signal the save produced —
    // Remove button (keychain tier) or plaintext opt-in offer (session tier).
    await browser.waitUntil(
      async () =>
        (await $("#provider-Stooq-remove").isExisting()) ||
        (await $("#provider-Stooq-plaintext").isExisting()),
      {
        timeout: 8000,
        timeoutMsg:
          "After a successful save, either the Remove button (keychain tier) or the " +
          "plaintext opt-in offer (session tier, KEY-012) must appear",
      },
    );

    // Step 2 — close and reopen to verify persistence (KEY-016 survives
    // dialog unmount/remount since get_provider_connections is called on open).
    await closeConnectionsDialog();
    await openConnectionsDialog();

    const removeBtnAfterReopen = await $("#provider-Stooq-remove");
    await removeBtnAfterReopen.waitForExist({ timeout: 8000 });
    assert.ok(
      await removeBtnAfterReopen.isExisting(),
      "Remove button must still be present after dialog close/reopen (KEY-016 persists across mounts)",
    );

    // Step 3 — remove the key.
    await removeBtnAfterReopen.click();

    // Confirmation panel appears. Click the confirm button.
    const confirmBtn = await $("#remove-confirm-ok");
    await confirmBtn.waitForExist({ timeout: 5000 });
    await confirmBtn.click();

    // Post-remove: Remove button disappears — proxy for has_key = false (KEY-013/016).
    await removeBtnAfterReopen.waitForExist({ timeout: 8000, reverse: true });
    assert.ok(
      !(await removeBtnAfterReopen.isExisting()),
      "Remove button must disappear after confirmed removal (KEY-013 clears all tiers → KEY-016 has_key=false)",
    );

    // Dialog should still be open with the no-key row visible.
    const keyInput = await $("#provider-key-input-Stooq");
    assert.ok(
      await keyInput.isExisting(),
      "Connections dialog must remain open after removal, showing the no-key row",
    );

    await closeConnectionsDialog();
  });

  // -------------------------------------------------------------------------
  // KEY-040 (account_details path) — per-account refresh gate
  //
  // Verifies that the per-account "Refresh prices" button on AccountDetailsView
  // also opens the Connections dialog when no Stooq key is stored.
  //
  // Precondition: no key stored — established explicitly via the idempotent
  // remove, not via test order.
  //
  // Seed: account + deposit so the account exists and has a cash holding.
  // The cash holding is excluded from fetch scope (MKT-116), but the gate
  // check (KEY-040) fires before fetch-scope derivation — it only inspects
  // whether a provider key exists.
  // -------------------------------------------------------------------------
  it("KEY-040: per-account Refresh prices with no Stooq key opens the Connections dialog", async () => {
    await removeProviderKey("Stooq");
    const accountId = await seedAccount("E2E KEY-040 Account");
    await seedDeposit(accountId, "2019-01-10", 500_000_000);

    await navigateToAccounts();
    await navigateToAccountDetails(accountId);

    const refreshBtn = await $("#account-details-refresh-prices");
    await refreshBtn.waitForExist({ timeout: 10000 });
    await refreshBtn.click();

    // Connections dialog must open.
    const keyInput = await $("#provider-key-input-Stooq");
    await keyInput.waitForExist({ timeout: 10000 });
    assert.ok(
      await keyInput.isExisting(),
      "Connections dialog must open when per-account Refresh prices is clicked with no Stooq key (KEY-040)",
    );

    await closeConnectionsDialog();
  });
});
