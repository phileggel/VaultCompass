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
   * Fetches all non-deleted assets.
   */
  async getAssets(): Promise<Result<Asset[], string>> {
    return await commands.getAssets();
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
   * Deletes an asset by ID.
   */
  async deleteAsset(id: string): Promise<Result<null, string>> {
    return await commands.deleteAsset(id);
  },
};
