import type { AssetPriceError } from "@/bindings";
import type { I18nMessage } from "@/ui/format/i18n";

/**
 * F27 — maps a manual-record (`record_asset_price`) error to an i18n key plus
 * optional interpolation vars. Pure function: no React, no useTranslation.
 * Exhaustive switch on `code` so TypeScript flags any new variant at compile time.
 */
export function recordPriceErrorToI18n(err: AssetPriceError): I18nMessage {
  switch (err.code) {
    case "InvalidDateFormat":
      return { key: "error.InvalidDateFormat", vars: { date: err.date } };
    case "NotFound":
    case "Archived":
    case "DatabaseError":
    case "PriceNotFound":
    case "NotPositive":
    case "NonFinite":
    case "DateInFuture":
      return { key: `error.${err.code}` };
    default: {
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}
