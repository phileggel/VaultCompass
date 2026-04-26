import { type AssetCategory, type CategoryCommandError, commands, type Result } from "@/bindings";

export const categoryGateway = {
  async getCategories(): Promise<Result<AssetCategory[], CategoryCommandError>> {
    return commands.getCategories();
  },

  async addCategory(label: string): Promise<Result<AssetCategory, CategoryCommandError>> {
    return commands.addCategory(label);
  },

  async updateCategory(
    id: string,
    label: string,
  ): Promise<Result<AssetCategory, CategoryCommandError>> {
    return commands.updateCategory(id, label);
  },

  async deleteCategory(id: string): Promise<Result<null, CategoryCommandError>> {
    return commands.deleteCategory(id);
  },
};
