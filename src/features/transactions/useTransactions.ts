import { useCallback } from "react";
import type { CreateTransactionDTO, Transaction } from "@/bindings";
import { logger } from "@/lib/logger";
import { transactionGateway } from "./gateway";

/**
 * Hook providing CRUD callbacks for the transaction feature.
 * Does not hold domain state — holdings are managed via the transaction store.
 */
export function useTransactions() {
  const addTransaction = useCallback(async (dto: CreateTransactionDTO) => {
    try {
      const res = await transactionGateway.addTransaction(dto);
      if (res.status === "ok") {
        return { data: res.data, error: null };
      }
      return { data: null, error: res.error };
    } catch (e) {
      logger.error("Failed to add transaction", { error: e });
      return { data: null, error: String(e) };
    }
  }, []);

  const updateTransaction = useCallback(async (id: string, dto: CreateTransactionDTO) => {
    try {
      const res = await transactionGateway.updateTransaction(id, dto);
      if (res.status === "ok") {
        return { data: res.data, error: null };
      }
      return { data: null, error: res.error };
    } catch (e) {
      logger.error("Failed to update transaction", { error: e });
      return { data: null, error: String(e) };
    }
  }, []);

  const deleteTransaction = useCallback(async (id: string) => {
    try {
      const res = await transactionGateway.deleteTransaction(id);
      if (res.status === "ok") {
        return { error: null };
      }
      return { error: res.error };
    } catch (e) {
      logger.error("Failed to delete transaction", { error: e });
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
    addTransaction,
    updateTransaction,
    deleteTransaction,
    getTransactions,
  };
}
