import { useCallback, useEffect, useMemo, useState } from "react";
import type { AccountPerformanceResponse } from "@/bindings";
import { logger } from "@/lib/logger";
import { getPerfViewMode, setPerfViewMode } from "@/lib/perfViewModeStorage";
import { useAppStore } from "@/lib/store";
import type { I18nMessage } from "@/ui/format/i18n";
import { accountPerformanceGateway } from "../gateway";
import {
  type PeriodRowViewModel,
  presentAccountPerformanceError,
  presentPeriodRow,
  presentValueChartSeries,
  type ValueChartPoint,
} from "../shared/presenter";

export type PerformanceViewMode = "month" | "year";

/**
 * PRF-014 — resolves the view mode to open with: the account's remembered choice when
 * still valid, clamped to year view when a remembered month view is no longer available,
 * else the default (month when available, otherwise year).
 */
function resolveViewMode(
  remembered: PerformanceViewMode | null,
  monthAvailable: boolean,
): PerformanceViewMode {
  if (remembered === "month" && !monthAvailable) return "year";
  if (remembered !== null) return remembered;
  return monthAvailable ? "month" : "year";
}

interface UseAccountPerformanceResult {
  isLoading: boolean;
  error: I18nMessage | null;
  retry: () => void;
  /** True only for Automatic/ManualDay/ManualWeek accounts (PRF-013). */
  monthViewAvailable: boolean;
  isEmpty: boolean;
  viewMode: PerformanceViewMode;
  setViewMode: (mode: PerformanceViewMode) => void;
  /** Years available in the monthly data, most-recent first (PRF-015). */
  availableYears: number[];
  selectedYear: number | null;
  setSelectedYear: (year: number) => void;
  /** Rows for the active view: yearly rows in year view, the selected year's months in month view. */
  rows: PeriodRowViewModel[];
  /** Account-value-over-time series for the line chart, chronological (oldest→newest). */
  chartPoints: ValueChartPoint[];
}

export function useAccountPerformance(accountId: string): UseAccountPerformanceResult {
  const [data, setData] = useState<AccountPerformanceResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<I18nMessage | null>(null);
  const [viewMode, setViewMode] = useState<PerformanceViewMode>("year");
  const [selectedYear, setSelectedYear] = useState<number | null>(null);

  const fetchPerformance = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await accountPerformanceGateway.getAccountPerformance(accountId);
      if (result.status === "ok") {
        setData(result.data);
        // PRF-014 — restore the account's remembered view mode (clamped to availability),
        // falling back to the default when there is no stored preference.
        setViewMode(resolveViewMode(getPerfViewMode(accountId), result.data.month_view_available));
        // PRF-015 — default the year selector to the most-recent year in the monthly data.
        const firstMonthlyYear = result.data.monthly[0]?.year ?? null;
        setSelectedYear(firstMonthlyYear);
      } else {
        logger.error("[useAccountPerformance] fetch failed", result.error);
        setError(presentAccountPerformanceError(result.error));
      }
    } catch (err) {
      logger.error("[useAccountPerformance] fetch threw", { error: err });
      setError({ key: "account_performance.error.database_error" });
    } finally {
      setIsLoading(false);
    }
  }, [accountId]);

  // PRF-014 — fetch on mount and on accountId change.
  useEffect(() => {
    fetchPerformance();
  }, [fetchPerformance]);

  // PRF-060 — re-fetch on TransactionUpdated, AssetPriceUpdated, or AccountUpdated.
  useEffect(() => {
    const unlistenPromise = accountPerformanceGateway.subscribeToEvents((type) => {
      // MKT-181 — coalesce per-asset events while a bulk price fetch runs.
      if (type === "AssetPriceUpdated" && useAppStore.getState().priceFetch.active) {
        return;
      }
      if (
        type === "TransactionUpdated" ||
        type === "AssetPriceUpdated" ||
        type === "AssetPriceFetchCompleted" ||
        type === "AccountUpdated"
      ) {
        fetchPerformance();
      }
    });
    return () => {
      void unlistenPromise?.then((unlisten) => unlisten());
    };
  }, [fetchPerformance]);

  // PRF-014 — remember the user's choice per account on every toggle.
  const selectViewMode = useCallback(
    (mode: PerformanceViewMode) => {
      setViewMode(mode);
      setPerfViewMode(accountId, mode);
    },
    [accountId],
  );

  const monthViewAvailable = data?.month_view_available ?? false;

  const isEmpty = useMemo(
    () => data !== null && data.yearly.length === 0 && data.monthly.length === 0,
    [data],
  );

  const availableYears = useMemo<number[]>(() => {
    if (!data) return [];
    const years = new Set<number>();
    for (const row of data.monthly) {
      years.add(row.year);
    }
    return [...years].sort((a, b) => b - a);
  }, [data]);

  // PRF-015 — the periods backing the active view: all yearly rows in year view,
  // the selected year's months in month view. Backend order (most-recent first).
  const activePeriods = useMemo(() => {
    if (!data) return [];
    if (viewMode === "year") {
      return data.yearly;
    }
    return data.monthly.filter((row) => selectedYear === null || row.year === selectedYear);
  }, [data, viewMode, selectedYear]);

  const rows = useMemo<PeriodRowViewModel[]>(
    () => activePeriods.map(presentPeriodRow),
    [activePeriods],
  );

  // Value-over-time series for the chart, chronological (oldest→newest) for the X axis.
  const chartPoints = useMemo<ValueChartPoint[]>(
    () => presentValueChartSeries(activePeriods),
    [activePeriods],
  );

  return {
    isLoading,
    error,
    retry: fetchPerformance,
    monthViewAvailable,
    isEmpty,
    viewMode,
    setViewMode: selectViewMode,
    availableYears,
    selectedYear,
    setSelectedYear,
    rows,
    chartPoints,
  };
}
