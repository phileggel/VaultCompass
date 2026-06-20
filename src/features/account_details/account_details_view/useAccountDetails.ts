import { useCallback, useEffect, useMemo, useState } from "react";
import type { AccountDetailsResponse, HoldingDetail } from "@/bindings";
import { accountMutationErrorToI18n } from "@/features/accounts/shared/presenter";
import { logger } from "@/lib/logger";
import { useAppStore } from "@/lib/store";
import type { I18nMessage } from "@/ui/format/i18n";
import { accountDetailsGateway } from "../gateway";
import {
  type AccountSummaryViewModel,
  type ClosedHoldingRowViewModel,
  type HoldingRowViewModel,
  isCashAsset,
  toAccountSummary,
  toClosedHoldingRow,
  toHoldingRow,
} from "../shared/presenter";

const UNKNOWN_ERROR: I18nMessage = { key: "error.Unknown" };

interface UseAccountDetailsResult {
  isLoading: boolean;
  error: I18nMessage | null;
  retry: () => void;
  holdings: HoldingRowViewModel[];
  /** Raw active HoldingDetail records — used to pass to PriceModal (MKT-013). */
  holdingDetails: HoldingDetail[];
  closedHoldings: ClosedHoldingRowViewModel[];
  summary: AccountSummaryViewModel | null;
  /** True when the account currently shows a cash row in the active table (CSH-019/092/095). */
  hasVisibleCashRow: boolean;
}

export function useAccountDetails(accountId: string): UseAccountDetailsResult {
  const [data, setData] = useState<AccountDetailsResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<I18nMessage | null>(null);
  // ACD-051 — asset class is read from the loaded asset catalog to group holdings.
  const assets = useAppStore((state) => state.assets);

  const fetchDetails = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await accountDetailsGateway.getAccountDetails(accountId);
      if (result.status === "ok") {
        setData(result.data);
      } else {
        logger.error("[useAccountDetails] fetch failed", result.error);
        setError(accountMutationErrorToI18n(result.error));
      }
    } catch (err) {
      logger.error("[useAccountDetails] fetch threw", { error: err });
      setError(UNKNOWN_ERROR);
    } finally {
      setIsLoading(false);
    }
  }, [accountId]);

  // ACD-037 — fetch on mount and on accountId change
  useEffect(() => {
    fetchDetails();
  }, [fetchDetails]);

  // ACD-039/040/MKT-036/FXR-036 — re-fetch on TransactionUpdated, AssetUpdated,
  // AssetPriceUpdated, or CurrencyRateUpdated
  useEffect(() => {
    const unlistenPromise = accountDetailsGateway.subscribeToEvents((type) => {
      if (
        type === "TransactionUpdated" ||
        type === "AssetUpdated" ||
        type === "AssetPriceUpdated" ||
        type === "CurrencyRateUpdated"
      ) {
        fetchDetails();
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [fetchDetails]);

  const holdingDetails = useMemo<HoldingDetail[]>(() => data?.holdings ?? [], [data]);

  // ACD-051 / CSH-092 — group active holdings by asset class: cash first, then
  // Stocks, then every other class; alphabetical by asset_name within each group.
  const holdings = useMemo<HoldingRowViewModel[]>(() => {
    if (!data) return [];
    const classById = new Map(assets.map((a) => [a.id, a.class]));
    const groupRank = (row: HoldingRowViewModel): number => {
      if (row.isCash) return 0;
      return classById.get(row.assetId) === "Stocks" ? 1 : 2;
    };
    return data.holdings
      .map(toHoldingRow)
      .sort(
        (a, b) =>
          groupRank(a) - groupRank(b) ||
          a.assetName.toLowerCase().localeCompare(b.assetName.toLowerCase()),
      );
  }, [data, assets]);

  const closedHoldings = useMemo<ClosedHoldingRowViewModel[]>(
    () => (data ? data.closed_holdings.map(toClosedHoldingRow) : []),
    [data],
  );

  const summary = useMemo<AccountSummaryViewModel | null>(
    () => (data ? toAccountSummary(data) : null),
    [data],
  );

  // CSH-097 — backend filters cash holding when its quantity is 0 (ACD-020), so
  // any cash holding present in `holdings` is by definition visible.
  const hasVisibleCashRow = useMemo<boolean>(
    () => (data ? data.holdings.some((h) => isCashAsset(h.asset_id)) : false),
    [data],
  );

  return {
    isLoading,
    error,
    retry: fetchDetails,
    holdings,
    holdingDetails,
    closedHoldings,
    summary,
    hasVisibleCashRow,
  };
}
