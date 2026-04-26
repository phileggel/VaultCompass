import type { AccountDetailsResponse, Result } from "@/bindings";
import { commands, events } from "@/bindings";

export const accountDetailsGateway = {
  async getAccountDetails(accountId: string): Promise<Result<AccountDetailsResponse, string>> {
    return commands.getAccountDetails(accountId);
  },

  /** Records (or overwrites) a market price for an asset on a given date (MKT-025). */
  async recordAssetPrice(
    assetId: string,
    date: string,
    price: number,
  ): Promise<Result<null, string>> {
    return commands.recordAssetPrice(assetId, date, price);
  },

  /** Subscribes to backend events relevant to account details (ACD-039, ACD-040, MKT-036). */
  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
