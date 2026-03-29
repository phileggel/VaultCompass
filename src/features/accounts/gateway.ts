import {
  type Account,
  type AssetAccount,
  type CreateAccountDTO,
  commands,
  type Result,
  type UpdateAccountDTO,
  type UpsertHoldingDTO,
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

  /**
   * Gets holdings for an account.
   */
  async getAccountHoldings(accountId: string): Promise<Result<AssetAccount[], string>> {
    return await commands.getAccountHoldings(accountId);
  },

  /**
   * Updates or creates an asset holding in an account.
   */
  async upsertAccountHolding(dto: UpsertHoldingDTO): Promise<Result<AssetAccount, string>> {
    return await commands.upsertAccountHolding(dto);
  },

  /**
   * Removes an asset holding from an account.
   */
  async removeAccountHolding(accountId: string, assetId: string): Promise<Result<null, string>> {
    return await commands.removeAccountHolding(accountId, assetId);
  },
};
