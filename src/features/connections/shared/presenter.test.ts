import { describe, expect, it } from "vitest";
import type { ConnectionError, ProviderKeyTestOutcome, StorageTier } from "@/bindings";

// Import the presenter under test. This file does not exist yet; the import
// failing is the expected red state for this layer.
import { connectionErrorToI18n, storageTierToLabel, testOutcomeToUiState } from "./presenter";

describe("connection presenter — connectionErrorToI18n", () => {
  // F27: ConnectionError.code → i18n key; pure function, no React, no t()

  it("maps EmptyKey to its i18n key", () => {
    const err: ConnectionError = { code: "EmptyKey" };
    expect(connectionErrorToI18n(err)).toBe("connection.error.empty_key");
  });

  it("maps KeyStoreError to its i18n key", () => {
    const err: ConnectionError = { code: "KeyStoreError" };
    expect(connectionErrorToI18n(err)).toBe("connection.error.key_store_error");
  });
});

describe("connection presenter — testOutcomeToUiState", () => {
  // KEY-023: three test outcomes → distinct UI states (string discriminants)

  it("maps Accepted outcome to accepted UI state", () => {
    const outcome: ProviderKeyTestOutcome = "Accepted";
    expect(testOutcomeToUiState(outcome)).toBe("accepted");
  });

  it("maps Rejected outcome to rejected UI state", () => {
    const outcome: ProviderKeyTestOutcome = "Rejected";
    expect(testOutcomeToUiState(outcome)).toBe("rejected");
  });

  it("maps Unreachable outcome to unreachable UI state", () => {
    const outcome: ProviderKeyTestOutcome = "Unreachable";
    expect(testOutcomeToUiState(outcome)).toBe("unreachable");
  });
});

describe("connection presenter — storageTierToLabel", () => {
  // KEY-015: StorageTier → i18n label key; pure function

  it("maps OsKeychain to its i18n label key", () => {
    const tier: StorageTier = "OsKeychain";
    expect(storageTierToLabel(tier)).toBe("connection.tier.os_keychain");
  });

  it("maps SessionMemory to its i18n label key", () => {
    const tier: StorageTier = "SessionMemory";
    expect(storageTierToLabel(tier)).toBe("connection.tier.session_memory");
  });

  it("maps PlaintextFile to its i18n label key", () => {
    const tier: StorageTier = "PlaintextFile";
    expect(storageTierToLabel(tier)).toBe("connection.tier.plaintext_file");
  });
});
