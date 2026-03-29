import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "@/lib/store";
import { FAB } from "@/ui/components/fab/FAB";
import { ManagerLayout } from "@/ui/components/layout/ManagerLayout";
import { AddCategoryModal } from "./add_category/AddCategory";
import { CategoryTable } from "./category_table/CategoryTable";

export function CategoryManager() {
  const { t } = useTranslation();
  const count = useAppStore((state) => state.categories.length);
  const [query, setQuery] = useState("");
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);

  return (
    <>
      <ManagerLayout
        searchId="category-search"
        title={t("category.title")}
        count={count}
        searchTerm={query}
        onSearchChange={setQuery}
        searchPlaceholder={t("category.search_placeholder")}
        table={<CategoryTable searchTerm={query} />}
      />
      <FAB onClick={() => setIsAddModalOpen(true)} label={t("category.fab_label")} />
      <AddCategoryModal isOpen={isAddModalOpen} onClose={() => setIsAddModalOpen(false)} />
    </>
  );
}
