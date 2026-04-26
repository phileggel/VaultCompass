import { useCallback } from "react";
import { useAppStore } from "../../lib/store";
import { categoryGateway } from "./gateway";

export function useCategories() {
  const categories = useAppStore((state) => state.categories);
  const loading = useAppStore((state) => state.isLoadingCategories);
  const error = useAppStore((state) => state.categoriesError);
  const fetchCategories = useAppStore((state) => state.fetchCategories);

  const addCategory = useCallback(async (label: string): Promise<{ error?: string }> => {
    const result = await categoryGateway.addCategory(label);
    if (result.status === "error") return { error: `error.${result.error.code}` };
    return {};
  }, []);

  const updateCategory = useCallback(
    async (id: string, label: string): Promise<{ error?: string }> => {
      const result = await categoryGateway.updateCategory(id, label);
      if (result.status === "error") return { error: `error.${result.error.code}` };
      return {};
    },
    [],
  );

  const deleteCategory = useCallback(async (id: string): Promise<{ error?: string }> => {
    const result = await categoryGateway.deleteCategory(id);
    if (result.status === "error") return { error: `error.${result.error.code}` };
    return {};
  }, []);

  return {
    categories,
    loading,
    error,
    fetchCategories,
    addCategory,
    updateCategory,
    deleteCategory,
  };
}
