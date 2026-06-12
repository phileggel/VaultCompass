import type {
  AccountApplicationError,
  AccountDetailsResponse,
  AssetCrudError,
  AssetPrice,
  AssetPriceError,
  CorrectTransactionDTO,
  DepositDTO,
  DividendDTO,
  DividendError,
  FetchAccountAssetPricesError,
  HoldingTransactionError,
  OpenHoldingDTO,
  OpenHoldingError,
  Result,
  Transaction,
  WithdrawalDTO,
} from "@/bindings";
import { commands, events } from "@/bindings";

export const accountDetailsGateway = {
  async getAccountDetails(
    accountId: string,
  ): Promise<Result<AccountDetailsResponse, AccountApplicationError>> {
    return commands.getAccountDetails(accountId);
  },

  async recordAssetPrice(
    assetId: string,
    date: string,
    price: number,
  ): Promise<Result<null, AssetPriceError>> {
    return commands.recordAssetPrice(assetId, date, price);
  },

  async getAssetPrices(assetId: string): Promise<Result<AssetPrice[], AssetPriceError>> {
    return commands.getAssetPrices(assetId);
  },

  async updateAssetPrice(
    assetId: string,
    originalDate: string,
    newDate: string,
    newPrice: number,
  ): Promise<Result<null, AssetPriceError>> {
    return commands.updateAssetPrice(assetId, originalDate, newDate, newPrice);
  },

  async deleteAssetPrice(assetId: string, date: string): Promise<Result<null, AssetPriceError>> {
    return commands.deleteAssetPrice(assetId, date);
  },

  async openHolding(dto: OpenHoldingDTO): Promise<Result<Transaction, OpenHoldingError>> {
    return commands.openHolding(dto);
  },

  async recordDeposit(dto: DepositDTO): Promise<Result<Transaction, HoldingTransactionError>> {
    return commands.recordDeposit(dto);
  },

  async recordWithdrawal(
    dto: WithdrawalDTO,
  ): Promise<Result<Transaction, HoldingTransactionError>> {
    return commands.recordWithdrawal(dto);
  },

  async recordDividend(dto: DividendDTO): Promise<Result<Transaction, DividendError>> {
    return commands.recordDividend(dto);
  },

  // CSH-111 — editing a cash Deposit/Withdrawal persists via correct_transaction.
  async correctTransaction(
    id: string,
    accountId: string,
    dto: CorrectTransactionDTO,
  ): Promise<Result<Transaction, HoldingTransactionError>> {
    return commands.correctTransaction(id, accountId, dto);
  },

  async fetchAccountAssetPrices(
    accountId: string,
    useApiKey: boolean,
  ): Promise<Result<null, FetchAccountAssetPricesError>> {
    return commands.fetchAccountAssetPrices(accountId, useApiKey);
  },

  async blockAssetPriceRefresh(assetId: string): Promise<Result<null, AssetCrudError>> {
    return commands.blockAssetPriceRefresh(assetId);
  },

  async unblockAssetPriceRefresh(assetId: string): Promise<Result<null, AssetCrudError>> {
    return commands.unblockAssetPriceRefresh(assetId);
  },

  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
