import { useCallback, useEffect, useState } from "react";
import type { AssetPrice } from "@/bindings";
import { logger } from "@/lib/logger";
import { accountDetailsGateway } from "../gateway";

interface UsePriceHistoryProps {
  assetId: string;
}

export interface UsePriceHistoryResult {
  prices: AssetPrice[];
  isLoading: boolean;
  fetchError: string | null;
  deleteError: string | null;
  deletingDate: string | null;
  refetch: () => void;
  /** Returns true on success, false on failure. */
  confirmDelete: (date: string) => Promise<boolean>;
}

export function usePriceHistory({ assetId }: UsePriceHistoryProps): UsePriceHistoryResult {
  const [prices, setPrices] = useState<AssetPrice[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [deletingDate, setDeletingDate] = useState<string | null>(null);

  const loadPrices = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await accountDetailsGateway.getAssetPrices(assetId);
      if (result.status === "ok") {
        setPrices(result.data);
        setFetchError(null);
      } else {
        logger.error("[usePriceHistory] getAssetPrices failed", result.error);
        setFetchError(result.error.code);
      }
    } catch (err) {
      logger.error("[usePriceHistory] getAssetPrices threw", err);
      setFetchError("Unknown");
    } finally {
      setIsLoading(false);
    }
  }, [assetId]);

  useEffect(() => {
    loadPrices();
  }, [loadPrices]);

  const confirmDelete = useCallback(
    async (date: string): Promise<boolean> => {
      setDeletingDate(date);
      const result = await accountDetailsGateway.deleteAssetPrice(assetId, date);
      setDeletingDate(null);
      if (result.status === "ok") {
        setDeleteError(null);
        loadPrices();
        return true;
      }
      logger.error("[usePriceHistory] deleteAssetPrice failed", result.error);
      setDeleteError(result.error.code);
      return false;
    },
    [assetId, loadPrices],
  );

  return {
    prices,
    isLoading,
    fetchError,
    deleteError,
    deletingDate,
    refetch: loadPrices,
    confirmDelete,
  };
}
