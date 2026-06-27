import type { AccountError, AccountPerformanceResponse, Result } from "@/bindings";
import { commands, events } from "@/bindings";

export const accountPerformanceGateway = {
  async getAccountPerformance(
    accountId: string,
  ): Promise<Result<AccountPerformanceResponse, AccountError>> {
    return commands.getAccountPerformance(accountId);
  },

  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
