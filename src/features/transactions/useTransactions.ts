import { useCallback } from "react";
import type { BuyHoldingDTO, CorrectTransactionDTO, SellHoldingDTO, Transaction } from "@/bindings";
import { logger } from "@/lib/logger";
import { transactionGateway } from "./gateway";

/**
 * Hook providing CRUD callbacks for the transaction feature.
 * Does not hold domain state — holdings are managed via the transaction store.
 */
export function useTransactions() {
  const buyHolding = useCallback(async (dto: BuyHoldingDTO) => {
    try {
      const res = await transactionGateway.buyHolding(dto);
      if (res.status === "ok") {
        return { data: res.data, error: null };
      }
      return { data: null, error: `error.${res.error.code}` };
    } catch (e) {
      logger.error("Failed to buy holding", { error: e });
      return { data: null, error: String(e) };
    }
  }, []);

  const sellHolding = useCallback(async (dto: SellHoldingDTO) => {
    try {
      const res = await transactionGateway.sellHolding(dto);
      if (res.status === "ok") {
        return { data: res.data, error: null };
      }
      return { data: null, error: `error.${res.error.code}` };
    } catch (e) {
      logger.error("Failed to sell holding", { error: e });
      return { data: null, error: String(e) };
    }
  }, []);

  const correctTransaction = useCallback(
    async (id: string, accountId: string, dto: CorrectTransactionDTO) => {
      try {
        const res = await transactionGateway.correctTransaction(id, accountId, dto);
        if (res.status === "ok") {
          return { data: res.data, error: null };
        }
        return { data: null, error: `error.${res.error.code}` };
      } catch (e) {
        logger.error("Failed to correct transaction", { error: e });
        return { data: null, error: String(e) };
      }
    },
    [],
  );

  const cancelTransaction = useCallback(async (id: string, accountId: string) => {
    try {
      const res = await transactionGateway.cancelTransaction(id, accountId);
      if (res.status === "ok") {
        return { error: null };
      }
      return { error: `error.${res.error.code}` };
    } catch (e) {
      logger.error("Failed to cancel transaction", { error: e });
      return { error: String(e) };
    }
  }, []);

  const getTransactions = useCallback(
    async (accountId: string, assetId: string): Promise<Transaction[]> => {
      try {
        const res = await transactionGateway.getTransactions(accountId, assetId);
        if (res.status === "ok") {
          return res.data;
        }
        logger.error("Failed to get transactions", { error: res.error });
        return [];
      } catch (e) {
        logger.error("Failed to get transactions", { error: e });
        return [];
      }
    },
    [],
  );

  return {
    buyHolding,
    sellHolding,
    correctTransaction,
    cancelTransaction,
    getTransactions,
  };
}
