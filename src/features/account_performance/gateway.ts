import type {
  AccountDetailsResponse,
  AccountError,
  AccountPerformanceResponse,
  Result,
} from "@/bindings";
import { commands, events } from "@/bindings";

export const accountPerformanceGateway = {
  async getAccountPerformance(
    accountId: string,
    assetId: string | null,
  ): Promise<Result<AccountPerformanceResponse, AccountError>> {
    return commands.getAccountPerformance(accountId, assetId);
  },

  /** PRF-082 — today's account holdings, the source of the asset-scope selector options. */
  async getAccountHoldings(
    accountId: string,
  ): Promise<Result<AccountDetailsResponse, AccountError>> {
    return commands.getAccountDetails(accountId, null);
  },

  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
