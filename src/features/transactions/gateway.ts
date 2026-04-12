import type { Result } from "../../bindings";
import { type CreateTransactionDTO, commands, type Transaction } from "../../bindings";

/**
 * Gateway for Transaction-related backend communication.
 * Centralizes all Tauri command calls for the Transaction feature.
 */
export const transactionGateway = {
  /**
   * Creates a new purchase transaction and updates the Holding atomically (TRX-027).
   */
  async addTransaction(dto: CreateTransactionDTO): Promise<Result<Transaction, string>> {
    return await commands.addTransaction(dto);
  },

  /**
   * Updates an existing transaction and recalculates the affected Holding(s) (TRX-031).
   */
  async updateTransaction(
    id: string,
    dto: CreateTransactionDTO,
  ): Promise<Result<Transaction, string>> {
    return await commands.updateTransaction(id, dto);
  },

  /**
   * Deletes a transaction and recalculates (or removes) the associated Holding (TRX-034).
   */
  async deleteTransaction(id: string): Promise<Result<null, string>> {
    return await commands.deleteTransaction(id);
  },

  /**
   * Retrieves all transactions for an account/asset pair.
   */
  async getTransactions(
    accountId: string,
    assetId: string,
  ): Promise<Result<Transaction[], string>> {
    return await commands.getTransactions(accountId, assetId);
  },
};
