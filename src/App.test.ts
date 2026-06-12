import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProviderConnection } from "@/bindings";

// Mock the gateways the launch helper reaches so no command.* call is made.
vi.mock("@/features/accounts/gateway", () => ({
  accountGateway: { fetchAllAssetPrices: vi.fn() },
}));
vi.mock("@/features/connections/gateway", () => ({
  connectionGateway: { getProviderConnections: vi.fn() },
}));
vi.mock("@/features/shell/gateway", () => ({
  shellGateway: { onMigrationError: vi.fn(() => Promise.resolve(() => {})) },
}));

import { accountGateway } from "@/features/accounts/gateway";
import { connectionGateway } from "@/features/connections/gateway";
import { setAutoFetch } from "@/lib/autoFetchStorage";
import { setUseStooqApiKey } from "@/lib/stooqKeyModeStorage";
// Import the testable helpers extracted from App.tsx per KEY-041 / KEY-052.
import { maybeLaunchAutoFetch, shouldLaunchFetch } from "./App";

describe("shouldLaunchFetch — KEY-041 launch auto-fetch gate", () => {
  // KEY-041 — returns false (skip) when no connections data is available
  it("returns false when connections list is empty", () => {
    const connections: ProviderConnection[] = [];
    expect(shouldLaunchFetch(connections)).toBe(false);
  });

  // KEY-041 — returns false when Stooq has no key
  it("returns false when Stooq has_key is false", () => {
    const connections: ProviderConnection[] = [
      { provider: "Stooq", has_key: false, active_tier: null },
    ];
    expect(shouldLaunchFetch(connections)).toBe(false);
  });

  // KEY-041 — returns true when Stooq has a key
  it("returns true when Stooq has_key is true", () => {
    const connections: ProviderConnection[] = [
      { provider: "Stooq", has_key: true, active_tier: "OsKeychain" },
    ];
    expect(shouldLaunchFetch(connections)).toBe(true);
  });

  // KEY-041 — returns true regardless of which storage tier the key lives in
  it("returns true when Stooq key is in SessionMemory tier", () => {
    const connections: ProviderConnection[] = [
      { provider: "Stooq", has_key: true, active_tier: "SessionMemory" },
    ];
    expect(shouldLaunchFetch(connections)).toBe(true);
  });

  // KEY-041 — returns true when Stooq key is in PlaintextFile tier
  it("returns true when Stooq key is in PlaintextFile tier", () => {
    const connections: ProviderConnection[] = [
      { provider: "Stooq", has_key: true, active_tier: "PlaintextFile" },
    ];
    expect(shouldLaunchFetch(connections)).toBe(true);
  });

  // KEY-041 — launch fetch skip is SILENT: no dialog opened, just returns false
  // (the dialog is only opened by KEY-040 on explicit user-triggered refresh)
  it("does not open any dialog — pure predicate returning false when no key", () => {
    const connections: ProviderConnection[] = [
      { provider: "Stooq", has_key: false, active_tier: null },
    ];
    // If shouldLaunchFetch were to throw or navigate, this test would fail.
    // The function must be a pure boolean predicate — no side effects.
    const result = shouldLaunchFetch(connections);
    expect(result).toBe(false);
    expect(typeof result).toBe("boolean");
  });
});

describe("maybeLaunchAutoFetch — MKT-121 / KEY-052 launch dispatch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear(); // keyed default; auto-fetch off
  });

  // MKT-120 — auto-fetch disabled: no dispatch at all.
  it("does not dispatch when auto-fetch is disabled", async () => {
    setAutoFetch(false);
    await maybeLaunchAutoFetch();
    expect(connectionGateway.getProviderConnections).not.toHaveBeenCalled();
    expect(accountGateway.fetchAllAssetPrices).not.toHaveBeenCalled();
  });

  // KEY-052 — keyless mode: dispatch anonymously WITHOUT the KEY-041 no-key skip,
  // never consulting the stored key.
  it("keyless mode dispatches anonymously without the key gate (KEY-052)", async () => {
    setAutoFetch(true);
    setUseStooqApiKey(false);
    vi.mocked(accountGateway.fetchAllAssetPrices).mockResolvedValue({ status: "ok", data: null });

    await maybeLaunchAutoFetch();

    expect(connectionGateway.getProviderConnections).not.toHaveBeenCalled();
    expect(accountGateway.fetchAllAssetPrices).toHaveBeenCalledWith(false);
  });

  // KEY-041 — keyed mode, no stored key: the launch skip applies, no dispatch.
  it("keyed mode with no key skips the launch fetch silently (KEY-041)", async () => {
    setAutoFetch(true); // keyed is the default (localStorage cleared)
    vi.mocked(connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: false, active_tier: null }],
    });

    await maybeLaunchAutoFetch();

    expect(connectionGateway.getProviderConnections).toHaveBeenCalledTimes(1);
    expect(accountGateway.fetchAllAssetPrices).not.toHaveBeenCalled();
  });

  // KEY-041 / KEY-054 — keyed mode with a stored key: dispatch with use_api_key=true.
  it("keyed mode with a stored key dispatches with the key (KEY-054)", async () => {
    setAutoFetch(true);
    vi.mocked(connectionGateway.getProviderConnections).mockResolvedValue({
      status: "ok",
      data: [{ provider: "Stooq", has_key: true, active_tier: "OsKeychain" }],
    });
    vi.mocked(accountGateway.fetchAllAssetPrices).mockResolvedValue({ status: "ok", data: null });

    await maybeLaunchAutoFetch();

    expect(accountGateway.fetchAllAssetPrices).toHaveBeenCalledWith(true);
  });
});
