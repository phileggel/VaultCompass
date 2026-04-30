import type {
  AccountDetailsCommandError,
  AccountDetailsResponse,
  AssetPrice,
  AssetPriceCommandError,
  DeleteAssetPriceCommandError,
  Result,
  UpdateAssetPriceCommandError,
} from "@/bindings";
import { commands, events } from "@/bindings";

export const accountDetailsGateway = {
  async getAccountDetails(
    accountId: string,
  ): Promise<Result<AccountDetailsResponse, AccountDetailsCommandError>> {
    return commands.getAccountDetails(accountId);
  },

  async recordAssetPrice(
    assetId: string,
    date: string,
    price: number,
  ): Promise<Result<null, AssetPriceCommandError>> {
    return commands.recordAssetPrice(assetId, date, price);
  },

  async getAssetPrices(assetId: string): Promise<Result<AssetPrice[], AssetPriceCommandError>> {
    return commands.getAssetPrices(assetId);
  },

  async updateAssetPrice(
    assetId: string,
    originalDate: string,
    newDate: string,
    newPrice: number,
  ): Promise<Result<null, UpdateAssetPriceCommandError>> {
    return commands.updateAssetPrice(assetId, originalDate, newDate, newPrice);
  },

  async deleteAssetPrice(
    assetId: string,
    date: string,
  ): Promise<Result<null, DeleteAssetPriceCommandError>> {
    return commands.deleteAssetPrice(assetId, date);
  },

  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
