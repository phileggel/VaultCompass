import type {
  AccountApplicationError,
  AccountCrudError,
  FetchAccountAssetPricesError,
  FetchAccountAssetPricesForDateError,
  FetchAllAssetPricesError,
  UpdateFrequency,
} from "@/bindings";
import { microToFormatted } from "@/lib/microUnits";
import type { I18nMessage, SnackbarMessage } from "@/ui/format/i18n";

/**
 * ACC-023 — formats an account's account-wide unrealized P&L (account-currency
 * micros) to 2 decimals; "—" when the account has no computable holding (`null`).
 */
export function formatAccountRowTotalUnrealizedPnl(pnl: number | null): string {
  return pnl === null ? "—" : microToFormatted(pnl, 2);
}

/**
 * ACC-024 — formats an account's year-to-date performance (micro-percent) to a
 * signed percentage (e.g. "+8,00%" / "-3,70%"); "—" when `null` (no baseline /
 * zero Dietz denominator).
 */
export function formatAccountRowYtdPerformancePct(pct: number | null): string {
  if (pct === null) return "—";
  const sign = pct >= 0 ? "+" : "";
  return `${sign}${microToFormatted(pct, 2)}%`;
}

/**
 * F27 — Maps any asset-price fetch error (per-account or all-accounts) to a
 * snackbar message + severity. Pure function, no React, no useTranslation.
 *
 * Covers `FetchAccountAssetPricesError | FetchAllAssetPricesError` — both
 * compose AssetError + AccountApplicationError + FetchPriceTask on the wire.
 *
 * reviewer-arch FP: severity is intentionally narrower than SnackbarVariant
 * (no "success") because an error presenter never returns success — the narrow
 * union documents that constraint at the type level. See PR #NN.
 */
export function fetchPriceErrorToI18n(
  err: FetchAccountAssetPricesError | FetchAllAssetPricesError,
): SnackbarMessage {
  switch (err.code) {
    case "FetchAlreadyRunning":
      return { key: "mkt.fetch_already_running", severity: "info" };
    case "NoFetchableHoldings":
      return { key: "mkt.fetch_no_holdings", severity: "info" };
    case "AccountNotFound":
      return { key: "error.AccountNotFound", severity: "error" };
    case "NameAlreadyExists":
    case "DatabaseError":
    case "UnknownError":
      return { key: "error.DatabaseError", severity: "error" };
    default: {
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}

/**
 * F27 — Maps a date-scoped price-fetch error to a snackbar message + severity.
 * Pure function, no React, no useTranslation.
 *
 * Covers `FetchAccountAssetPricesForDateError` — composes AssetError +
 * AccountApplicationError + FetchPriceForDateTask on the wire.
 */
export function fetchPriceForDateErrorToI18n(
  err: FetchAccountAssetPricesForDateError,
): SnackbarMessage {
  switch (err.code) {
    case "InvalidDate":
      return { key: "mkt.fetch_date_invalid", severity: "error" };
    case "DateInFuture":
      return { key: "mkt.fetch_date_future", severity: "error" };
    case "AccountNotFound":
      return { key: "error.AccountNotFound", severity: "error" };
    case "NameAlreadyExists":
    case "DatabaseError":
    case "UnknownError":
      return { key: "error.DatabaseError", severity: "error" };
    default: {
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}

/**
 * F27 — Maps any account-BC mutation error (add / update / delete / deletion-summary)
 * to an i18n key + interpolation vars. Pure function, no React, no useTranslation.
 *
 * Covers AccountCrudError (add/update) and AccountApplicationError (delete and the
 * pre-deletion summary lookup) — both unions share the same variant pool.
 */
export function accountMutationErrorToI18n(
  err: AccountCrudError | AccountApplicationError,
): I18nMessage {
  switch (err.code) {
    case "InvalidCurrency":
      return { key: "error.InvalidCurrency", vars: { currency: err.currency } };
    case "NameEmpty":
    case "AccountNotFound":
    case "NameAlreadyExists":
    case "DatabaseError":
      return { key: `error.${err.code}` };
    default: {
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}

// i18n keys for UpdateFrequency display labels
export const FREQUENCY_I18N_KEYS: Record<UpdateFrequency, string> = {
  Automatic: "account.frequency_automatic",
  ManualDay: "account.frequency_manual_day",
  ManualWeek: "account.frequency_manual_week",
  ManualMonth: "account.frequency_manual_month",
  ManualYear: "account.frequency_manual_year",
};

// Ordered list of all frequencies — derived from FREQUENCY_I18N_KEYS to stay in sync with Specta bindings
export const FREQUENCIES = Object.keys(FREQUENCY_I18N_KEYS) as UpdateFrequency[];

// R9 — logical sort order for UpdateFrequency (not alphabetical)
export const FREQUENCY_ORDER: Record<UpdateFrequency, number> = {
  Automatic: 0,
  ManualDay: 1,
  ManualWeek: 2,
  ManualMonth: 3,
  ManualYear: 4,
};
