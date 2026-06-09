import { describe, expect, it } from "vitest";
import type { ProviderConnection } from "@/bindings";

// Import the testable helper extracted from App.tsx per KEY-041.
// This import fails until App.tsx exports the helper — that is the expected red state.
import { shouldLaunchFetch } from "./App";

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
