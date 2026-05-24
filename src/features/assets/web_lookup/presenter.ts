import type { WebLookupApplicationError } from "@/bindings";

/**
 * Maps a `WebLookupApplicationError` to the i18n key for its inline copy.
 * Pure function — call `t(key)` at render time (F27).
 */
export function presentWebLookupError(error: WebLookupApplicationError): string {
  switch (error.code) {
    case "InvalidIsinFormat":
      return "asset.web_lookup.error_invalid_isin";
    case "RateLimited":
      return "asset.web_lookup.error_rate_limit";
    case "NetworkError":
      return "asset.web_lookup.error_network";
  }
}
