import { commands } from "@/bindings";

export const categoryGateway = {
  async getCategories() {
    const res = await commands.getCategories();
    if (res.status === "error") throw new Error(res.error);
    return res.data;
  },

  async addCategory(label: string) {
    const res = await commands.addCategory(label);
    if (res.status === "error") throw new Error(res.error);
    return res.data;
  },

  async updateCategory(id: string, label: string) {
    const res = await commands.updateCategory(id, label);
    if (res.status === "error") throw new Error(res.error);
    return res.data;
  },

  async deleteCategory(id: string) {
    const res = await commands.deleteCategory(id);
    if (res.status === "error") throw new Error(res.error);
    return res.data;
  },
};
