import { useCallback, useEffect, useMemo, useState } from "react";
import type { AccountDetailsResponse, HoldingDetail } from "@/bindings";
import { logger } from "@/lib/logger";
import { accountDetailsGateway } from "../gateway";
import {
  type AccountSummaryViewModel,
  type ClosedHoldingRowViewModel,
  type HoldingRowViewModel,
  toAccountSummary,
  toClosedHoldingRow,
  toHoldingRow,
} from "../shared/presenter";

interface UseAccountDetailsResult {
  isLoading: boolean;
  error: string | null;
  retry: () => void;
  holdings: HoldingRowViewModel[];
  /** Raw active HoldingDetail records — used to pass to PriceModal (MKT-013). */
  holdingDetails: HoldingDetail[];
  closedHoldings: ClosedHoldingRowViewModel[];
  summary: AccountSummaryViewModel | null;
}

export function useAccountDetails(accountId: string): UseAccountDetailsResult {
  const [data, setData] = useState<AccountDetailsResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchDetails = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    const result = await accountDetailsGateway.getAccountDetails(accountId);
    if (result.status === "ok") {
      setData(result.data);
    } else {
      logger.error("[useAccountDetails] fetch failed", result.error);
      setError(result.error);
    }
    setIsLoading(false);
  }, [accountId]);

  // ACD-037 — fetch on mount and on accountId change
  useEffect(() => {
    fetchDetails();
  }, [fetchDetails]);

  // ACD-039/040/MKT-036 — re-fetch on TransactionUpdated, AssetUpdated, or AssetPriceUpdated
  useEffect(() => {
    const unlistenPromise = accountDetailsGateway.subscribeToEvents((type) => {
      if (
        type === "TransactionUpdated" ||
        type === "AssetUpdated" ||
        type === "AssetPriceUpdated"
      ) {
        fetchDetails();
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [fetchDetails]);

  const holdingDetails = useMemo<HoldingDetail[]>(() => data?.holdings ?? [], [data]);

  const holdings = useMemo<HoldingRowViewModel[]>(
    () => (data ? data.holdings.map(toHoldingRow) : []),
    [data],
  );

  const closedHoldings = useMemo<ClosedHoldingRowViewModel[]>(
    () => (data ? data.closed_holdings.map(toClosedHoldingRow) : []),
    [data],
  );

  const summary = useMemo<AccountSummaryViewModel | null>(
    () => (data ? toAccountSummary(data) : null),
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
  };
}
