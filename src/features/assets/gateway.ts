import {
  type Asset,
  type CreateAssetDTO,
  commands,
  type Result,
  type UpdateAssetDTO,
} from "../../bindings";

/**
 * Gateway for Asset-related backend communication.
 * Centralizes all Tauri command calls for the Asset feature.
 */
export const assetGateway = {
  /**
   * Fetches all active (non-archived) assets.
   */
  async getAssets(): Promise<Result<Asset[], string>> {
    return await commands.getAssets();
  },

  /**
   * Fetches all assets including archived ones.
   */
  async getAssetsWithArchived(): Promise<Result<Asset[], string>> {
    return await commands.getAssetsWithArchived();
  },

  /**
   * Creates a new asset.
   */
  async createAsset(dto: CreateAssetDTO): Promise<Result<Asset, string>> {
    return await commands.addAsset(dto);
  },

  /**
   * Updates an existing asset.
   */
  async updateAsset(dto: UpdateAssetDTO): Promise<Result<Asset, string>> {
    return await commands.updateAsset(dto);
  },

  /**
   * Archives an asset (reversible — R6).
   */
  async archiveAsset(id: string): Promise<Result<null, string>> {
    return await commands.archiveAsset(id);
  },

  /**
   * Unarchives an asset (R18).
   */
  async unarchiveAsset(id: string): Promise<Result<null, string>> {
    return await commands.unarchiveAsset(id);
  },

  /**
   * Deletes an asset by ID.
   */
  async deleteAsset(id: string): Promise<Result<null, string>> {
    return await commands.deleteAsset(id);
  },
};
