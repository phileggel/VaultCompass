import {
  type Account,
  type AccountDeletionSummary,
  type AccountError,
  type AccountSummary,
  type CreateAccountDTO,
  commands,
  events,
  type FetchAllAssetPricesError,
  type Result,
  type UpdateAccountDTO,
} from "../../bindings";

/**
 * Gateway for Account-related backend communication.
 * Centralizes all Tauri command calls for the Account feature.
 */
export const accountGateway = {
  async getAccounts(): Promise<Result<Account[], AccountError>> {
    return await commands.getAccounts();
  },

  async getAccountSummaries(): Promise<Result<AccountSummary[], AccountError>> {
    return await commands.getAccountSummaries();
  },

  async addAccount(dto: CreateAccountDTO): Promise<Result<Account, AccountError>> {
    return await commands.addAccount(dto);
  },

  async updateAccount(dto: UpdateAccountDTO): Promise<Result<Account, AccountError>> {
    return await commands.updateAccount(dto);
  },

  async deleteAccount(id: string): Promise<Result<null, AccountError>> {
    return await commands.deleteAccount(id);
  },

  async getAccountDeletionSummary(
    accountId: string,
  ): Promise<Result<AccountDeletionSummary, AccountError>> {
    return await commands.getAccountDeletionSummary(accountId);
  },

  async fetchAllAssetPrices(): Promise<Result<null, FetchAllAssetPricesError>> {
    return commands.fetchAllAssetPrices();
  },

  async subscribeToEvents(callback: (type: string) => void): Promise<() => void> {
    return events.event.listen((event) => {
      callback(event.payload.type);
    });
  },
};
