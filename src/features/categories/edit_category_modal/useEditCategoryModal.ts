import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AssetCategory } from "@/bindings";
import { useCategories } from "../useCategories";

interface UseEditCategoryModalProps {
  category: AssetCategory | null;
  onClose: () => void;
}

export function useEditCategoryModal({ category, onClose }: UseEditCategoryModalProps) {
  const { t } = useTranslation();
  const { updateCategory } = useCategories();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (category) {
      setName(category.name);
      setError(null);
    }
  }, [category]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setName(e.target.value);
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!category || !name.trim()) return;

    setIsSubmitting(true);
    try {
      await updateCategory(category.id, name.trim());
      onClose();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("duplicate_name")) {
        setError(t("category.error_duplicate"));
      } else if (msg.includes("system_readonly")) {
        setError(t("category.error_system_readonly"));
      } else {
        setError(t("category.error_generic"));
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return { name, error, isSubmitting, handleChange, handleSubmit };
}
