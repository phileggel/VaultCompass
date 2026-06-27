import type {
  AccountError,
  AssetPriceError,
  BuyHoldingDTO,
  CorrectTransactionDTO,
  Event,
  SellHoldingDTO,
  Transaction,
} from "../../bindings";
import { commands, events, type Result } from "../../bindings";

/**
 * Gateway for Transaction-related backend communication.
 * Centralizes all Tauri command calls for the Transaction feature.
 */
export const transactionGateway = {
  async buyHolding(dto: BuyHoldingDTO): Promise<Result<Transaction, AccountError>> {
    return await commands.buyHolding(dto);
  },

  async sellHolding(dto: SellHoldingDTO): Promise<Result<Transaction, AccountError>> {
    return await commands.sellHolding(dto);
  },

  async correctTransaction(
    id: string,
    accountId: string,
    dto: CorrectTransactionDTO,
  ): Promise<Result<Transaction, AccountError>> {
    return await commands.correctTransaction(id, accountId, dto);
  },

  async cancelTransaction(id: string, accountId: string): Promise<Result<null, AccountError>> {
    return await commands.cancelTransaction(id, accountId);
  },

  async getTransactions(
    accountId: string,
    assetId: string,
  ): Promise<Result<Transaction[], AccountError>> {
    return await commands.getTransactions(accountId, assetId);
  },

  async getAllTransactionsForAccount(
    accountId: string,
  ): Promise<Result<Transaction[], AccountError>> {
    return await commands.getAllTransactionsForAccount(accountId);
  },

  async getAssetIdsForAccount(accountId: string): Promise<Result<string[], AccountError>> {
    return await commands.getAssetIdsForAccount(accountId);
  },

  async recordAssetPrice(
    assetId: string,
    date: string,
    price: number,
  ): Promise<Result<null, AssetPriceError>> {
    return await commands.recordAssetPrice(assetId, date, price);
  },

  /** Subscribe to the backend event bus; invokes `callback` with each event's discriminant. */
  async subscribeToEvents(callback: (type: Event["type"]) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
