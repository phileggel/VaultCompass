import { useCallback } from "react";
import { useAppStore } from "../../lib/store";
import { categoryGateway } from "./gateway";

export function useCategories() {
  const categories = useAppStore((state) => state.categories);
  const loading = useAppStore((state) => state.isLoadingCategories);
  const error = useAppStore((state) => state.categoriesError);
  const fetchCategories = useAppStore((state) => state.fetchCategories);

  const addCategory = useCallback(async (label: string) => {
    await categoryGateway.addCategory(label);
  }, []);

  const updateCategory = useCallback(async (id: string, label: string) => {
    await categoryGateway.updateCategory(id, label);
  }, []);

  const deleteCategory = useCallback(async (id: string) => {
    await categoryGateway.deleteCategory(id);
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
