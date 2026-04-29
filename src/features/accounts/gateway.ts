import {
  type Account,
  type AccountCommandError,
  type AccountDeletionCommandError,
  type AccountDeletionSummary,
  type CreateAccountDTO,
  commands,
  type Result,
  type UpdateAccountDTO,
} from "../../bindings";

/**
 * Gateway for Account-related backend communication.
 * Centralizes all Tauri command calls for the Account feature.
 */
export const accountGateway = {
  async getAccounts(): Promise<Result<Account[], AccountCommandError>> {
    return await commands.getAccounts();
  },

  async addAccount(dto: CreateAccountDTO): Promise<Result<Account, AccountCommandError>> {
    return await commands.addAccount(dto);
  },

  async updateAccount(dto: UpdateAccountDTO): Promise<Result<Account, AccountCommandError>> {
    return await commands.updateAccount(dto);
  },

  async deleteAccount(id: string): Promise<Result<null, AccountCommandError>> {
    return await commands.deleteAccount(id);
  },

  async getAccountDeletionSummary(
    accountId: string,
  ): Promise<Result<AccountDeletionSummary, AccountDeletionCommandError>> {
    return await commands.getAccountDeletionSummary(accountId);
  },
};
