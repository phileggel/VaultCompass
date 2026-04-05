import type { UpdateFrequency } from "@/bindings";

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
