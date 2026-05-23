import type { AccountApplicationError, AccountCrudError, UpdateFrequency } from "@/bindings";
import type { I18nMessage } from "@/ui/format/i18n";

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
