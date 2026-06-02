import { useCallback, useEffect, useMemo, useState } from "react";
import type { AccountDetailsResponse, HoldingDetail } from "@/bindings";
import { accountMutationErrorToI18n } from "@/features/accounts/shared/presenter";
import { logger } from "@/lib/logger";
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

  // CSH-092 — Cash row rendered first, ahead of the existing alphabetical sort.
  const holdings = useMemo<HoldingRowViewModel[]>(() => {
    if (!data) return [];
    const rows = data.holdings.map(toHoldingRow);
    const cashRows = rows.filter((r) => r.isCash);
    const otherRows = rows.filter((r) => !r.isCash);
    return [...cashRows, ...otherRows];
  }, [data]);

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
