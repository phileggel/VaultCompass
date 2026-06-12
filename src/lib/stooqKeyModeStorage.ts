const STOOQ_USE_API_KEY = "stooq_use_api_key";

/**
 * KEY-050 — Read the Stooq fetch-mode preference from localStorage.
 * Returns true (keyed, BYOK) when the key is absent — keyed is the default
 * and the robust path (KEY-054); false means keyless/anonymous fetching.
 */
export function getUseStooqApiKey(): boolean {
  return localStorage.getItem(STOOQ_USE_API_KEY) !== "false";
}

/**
 * KEY-050 — Persist the Stooq fetch-mode preference to localStorage.
 */
export function setUseStooqApiKey(enabled: boolean): void {
  localStorage.setItem(STOOQ_USE_API_KEY, String(enabled));
}
