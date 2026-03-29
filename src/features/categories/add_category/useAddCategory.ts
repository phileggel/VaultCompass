import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useCategories } from "../useCategories";

interface UseAddCategoryProps {
  onSubmitSuccess?: () => void;
}

export function useAddCategory({ onSubmitSuccess }: UseAddCategoryProps = {}) {
  const { t } = useTranslation();
  const { addCategory } = useCategories();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setName(e.target.value);
    setError(null);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setIsSubmitting(true);
    try {
      await addCategory(name.trim());
      setName("");
      setError(null);
      onSubmitSuccess?.();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("duplicate_name")) {
        setError(t("category.error_duplicate"));
      } else {
        setError(t("category.error_generic"));
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return { name, error, isSubmitting, handleChange, handleSubmit };
}
