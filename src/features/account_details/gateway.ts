import type {
  AccountDetailsCommandError,
  AccountDetailsResponse,
  AssetPriceCommandError,
  Result,
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

  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
