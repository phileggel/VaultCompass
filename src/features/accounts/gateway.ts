import {
  type Account,
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
  /**
   * Retrieves all accounts.
   */
  async getAccounts(): Promise<Result<Account[], string>> {
    return await commands.getAccounts();
  },

  /**
   * Adds a new account.
   */
  async addAccount(dto: CreateAccountDTO): Promise<Result<Account, string>> {
    return await commands.addAccount(dto);
  },

  /**
   * Updates an existing account.
   */
  async updateAccount(dto: UpdateAccountDTO): Promise<Result<Account, string>> {
    return await commands.updateAccount(dto);
  },

  /**
   * Deletes an account.
   */
  async deleteAccount(id: string): Promise<Result<null, string>> {
    return await commands.deleteAccount(id);
  },
};
