import type { Result, TransactionCommandError } from "../../bindings";
import { type CreateTransactionDTO, commands, type Transaction } from "../../bindings";

/**
 * Gateway for Transaction-related backend communication.
 * Centralizes all Tauri command calls for the Transaction feature.
 */
export const transactionGateway = {
  async addTransaction(
    dto: CreateTransactionDTO,
  ): Promise<Result<Transaction, TransactionCommandError>> {
    return await commands.addTransaction(dto);
  },

  async updateTransaction(
    id: string,
    dto: CreateTransactionDTO,
  ): Promise<Result<Transaction, TransactionCommandError>> {
    return await commands.updateTransaction(id, dto);
  },

  async deleteTransaction(id: string): Promise<Result<null, TransactionCommandError>> {
    return await commands.deleteTransaction(id);
  },

  async getTransactions(
    accountId: string,
    assetId: string,
  ): Promise<Result<Transaction[], TransactionCommandError>> {
    return await commands.getTransactions(accountId, assetId);
  },

  async getAssetIdsForAccount(accountId: string): Promise<Result<string[], string>> {
    return await commands.getAssetIdsForAccount(accountId);
  },
};
