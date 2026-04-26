import { useCallback, useEffect, useMemo, useState } from "react";
import type { AccountDetailsResponse } from "@/bindings";
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

  // ACD-039/040 — re-fetch on TransactionUpdated or AssetUpdated via gateway
  useEffect(() => {
    const unlistenPromise = accountDetailsGateway.subscribeToEvents((type) => {
      if (type === "TransactionUpdated" || type === "AssetUpdated") {
        fetchDetails();
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [fetchDetails]);

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

  return { isLoading, error, retry: fetchDetails, holdings, closedHoldings, summary };
}
