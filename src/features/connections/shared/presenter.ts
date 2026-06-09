import type { ConnectionError, ProviderKeyTestOutcome, StorageTier } from "@/bindings";

/**
 * Maps a `ConnectionError` to its i18n key (F27 — pure, no React, no `t()`).
 */
export function connectionErrorToI18n(error: ConnectionError): string {
  switch (error.code) {
    case "EmptyKey":
      return "connection.error.empty_key";
    case "KeyStoreError":
      return "connection.error.key_store_error";
  }
}

/** UI state for a key-test outcome (KEY-023). */
export type TestOutcomeUiState = "accepted" | "rejected" | "unreachable";

/**
 * Maps a provider key-test outcome to its UI state (KEY-023). The three outcomes
 * are distinct visible states, not errors.
 */
export function testOutcomeToUiState(outcome: ProviderKeyTestOutcome): TestOutcomeUiState {
  switch (outcome) {
    case "Accepted":
      return "accepted";
    case "Rejected":
      return "rejected";
    case "Unreachable":
      return "unreachable";
  }
}

/** Maps a storage tier to its i18n label key (KEY-015). */
export function storageTierToLabel(tier: StorageTier): string {
  switch (tier) {
    case "OsKeychain":
      return "connection.tier.os_keychain";
    case "SessionMemory":
      return "connection.tier.session_memory";
    case "PlaintextFile":
      return "connection.tier.plaintext_file";
  }
}
