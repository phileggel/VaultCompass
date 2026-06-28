import type {
  AccountDetailsResponse,
  AccountError,
  AssetCrudError,
  AssetPrice,
  AssetPriceError,
  DepositDTO,
  DividendDTO,
  DividendError,
  FetchAccountAssetPricesError,
  FetchAccountAssetPricesForDateError,
  FetchForDateOutcome,
  FreeSharesDTO,
  FreeSharesError,
  HoldingSnapshot,
  OpenHoldingDTO,
  OpenHoldingError,
  Result,
  Transaction,
  WithdrawalDTO,
} from "@/bindings";
import { commands, events } from "@/bindings";
import type { CorrectTransactionFields } from "@/features/transactions/shared/types";
import { useAppStore } from "@/lib/store";

export const accountDetailsGateway = {
  async getAccountDetails(
    accountId: string,
  ): Promise<Result<AccountDetailsResponse, AccountError>> {
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

  async recordDeposit(dto: DepositDTO): Promise<Result<Transaction, AccountError>> {
    return commands.recordDeposit(dto);
  },

  async recordWithdrawal(dto: WithdrawalDTO): Promise<Result<Transaction, AccountError>> {
    return commands.recordWithdrawal(dto);
  },

  async recordDividend(dto: DividendDTO): Promise<Result<Transaction, DividendError>> {
    return commands.recordDividend(dto);
  },

  // FSD-022 — record a zero-cost free-share distribution attributed to a held asset.
  async recordFreeShares(dto: FreeSharesDTO): Promise<Result<Transaction, FreeSharesError>> {
    return commands.recordFreeShares(dto);
  },

  // CSH-111 — editing a cash Deposit/Withdrawal persists via correct_transaction.
  async correctTransaction(
    id: string,
    accountId: string,
    dto: CorrectTransactionFields,
  ): Promise<Result<Transaction, AccountError>> {
    return commands.correctTransaction({ ...dto, account_id: accountId, transaction_id: id });
  },

  // TDI-010 — holding quantity + VWAP average cost as of a date (trade-dialog insights).
  async getHoldingSnapshotAsOf(
    accountId: string,
    assetId: string,
    date: string,
  ): Promise<Result<HoldingSnapshot, AccountError>> {
    return commands.getHoldingSnapshotAsOf(accountId, assetId, date);
  },

  async fetchAccountAssetPrices(
    accountId: string,
  ): Promise<Result<null, FetchAccountAssetPricesError>> {
    return commands.fetchAccountAssetPrices(accountId);
  },

  async fetchAccountAssetPricesForDate(
    accountId: string,
    date: string,
  ): Promise<Result<FetchForDateOutcome, FetchAccountAssetPricesForDateError>> {
    return commands.fetchAccountAssetPricesForDate(accountId, date);
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

/**
 * Asset catalog from the shared BE/FE cache (F28). Feature code reads the cache
 * through its own gateway rather than importing the store directly.
 */
export function useCachedAssets() {
  return useAppStore((state) => state.assets);
}

/** Account list from the shared BE/FE cache (F28). */
export function useCachedAccounts() {
  return useAppStore((state) => state.accounts);
}
