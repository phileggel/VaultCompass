import {
  type ArchiveAssetCommandError,
  type Asset,
  type AssetCommandError,
  type CreateAssetDTO,
  commands,
  type DeleteAssetCommandError,
  type Result,
  type UpdateAssetDTO,
} from "../../bindings";

/**
 * Gateway for Asset-related backend communication.
 * Centralizes all Tauri command calls for the Asset feature.
 */
export const assetGateway = {
  async getAssets(): Promise<Result<Asset[], AssetCommandError>> {
    return await commands.getAssets();
  },

  async getAssetsWithArchived(): Promise<Result<Asset[], AssetCommandError>> {
    return await commands.getAssetsWithArchived();
  },

  async createAsset(dto: CreateAssetDTO): Promise<Result<Asset, AssetCommandError>> {
    return await commands.addAsset(dto);
  },

  async updateAsset(dto: UpdateAssetDTO): Promise<Result<Asset, AssetCommandError>> {
    return await commands.updateAsset(dto);
  },

  async archiveAsset(id: string): Promise<Result<null, ArchiveAssetCommandError>> {
    return await commands.archiveAsset(id);
  },

  async unarchiveAsset(id: string): Promise<Result<null, AssetCommandError>> {
    return await commands.unarchiveAsset(id);
  },

  async deleteAsset(id: string): Promise<Result<null, DeleteAssetCommandError>> {
    return await commands.deleteAsset(id);
  },
};
