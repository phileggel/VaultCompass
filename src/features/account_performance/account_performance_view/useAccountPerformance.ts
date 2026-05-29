import { useCallback, useEffect, useMemo, useState } from "react";
import type { AccountPerformanceResponse } from "@/bindings";
import { logger } from "@/lib/logger";
import type { I18nMessage } from "@/ui/format/i18n";
import { accountPerformanceGateway } from "../gateway";
import {
  type PeriodRowViewModel,
  presentAccountPerformanceError,
  presentPeriodRow,
} from "../shared/presenter";

export type PerformanceViewMode = "month" | "year";

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
        // PRF-014 — open in month view by default when month view is available.
        setViewMode(result.data.month_view_available ? "month" : "year");
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
      if (
        type === "TransactionUpdated" ||
        type === "AssetPriceUpdated" ||
        type === "AccountUpdated"
      ) {
        fetchPerformance();
      }
    });
    return () => {
      void unlistenPromise?.then((unlisten) => unlisten());
    };
  }, [fetchPerformance]);

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

  // PRF-015 — in month view, slice the monthly rows to the selected year.
  const rows = useMemo<PeriodRowViewModel[]>(() => {
    if (!data) return [];
    if (viewMode === "year") {
      return data.yearly.map(presentPeriodRow);
    }
    return data.monthly
      .filter((row) => selectedYear === null || row.year === selectedYear)
      .map(presentPeriodRow);
  }, [data, viewMode, selectedYear]);

  return {
    isLoading,
    error,
    retry: fetchPerformance,
    monthViewAvailable,
    isEmpty,
    viewMode,
    setViewMode,
    availableYears,
    selectedYear,
    setSelectedYear,
    rows,
  };
}
