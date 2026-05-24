import { useCallback, useEffect, useState } from "react";
import type { AccountSummary } from "@/bindings";
import { logger } from "@/lib/logger";
import type { I18nMessage } from "@/ui/format/i18n";
import { accountGateway } from "./gateway";
import { accountMutationErrorToI18n } from "./shared/presenter";

const UNKNOWN_ERROR: I18nMessage = { key: "error.Unknown" };

interface UseAccountSummariesResult {
  summaries: AccountSummary[];
  isLoading: boolean;
  error: I18nMessage | null;
  refetch: () => Promise<void>;
}

/**
 * Fetches the per-account global-value view (ACC-021). Listens to backend
 * events so the Accounts list refreshes when account state or prices change.
 */
export function useAccountSummaries(): UseAccountSummariesResult {
  const [summaries, setSummaries] = useState<AccountSummary[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<I18nMessage | null>(null);

  const fetchSummaries = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await accountGateway.getAccountSummaries();
      if (result.status === "ok") {
        setSummaries(result.data);
      } else {
        logger.error("[useAccountSummaries] fetch failed", { error: result.error });
        setError(accountMutationErrorToI18n(result.error));
      }
    } catch (err) {
      logger.error("[useAccountSummaries] fetch threw", { error: err });
      setError(UNKNOWN_ERROR);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSummaries();
  }, [fetchSummaries]);

  // Re-fetch on events that can change account values: AccountUpdated covers
  // CRUD + transaction-driven holding changes (TRX-037); AssetPriceUpdated
  // covers value drift from manual price entries + auto-fetch (MKT-036);
  // AssetUpdated covers asset currency changes that flip the same-currency filter.
  useEffect(() => {
    const unlistenPromise = accountGateway.subscribeToEvents((type) => {
      if (
        type === "AccountUpdated" ||
        type === "AssetUpdated" ||
        type === "AssetPriceUpdated" ||
        type === "TransactionUpdated"
      ) {
        fetchSummaries();
      }
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [fetchSummaries]);

  return { summaries, isLoading, error, refetch: fetchSummaries };
}
