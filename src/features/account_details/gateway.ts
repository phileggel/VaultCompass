import type { AccountDetailsResponse, Result } from "@/bindings";
import { commands, events } from "@/bindings";

export const accountDetailsGateway = {
  async getAccountDetails(accountId: string): Promise<Result<AccountDetailsResponse, string>> {
    return commands.getAccountDetails(accountId);
  },

  /** Subscribes to backend events relevant to account details (ACD-039, ACD-040). */
  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
