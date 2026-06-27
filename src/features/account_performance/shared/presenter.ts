import type { AccountError, PerformanceMetric, PerformancePeriod } from "@/bindings";
import { microToFormatted } from "@/lib/microUnits";
import type { I18nMessage } from "@/ui/format/i18n";

const DASH = "—";

const MONTH_LABEL_KEYS = [
  "account_performance.month.january",
  "account_performance.month.february",
  "account_performance.month.march",
  "account_performance.month.april",
  "account_performance.month.may",
  "account_performance.month.june",
  "account_performance.month.july",
  "account_performance.month.august",
  "account_performance.month.september",
  "account_performance.month.october",
  "account_performance.month.november",
  "account_performance.month.december",
] as const;

/**
 * F27 — Maps an account-performance read error to its i18n key. Pure function:
 * no React, no useTranslation. Handles the two reachable codes
 * (`AccountNotFound`, `DatabaseError`); any other code in the BC-wide
 * `AccountError` union falls back to the generic database-error key.
 */
export function presentAccountPerformanceError(error: AccountError): I18nMessage {
  switch (error.code) {
    case "AccountNotFound":
      return { key: "account_performance.error.account_not_found" };
    case "DatabaseError":
      return { key: "account_performance.error.database_error" };
    default:
      return { key: "account_performance.error.database_error" };
  }
}

/** PRF-020 — Formats a period-end Global Value (account-currency micros) for display. */
export function formatEndValue(endValue: number): string {
  return microToFormatted(endValue, 2);
}

/** PRF-036 / PRF-042 — Formats a metric gain (account-currency micros), or "—" when absent. */
export function formatMetricGain(metric: PerformanceMetric | null): string {
  if (metric === null) return DASH;
  return microToFormatted(metric.gain, 2);
}

/** PRF-036 / PRF-032 / PRF-042 — Formats a metric percentage (micro-percent), or "—" when absent. */
export function formatMetricPct(metric: PerformanceMetric | null): string {
  if (metric === null || metric.pct === null) return DASH;
  return `${microToFormatted(metric.pct, 2)}%`;
}

/**
 * PRF-036 — Sign-based colour class for a metric gain. Neutral when the metric
 * is absent (PRF-042) or the gain is zero; distinct positive and negative classes.
 */
export function gainColorClass(metric: PerformanceMetric | null): string {
  if (metric === null || metric.gain === 0) return "text-m3-on-surface";
  return metric.gain > 0 ? "text-m3-success" : "text-m3-error";
}

/** PRF-015 — Returns the i18n key for a month number 1–12. */
export function monthLabel(month: number): string {
  return MONTH_LABEL_KEYS[month - 1] ?? MONTH_LABEL_KEYS[0];
}

/**
 * PRF-070 / PRF-071 / PRF-073 — Sign-based colour class for a bridge flow or P&L
 * amount (micros). Neutral when zero; distinct positive and negative classes.
 */
export function pnlColorClass(amount: number): string {
  if (amount === 0) return "text-m3-on-surface";
  return amount > 0 ? "text-m3-success" : "text-m3-error";
}

/** Formatted + colourised view of a sign-bearing bridge cell (cash/asset flow or P&L). */
export interface PnlCellViewModel {
  formatted: string;
  colorClass: string;
}

function toPnlCell(amount: number): PnlCellViewModel {
  return { formatted: microToFormatted(amount, 2), colorClass: pnlColorClass(amount) };
}

/** Formatted + colourised view of a single performance metric cell. */
export interface MetricCellViewModel {
  gainFormatted: string;
  pctFormatted: string;
  colorClass: string;
}

function toMetricCell(metric: PerformanceMetric | null): MetricCellViewModel {
  return {
    gainFormatted: formatMetricGain(metric),
    pctFormatted: formatMetricPct(metric),
    colorClass: gainColorClass(metric),
  };
}

export interface PeriodRowViewModel {
  /** Stable row key used for the `data-testid` (year for year rows, year-month for month rows). */
  rowKey: string;
  /** Calendar year of this row. */
  year: number;
  /** Some(1..=12) for month rows; null for year rows (PRF-011). */
  month: number | null;
  /** Display label: the month i18n key for month rows, the year string for year rows. */
  periodLabel: string;
  /** Formatted period-end Global Value (PRF-020). */
  endValueFormatted: string;
  /** Formatted previous period-end Global Value — the bridge baseline (PRF-074). */
  previousValueFormatted: string;
  /** Cash in/out within the period (PRF-070) — sign-coloured. */
  cashFlow: PnlCellViewModel;
  /** Asset in/out within the period (PRF-071) — sign-coloured. */
  assetFlow: PnlCellViewModel;
  /** Dividend income within the period (PRF-072). */
  dividendsFormatted: string;
  /** Investment P&L vs the previous period (PRF-073) — sign-coloured. */
  pnl: PnlCellViewModel;
  /** Period-over-period metric cell (PRF-033) — always present, "—" when absent (PRF-042). */
  periodOverPeriod: MetricCellViewModel;
  /** Year-to-date metric cell (PRF-034) — present only for month rows; omitted for year rows (PRF-037). */
  yearToDate?: MetricCellViewModel;
  /** Since-inception metric cell (PRF-035) — always present. */
  sinceInception: MetricCellViewModel;
}

/**
 * PRF-036 / PRF-037 / PRF-041 / PRF-042 — Maps a backend PerformancePeriod to a
 * row view model. Year rows (month === null) omit the year-to-date cell.
 */
export function presentPeriodRow(period: PerformancePeriod): PeriodRowViewModel {
  const isYearRow = period.month === null;
  return {
    rowKey: isYearRow ? String(period.year) : `${period.year}-${period.month}`,
    year: period.year,
    month: period.month,
    periodLabel: period.month !== null ? monthLabel(period.month) : String(period.year),
    endValueFormatted: formatEndValue(period.end_value),
    previousValueFormatted: formatEndValue(period.previous_value),
    cashFlow: toPnlCell(period.cash_flow),
    assetFlow: toPnlCell(period.asset_flow),
    dividendsFormatted: microToFormatted(period.dividends, 2),
    pnl: toPnlCell(period.pnl),
    periodOverPeriod: toMetricCell(period.period_over_period),
    yearToDate: isYearRow ? undefined : toMetricCell(period.year_to_date),
    sinceInception: toMetricCell(period.since_inception),
  };
}
